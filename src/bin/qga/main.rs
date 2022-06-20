use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use tokio::time::timeout;
use std::time::Duration;
use std::path::PathBuf;

mod exec;
mod info;
mod shutdown;

pub(crate) type QgaStream = qapi::futures::QapiService<qapi::futures::QgaStreamTokio<tokio::io::WriteHalf<tokio::net::UnixStream>>>;

#[derive(Args, Debug)]
pub(crate) struct GlobalArgs {
}

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Cli {
	/// QEMU guest agent socket path
	#[clap(short, long, env("QEMUCOMM_QGA_SOCKET_PATH"))]
	socket: PathBuf,
	#[clap(flatten)]
	args: GlobalArgs,
	#[clap(subcommand)]
	command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
	Exec(exec::Exec),
	Info(info::Info),
	Shutdown(shutdown::Shutdown),
}

#[tokio::main]
async fn main() -> Result<()> {
	::env_logger::init();

	let args = Cli::parse();

	let stream = qapi::futures::QgaStreamTokio::open_uds(&args.socket).await?;
	let (qga, handle) = stream.spawn_tokio();

	let sync_value = &qga as *const _ as usize as i32;
	qga.guest_sync(sync_value).await?;

	let res = match args.command {
		Command::Exec(c) => c.run(qga, args.args).await,
		Command::Info(c) => c.run(qga, args.args).await,
		Command::Shutdown(c) => c.run(qga, args.args).await,
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
