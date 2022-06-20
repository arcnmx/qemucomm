use anyhow::Result;
use clap::Parser;
use qapi::qmp;
use super::{GlobalArgs, QmpStream};

#[derive(Parser, Debug)]
pub(crate) struct Status {
}

impl Status {
	pub async fn run(self, qmp: QmpStream, _args: GlobalArgs) -> Result<i32> {
		let status = qmp.execute(qmp::query_status { }).await?;
		println!("VCPU Status: {:#?}", status);
		Ok(0)
	}
}
