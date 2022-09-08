use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use futures::StreamExt;
use tokio::time::timeout;
use tokio::sync::broadcast;
use std::time::Duration;
use std::path::PathBuf;

mod command;
mod status;
mod device;
mod object;
mod hmp;

pub(crate) type QmpStreamWrite = qapi::futures::QmpStreamTokio<tokio::io::WriteHalf<tokio::net::UnixStream>>;
pub(crate) type QmpStreamRead = qapi::futures::QmpStreamTokio<tokio::io::ReadHalf<tokio::net::UnixStream>>;
pub(crate) type QmpStream = qapi::futures::QapiService<QmpStreamWrite>;

#[derive(Args, Debug)]
pub(crate) struct GlobalArgs {
}

#[derive(Args, Debug)]
pub(crate) struct ConnectionArgs {
	/// QEMU QMP socket path
	#[clap(short, long, env("QEMUCOMM_QMP_SOCKET_PATH"))]
	socket: PathBuf,
	#[clap(short, long)]
	wait: bool,
	#[clap(short, long = "timeout")]
	timeout_seconds: Option<u64>,
}

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Cli {
	#[clap(flatten)]
	connection: ConnectionArgs,
	#[clap(flatten)]
	args: GlobalArgs,
	#[clap(subcommand)]
	command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
	Ping,
	Status(status::Status),
	#[clap(alias = "hmp")]
	HumanCommand(hmp::HumanCommand),
	AddDevice(device::AddDevice),
	DelDevice(device::DelDevice),
	AddObject(object::AddObject),
	DelObject(object::DelObject),
	Stop(command::StopCommand),
	#[clap(alias = "cont")]
	Continue(command::ContinueCommand),
	Quit(command::QuitCommand),
}

#[tokio::main]
async fn main() -> Result<()> {
	::env_logger::init();

	let args = Cli::parse();

	let (stream, _caps) = args.connection.connect().await?;
	let (qmp, mut stream) = stream.into_parts();
	let (event_send, events) = broadcast::channel(8);

	let _ = stream.release();
	let handle = tokio::spawn(async move {
		while let Some(event) = stream.next().await {
			match event {
				Ok(e) => match event_send.send(e) {
					Err(e) =>
						log::info!("event ignored: {:?}", e.0),
					Ok(..) => (),
				},
				Err(e) => {
					log::warn!("stream error: {:?}", e);
					break
				},
			}
		}
	});

	let res = match args.command {
		Command::Ping => {
			drop(qmp);
			Ok(0)
		},
		Command::Status(c) => c.run(qmp, args.args).await,
		Command::HumanCommand(c) => c.run(qmp, args.args).await,
		Command::AddDevice(c) => c.run(qmp, events, args.args).await,
		Command::DelDevice(c) => c.run(qmp, events, args.args).await,
		Command::AddObject(c) => c.run(qmp, args.args).await,
		Command::DelObject(c) => c.run(qmp, args.args).await,
		Command::Stop(c) => c.run(qmp, args.args).await,
		Command::Continue(c) => c.run(qmp, args.args).await,
		Command::Quit(c) => c.run(qmp, args.args).await,
	};

	match timeout(Duration::from_secs(1), handle).await {
		Err(_elapsed) => log::warn!("timed out waiting for handle to clean up"),
		Ok(res) => res?,
	}

	match res {
		Err(e) => Err(e),
		Ok(0) => Ok(()),
		Ok(code) => {
			std::process::exit(code)
		},
	}
}

impl ConnectionArgs {
	fn timeout(&self) -> Option<Option<Duration>> {
		match self.wait {
			true => Some(self.timeout_seconds.map(Duration::from_secs)),
			false => None,
		}
	}

	async fn connect(&self) -> Result<(qapi::futures::QapiStream<QmpStreamRead, QmpStreamWrite>, qapi::qmp::QapiCapabilities)> {
		if let Some(timeout) = self.timeout() {
			qemucomm::wait(timeout, qemucomm::wait_for_socket(&self.socket)).await?;
		}

		let stream = qapi::futures::QmpStreamTokio::open_uds(&self.socket).await?;
		let capabilities = stream.capabilities.clone();
		log::trace!("QEMU QMP Capabilities: {:#?}", capabilities);
		let stream = stream.negotiate().await?;

		Ok((stream, capabilities))
	}
}
