use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use tokio::task::JoinHandle;
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

#[derive(Args, Debug)]
pub(crate) struct ConnectionArgs {
	/// QEMU guest agent socket path
	#[arg(short, long, env("QEMUCOMM_QGA_SOCKET_PATH"))]
	socket: PathBuf,
	#[arg(long, short = 'S')]
	no_sync: bool,
	#[arg(short, long)]
	wait: bool,
	#[arg(short, long = "timeout")]
	timeout_seconds: Option<u64>,
}

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
	#[command(flatten)]
	connection: ConnectionArgs,
	#[command(flatten)]
	args: GlobalArgs,
	#[command(subcommand)]
	command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
	Ping(info::Ping),
	Info(info::Info),
	Exec(exec::Exec),
	Shutdown(shutdown::Shutdown),
}

#[tokio::main]
async fn main() -> Result<()> {
	::env_logger::init();

	let args = Cli::parse();

	let (qga, handle) = args.connection.connect().await?;

	let sync_value = &qga as *const _ as usize as i32;
	qga.guest_sync(sync_value).await?;

	let res = match args.command {
		Command::Ping(c) => c.run(qga, args.args).await,
		Command::Info(c) => c.run(qga, args.args).await,
		Command::Exec(c) => c.run(qga, args.args).await,
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

impl ConnectionArgs {
	fn timeout(&self) -> Option<Option<Duration>> {
		match self.wait {
			true => Some(self.timeout_seconds.map(Duration::from_secs)),
			false => None,
		}
	}

	async fn connect(&self) -> Result<(QgaStream, JoinHandle<()>)> {
		if let Some(timeout) = self.timeout() {
			qemucomm::wait(timeout, qemucomm::wait_for_socket(&self.socket)).await?;
		}

		let stream = qapi::futures::QgaStreamTokio::open_uds(&self.socket).await?;
		let (qga, handle) = stream.spawn_tokio();

		if !self.no_sync {
			let sync_value = &qga as *const _ as usize as i32;
			qga.guest_sync(sync_value).await?;
		}

		Ok((qga, handle))
	}
}
