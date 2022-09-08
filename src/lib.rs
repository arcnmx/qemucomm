use anyhow::{Result, Error, format_err};
use futures::Future;
use tokio::time::{Duration, timeout};
use serde::de::{IntoDeserializer, Deserialize};
use std::path::Path;
use std::str::FromStr;
use std::{io, fs};

pub fn key_val<K: FromStr, V: FromStr>(s: &str) -> Result<(K, V)> where
	K::Err: Into<Error>,
	V::Err: Into<Error>,
{
	let pos = s.find('=')
		.ok_or_else(|| format_err!("invalid KEY=value: no `=` found in `{}`", s))?;
		Ok((
			s[..pos].parse().map_err(Into::into)?,
			s[pos + 1..].parse().map_err(Into::into)?
		))
}

#[derive(Debug)]
pub struct Pair<K, V> {
	pub key: K,
	pub value: V,
}

impl Pair<String, String> {
	pub fn object_pair(self) -> (String, qapi::Any) {
		(self.key, object_value(self.value))
	}
}

impl<K: FromStr, V: FromStr> FromStr for Pair<K, V> where
	K::Err: Into<Error>,
	V::Err: Into<Error>,
{
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self> {
		key_val(s).map(Into::into)
	}
}

impl<K, V> From<(K, V)> for Pair<K, V> {
	fn from((key, value): (K, V)) -> Self {
		Pair {
			key,
			value,
		}
	}
}

pub type Arguments = Vec<Pair<String, String>>;

pub fn object_value(value: String) -> qapi::Any {
	qapi::Any::String(value) // TODO: determine how to parse numbers and so on
}

pub fn args_options(id: Option<String>, args: Arguments) -> Result<qapi::qmp::ObjectOptions> {
	let props = args.into_iter()
		.map(Pair::object_pair)
		.chain(id.map(|id| ("id".into(), object_value(id))))
		.collect::<qapi::Dictionary>();
	let props = qapi::Any::Object(props).into_deserializer();
	Deserialize::deserialize(props).map_err(Into::into)
}

pub async fn wait<O, E, F: Future<Output=std::result::Result<O, E>>>(duration: Option<Duration>, future: F) -> Result<O> where
	E: Into<Error>
{
	match duration {
		None => future.await.map_err(Into::into),
		Some(duration) => match timeout(duration, future).await {
			Err(e) => Err(e.into()),
			Ok(res) => res.map_err(Into::into),
		}
	}
}

pub async fn wait_for_socket(socket: &Path) -> Result<()> {
	use futures::StreamExt;
	use inotify::{
		EventMask,
		WatchMask,
		Inotify,
	};

	match fs::metadata(socket) {
		Err(e) if e.kind() == io::ErrorKind::NotFound => (),
		Err(e) => return Err(e.into()),
		Ok(_) => return Ok(()),
	}

	let (parent, file_name) = socket.parent()
		.and_then(|p| socket.file_name().map(|n| (p, n)))
		.ok_or_else(|| format_err!("socket path {} could not be parsed", socket.display()))?;

	let mut inotify = Inotify::init()?;
	inotify.add_watch(parent, WatchMask::CREATE)?;
	let mut buffer = [0u8; 4096];
	let mut events = inotify.event_stream(&mut buffer)?;

	while let Some(event) = events.next().await {
		let event = event?;
		if event.mask.contains(EventMask::CREATE) {
			if event.name.as_ref().map(|n| &n[..]) == Some(file_name) {
				return Ok(())
			} else {
				log::info!("ignoring inotify event for {:?}", event);
			}
		} else {
			log::warn!("unexpected inotify event {:?}", event);
		}
	}

	Err(format_err!("inotify ran out of events?"))
}
