use anyhow::Result;
use qapi::qga;
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
		qga.execute(qga::guest_shutdown {
			mode: Some(self.mode.into()),
		}).await?;

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
