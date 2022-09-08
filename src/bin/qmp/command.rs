use anyhow::Result;
use clap::Parser;
use qapi::qmp;
use super::{GlobalArgs, QmpStream};

#[derive(Parser, Debug)]
pub(crate) struct StopCommand {
}

impl StopCommand {
	pub async fn run(self, qmp: QmpStream, _args: GlobalArgs) -> Result<i32> {
		qmp.execute(qmp::stop { }).await?;
		Ok(0)
	}
}

#[derive(Parser, Debug)]
pub(crate) struct ContinueCommand {
}

impl ContinueCommand {
	pub async fn run(self, qmp: QmpStream, _args: GlobalArgs) -> Result<i32> {
		qmp.execute(qmp::cont { }).await?;
		Ok(0)
	}
}

#[derive(Parser, Debug)]
pub(crate) struct QuitCommand {
}

impl QuitCommand {
	pub async fn run(self, qmp: QmpStream, _args: GlobalArgs) -> Result<i32> {
		qmp.execute(qmp::quit { }).await?;
		Ok(0)
	}
}
