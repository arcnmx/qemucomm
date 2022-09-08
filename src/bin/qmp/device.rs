use anyhow::{Result, Error, format_err};
use clap::Parser;
use qapi::qmp;
use tokio::time::{Duration, sleep, sleep_until};
use tokio::sync::broadcast;
use futures::{TryFutureExt, future};
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
	#[clap(short, long)]
	force: bool,
	#[clap(short, long)]
	no_clobber: bool,
}

#[derive(Parser, Debug)]
pub(crate) struct DelDevice {
	id: String,
	#[clap(short, long)]
	wait: bool,
}

impl AddDevice {
	pub async fn run(self, qmp: QmpStream, mut events: broadcast::Receiver<qmp::Event>, _args: GlobalArgs) -> Result<i32> {
		let deadline = None;

		let add = qmp::device_add {
			driver: self.driver,
			bus: self.bus,
			id: self.id,
			arguments: self.arguments.into_iter().map(Pair::object_pair).collect(),
		};

		if let Some(id) = add.id.as_ref() {
			let exists = if self.force || self.no_clobber {
				device_exists(&qmp, &id).await?
			} else {
				false
			};
			if exists {
				if self.no_clobber {
					log::warn!("{} already exists, nothing to do", id);
					return Ok(0)
				} else if self.force {
					log::info!("{} already exists, replacing...", id);
					device_del(&qmp, &mut events, true, id).await?;
				}
			}
		}

		if let Some(deadline) = deadline {
			sleep_until(deadline).await;
		}

		qmp.execute(add).await?;
		Ok(0)
	}
}

impl DelDevice {
	pub async fn run(self, qmp: QmpStream, mut events: broadcast::Receiver<qmp::Event>, _args: GlobalArgs) -> Result<i32> {
		device_del(&qmp, &mut events, self.wait, &self.id).await?;
		Ok(0)
	}
}

async fn device_exists(qmp: &QmpStream, id: &str) -> Result<bool> {
	let path = format!("/machine/peripheral/{}", id);
	match qmp.execute(qmp::qom_list { path }).await {
		Ok(..) => Ok(true),
		Err(qapi::ExecuteError::Qapi(qapi::Error { class: qapi::ErrorClass::DeviceNotFound, .. })) =>
			Ok(false),
		Err(e) => Err(e.into()),
	}
}

async fn device_del(qmp: &QmpStream, events: &mut broadcast::Receiver<qmp::Event>, wait: bool, id: &str) -> Result<()> {
	let delete = qmp.execute(qapi::qmp::device_del { id: id.into() })
		.map_err(Error::from)
		.map_ok(drop);
	if wait {
		let wait = async move {
			loop {
				match events.recv().await {
					Ok(qapi::qmp::Event::DEVICE_DELETED { ref data, .. }) if data.device.as_ref().map(|s| &s[..]) == Some(&id) => {
						// work around qemu bug. without this delay, device_add will work but the new device might be immediately deleted
						sleep(Duration::from_millis(128)).await;

						break Ok(())
					},
					Err(broadcast::error::RecvError::Closed) =>
						break Err(format_err!("Expected DEVICE_DELETED event")),
					_ => (),
				}
			}
		};
		future::try_join(wait, delete).await
			.map(|((), ())| ())
	} else {
		delete.await
	}
}
