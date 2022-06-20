use anyhow::Result;
use clap::Parser;
use qapi::qmp;
use qemucomm::{Pair, args_options};
use super::{GlobalArgs, QmpStream};

#[derive(Parser, Debug)]
pub(crate) struct AddObject {
	#[clap(short, long)]
	id: Option<String>,
	arguments: Vec<Pair<String, String>>,
}

#[derive(Parser, Debug)]
pub(crate) struct DelObject {
	id: String,
}

impl AddObject {
	pub async fn run(self, qmp: QmpStream, _args: GlobalArgs) -> Result<i32> {
		qmp.execute(qmp::object_add(args_options(self.id, self.arguments)?)).await?;
		Ok(0)
	}
}

impl DelObject {
	pub async fn run(self, qmp: QmpStream, _args: GlobalArgs) -> Result<i32> {
		qmp.execute(qmp::object_del {
			id: self.id,
		}).await?;
		Ok(0)
	}
}
