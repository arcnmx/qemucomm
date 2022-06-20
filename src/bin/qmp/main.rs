use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use tokio::time::timeout;
use std::time::Duration;
use std::path::PathBuf;

mod status;
mod device;
mod object;

pub(crate) type QmpStream = qapi::futures::QapiService<qapi::futures::QmpStreamTokio<tokio::io::WriteHalf<tokio::net::UnixStream>>>;

#[derive(Args, Debug)]
pub(crate) struct GlobalArgs {
}

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Cli {
	/// QEMU QMP socket path
	#[clap(short, long, env("QEMUCOMM_QMP_SOCKET_PATH"))]
	socket: PathBuf,
	#[clap(flatten)]
	args: GlobalArgs,
	#[clap(subcommand)]
	command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
	Status(status::Status),
	AddDevice(device::AddDevice),
	DelDevice(device::DelDevice),
	AddObject(object::AddObject),
	DelObject(object::DelObject),
}

#[tokio::main]
async fn main() -> Result<()> {
	::env_logger::init();

	let args = Cli::parse();

	let stream = qapi::futures::QmpStreamTokio::open_uds(&args.socket).await?;
	log::trace!("QEMU QMP Capabilities: {:#?}", stream.capabilities);
	let stream = stream.negotiate().await?;
	let (qmp, handle) = stream.spawn_tokio();

	let res = match args.command {
		Command::Status(c) => c.run(qmp, args.args).await,
		Command::AddDevice(c) => c.run(qmp, args.args).await,
		Command::DelDevice(c) => c.run(qmp, args.args).await,
		Command::AddObject(c) => c.run(qmp, args.args).await,
		Command::DelObject(c) => c.run(qmp, args.args).await,
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
