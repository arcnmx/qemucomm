use anyhow::Result;
use clap::Parser;
use qapi::qmp;
use qemucomm::Pair;
use super::{GlobalArgs, QmpStream};

#[derive(Parser, Debug)]
pub(crate) struct AddDevice {
	#[clap(short, long)]
	id: Option<String>,
	driver: String,
	arguments: Vec<Pair<String, String>>,
	#[clap(short, long)]
	bus: Option<String>,
}

#[derive(Parser, Debug)]
pub(crate) struct DelDevice {
	id: String,
}

impl AddDevice {
	pub async fn run(self, qmp: QmpStream, _args: GlobalArgs) -> Result<i32> {
		qmp.execute(qmp::device_add {
			driver: self.driver,
			bus: self.bus,
			id: self.id,
			arguments: self.arguments.into_iter().map(Pair::object_pair).collect(),
		}).await?;
		Ok(0)
	}
}

impl DelDevice {
	pub async fn run(self, qmp: QmpStream, _args: GlobalArgs) -> Result<i32> {
		qmp.execute(qmp::device_del {
			id: self.id,
		}).await?;
		Ok(0)
	}
}
