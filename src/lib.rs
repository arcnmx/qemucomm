use anyhow::{Result, Error, format_err};
use serde::de::{IntoDeserializer, Deserialize};
use std::str::FromStr;

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
