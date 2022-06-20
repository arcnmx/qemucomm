use anyhow::Result;
use qapi::qga;
use clap::Parser;
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
