use anyhow::Result;
use qapi::qga;
use clap::Parser;
use tokio::time::{Duration, timeout};
use super::{GlobalArgs, QgaStream};

#[derive(Parser, Debug)]
/// Displays information about the guest, and can be used to check that the guest agent is running
pub(crate) struct Info {
	#[clap(short = 'O', long = "os")]
	os_info: bool,
}

impl Info {
	pub async fn run(self, qga: QgaStream, _args: GlobalArgs) -> Result<i32> {
		let info = qga.execute(qga::guest_info { }).await?;
		println!("Guest Info: {:#?}", info);

		if self.os_info {
			let osinfo = qga.execute(qga::guest_get_osinfo { }).await?;
			println!("Guest OS Info: {:#?}", osinfo);
		}

		Ok(0)
	}
}

#[derive(Parser, Debug)]
/// Displays information about the guest, and can be used to check that the guest agent is running
pub(crate) struct Ping {
	#[clap(short, long)]
	repeat: bool,
	#[clap(short, long = "timeout")]
	timeout_seconds: Option<u64>,
}

impl Ping {
	fn timeout(&self) -> Option<Duration> {
		self.timeout_seconds.map(Duration::from_secs)
	}

	pub async fn run(self, qga: QgaStream, args: GlobalArgs) -> Result<i32> {
		let duration = self.timeout();
		if self.repeat {
			qemucomm::wait(duration, async move {
				let mut sync_value = 0;
				loop {
					match timeout(Duration::from_secs(1), self.ping(&qga, &args)).await {
						Err(_) => {
							let _ = timeout(Duration::from_secs(1), qga.guest_sync(sync_value)).await;
							sync_value = sync_value.wrapping_add(1);
						},
						Ok(Err(e)) => break Err(e),
						Ok(Ok(())) => break Ok(()),
					}
				}
			}).await?;
		} else {
			qemucomm::wait(duration, self.ping(&qga, &args)).await?;
		}
		Ok(0)
	}

	async fn ping(&self, qga: &QgaStream, _args: &GlobalArgs) -> Result<()> {
		qga.execute(qga::guest_ping { }).await?;
		Ok(())
	}
}
