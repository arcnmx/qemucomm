use anyhow::Result;
use qapi::qga;
use clap::Parser;
use futures::future::{FutureExt, Fuse};
use futures::stream::StreamExt;
use futures::{pin_mut, select};
use tokio::time::sleep;
use std::time::Duration;
use std::path::PathBuf;
use std::fs::File;
use std::io::{self, Read};
use super::{GlobalArgs, QgaStream};

#[derive(Parser, Debug)]
/// Executes a process inside the guest
pub(crate) struct Exec {
	/// set environment variable(s)
	#[clap(short = 'e', long = "env")]
	environment: Option<Vec<String>>,
	/// send contents as stdin
	#[clap(short = 'i', long)]
	stdin: Option<PathBuf>,
	/// do not wait for process to terminate
	#[clap(short = 'W', long)]
	no_wait: bool,
	path: String,
	arguments: Vec<String>,
}

fn guest_kill(pid: i64, os: qga::GuestOSInfo, force: bool) -> qga::guest_exec {
	let os_id = os.id.as_ref().map(String::as_str).unwrap_or("");
	match os_id {
		"mswindows" => qga::guest_exec {
			path: "powershell.exe".into(),
			arg: Some(if force {
				vec!["-Command".into(), format!("Stop-Process -Id {pid} -Force")]
			} else {
				vec!["-Command".into(), format!("Stop-Process -Id {pid}")]
			}),
			env: Default::default(),
			input_data: Default::default(),
			capture_output: Some(false),
		},
		_ => qga::guest_exec {
			path: "kill".into(),
			arg: Some(if force {
				vec!["-TERM".into(), format!("{pid}")]
			} else {
				vec![format!("{pid}")]
			}),
			env: Default::default(),
			input_data: Default::default(),
			capture_output: Some(false),
		},
	}
}

const SIGINT: i32 = 2;

impl Exec {
	pub async fn run(self, qga: QgaStream, _args: GlobalArgs) -> Result<i32> {
		let guest_exec = qga::guest_exec {
			path: self.path,
			arg: Some(self.arguments),
			env: self.environment,
			input_data: {
				let mut bytes = Vec::new();
				match self.stdin.as_ref().map(|p| (p.to_str(), p)) {
					Some((Some("-"), _)) => io::stdin().read_to_end(&mut bytes).map(Some),
					Some((_, p)) => File::open(p).and_then(|mut f| f.read_to_end(&mut bytes)).map(Some),
					None => Ok(None),
				}?.map(|_| bytes)
			},
			capture_output: Some(!self.no_wait),
		};
		log::trace!("QEMU GA Exec {:#?}", guest_exec);

		let qga::GuestExec { pid } = qga.execute(guest_exec).await?;
		log::trace!("QEMU GA PID {:?}", pid);

		if self.no_wait {
			return Ok(0)
		}

		let ctrlc = StreamExt::fuse(async_ctrlc::CtrlC::new().expect("ctrl+c"));
		let mut ctrlc_counter = 0u8;
		pin_mut!(ctrlc);
		let timeout = sleep(Default::default()).fuse();
		pin_mut!(timeout);
		let status = Fuse::terminated();
		pin_mut!(status);

		let status = loop {
			select! {
				_ = ctrlc.next() => {
					ctrlc_counter = ctrlc_counter.saturating_add(1);
					match ctrlc_counter {
						1 => {
							let os_info = qga.execute(qga::guest_get_osinfo { }).await?;
							qga.execute(guest_kill(pid, os_info, false)).await?;
							timeout.set(sleep(Duration::from_millis(1)).fuse());
						},
						2 => {
							let os_info = qga.execute(qga::guest_get_osinfo { }).await?;
							qga.execute(guest_kill(pid, os_info, true)).await?;
							timeout.set(sleep(Duration::from_millis(1)).fuse());
						},
						_ => {
							return Ok(32)
						},
					}
				},
				_ = timeout => {
					status.set(qga.execute(qga::guest_exec_status { pid }).fuse());
				},
				status = status => {
					let status = status?;
					timeout.set(sleep(Duration::from_millis(5)).fuse());

					if let Some(err) = &status.err_data {
						io::copy(&mut &err[..], &mut io::stderr())?;
					}
					if let Some(out) = &status.out_data {
						io::copy(&mut &out[..], &mut io::stdout())?;
					}

					if status.exited {
						break status
					}
				},
			}
		};
		log::trace!("QEMU GA ExecStatus {:?}", status);

		if matches!(status.err_truncated, Some(true)) {
			log::warn!("STDERR truncated")
		}
		if matches!(status.out_truncated, Some(true)) {
			log::warn!("STDOUT truncated")
		}

		match status.exited {
			false => Ok(1),
			true => match (status.exitcode, status.signal) {
				(Some(code), _) => Ok(code as i32),
				_ if ctrlc_counter > 0 => Ok(128 + SIGINT),
				(None, Some(signal)) if signal > 0 && signal < 128 => Ok(128 + signal as i32),
				(None, Some(_)) => Ok(125),
				_ => Ok(0),
			},
		}
	}
}
