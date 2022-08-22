use anyhow::Result;
use clap::Parser;
use qapi::qmp;
use super::{GlobalArgs, QmpStream};

#[derive(Parser, Debug)]
pub(crate) struct HumanCommand {
	#[clap(long = "cpu")]
	pub cpu_index: Option<i64>,
	pub command: String,
}

impl HumanCommand {
	pub async fn run(self, qmp: QmpStream, _args: GlobalArgs) -> Result<i32> {
		let response = qmp.execute(qmp::human_monitor_command {
			cpu_index: self.cpu_index,
			command_line: self.command,
		}).await?;
		println!("{}", response);
		Ok(0)
	}
}
