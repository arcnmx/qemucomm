use anyhow::Result;
use qapi::qga;
use log::warn;
use tokio::time::{timeout, Duration};
use clap::{Parser, ValueEnum};
use super::{GlobalArgs, QgaStream};

#[derive(Parser, Debug)]
/// Tells the guest to initiate a system shutdown
pub(crate) struct Shutdown {
	#[clap(short, long, value_enum, default_value_t = Mode::Powerdown)]
	mode: Mode,
}

impl Shutdown {
	pub async fn run(self, qga: QgaStream, _args: GlobalArgs) -> Result<i32> {
		let cmd = qga.execute(qga::guest_shutdown {
			mode: Some(self.mode.into()),
		});

		match timeout(Duration::from_secs(1), cmd).await {
			Ok(res) => res.map(drop)?,
			Err(_) => warn!("Shutdown response timed out"),
		}

		Ok(0)
	}
}

#[derive(ValueEnum, Copy, Clone, Debug)]
pub enum Mode {
	Powerdown,
	/// halt immediately
	Halt,
	Reboot,
}

impl Into<qga::GuestShutdownMode> for Mode {
	fn into(self) -> qga::GuestShutdownMode {
		match self {
			Mode::Reboot => qga::GuestShutdownMode::Reboot,
			Mode::Powerdown => qga::GuestShutdownMode::Powerdown,
			Mode::Halt => qga::GuestShutdownMode::Halt,
		}
	}
}
