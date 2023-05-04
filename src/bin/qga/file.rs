use anyhow::{Result, anyhow};
use futures::Future;
use qapi::qga;
use log::{debug, warn};
use bytes::BytesMut;
use tokio::{time::{timeout, Duration}, io::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt}};
use clap::{Parser, ValueEnum};
use super::{GlobalArgs, QgaStream};

type QgaResult<T> = std::result::Result<T, qapi::ExecuteError>;

#[derive(Parser, Debug)]
/// Write a file to the guest filesystem
pub(crate) struct FileOpen {
	path: String,
	#[clap(short, long)]
	mode: Option<String>,
	#[clap(short, long)]
	offset: Option<i64>,
	#[clap(short, long, default_value = "beginning")]
	seek: SeekWhence,
	#[clap(short, long, default_value = "60")]
	timeout_seconds: u64,
}

impl FileOpen {
	pub async fn open<'q>(self, qga: &'q QgaStream) -> Result<QgaFile<'q>> {
		let file = QgaFile::open(qga, self.path, self.mode).await?;

		if let Some(offset) = self.offset {
			if let Err(e) = file.seek(self.seek.into(), offset).await {
				let _ = file.close().await;
				return Err(e.into())
			}
		}

		Ok(file)
	}

	pub async fn run<'q, O: FnOnce(QgaFile<'q>) -> F, F: Future<Output=Result<i32>> + 'q>(self, qga: &'q QgaStream, _args: GlobalArgs, op: O) -> Result<i32> {
		let duration = Duration::from_secs(self.timeout_seconds);
		let res = async move {
			let file = self.open(qga).await?;

			let res = op(file.clone()).await;
			let _ = file.close().await;
			res
		};

		match timeout(duration, res).await {
			Ok(res) => res,
			Err(e) => Err(e.into()),
		}
	}
}

#[derive(Parser, Debug)]
/// Write a file to the guest filesystem
pub(crate) struct WriteFile {
	#[command(flatten)]
	open: FileOpen,
}

impl WriteFile {
	pub async fn run(self, qga: QgaStream, args: GlobalArgs) -> Result<i32> {
		self.open.run(&qga, args, |file| async move {
			let read = tokio::io::stdin();
			file.write_from(read).await?;

			Ok(0)
		}).await
	}
}

#[derive(Parser, Debug)]
/// Read a file from the guest filesystem
pub(crate) struct ReadFile {
	#[command(flatten)]
	open: FileOpen,
}

impl ReadFile {
	pub async fn run(self, qga: QgaStream, args: GlobalArgs) -> Result<i32> {
		self.open.run(&qga, args, |file| async move {
			let write = tokio::io::stdout();
			file.read_to(write).await?;

			Ok(0)
		}).await
	}
}

#[derive(Clone)]
pub struct QgaFile<'q> {
	qga: &'q QgaStream,
	handle: i64,
}

impl<'q> QgaFile<'q> {
	pub async fn open<P: Into<String>>(qga: &'q QgaStream, path: P, mode: Option<String>) -> QgaResult<QgaFile<'q>> {
		let path = path.into();
		debug!("opening guest path {:?}", path);
		let handle = qga.execute(qga::guest_file_open {
			path,
			mode,
		}).await?;
		debug!("opened guest file:{}", handle);
		Ok(QgaFile {
			qga,
			handle,
		})
	}

	pub async fn close(self) -> QgaResult<()> {
		let res = self.qga.execute(qga::guest_file_close {
			handle: self.handle,
		}).await;
		if let Err(e) = &res {
			warn!("failed to close guest file: {e}");
		}
		res.map(drop)
	}

	pub async fn seek(&self, whence: qga::QGASeek, offset: i64) -> QgaResult<qga::GuestFileSeek> {
		let res = self.qga.execute(qga::guest_file_seek {
			handle: self.handle,
			offset,
			whence: qga::GuestFileWhence::name(whence),
		}).await?;
		debug!("seeked file:{} to {}{}", self.handle, res.position, if res.eof { " (EOF)" } else { "" });
		Ok(res)
	}

	pub async fn write_buf<B: Into<Vec<u8>>>(&self, buf: B) -> QgaResult<qga::GuestFileWrite> {
		let buf = buf.into();
		self.qga.execute(qga::guest_file_write {
			handle: self.handle,
			count: Some(buf.len() as i64),
			buf_b64: buf,
		}).await
	}

	pub async fn read_buf(&self, len: Option<usize>) -> QgaResult<qga::GuestFileRead> {
		self.qga.execute(qga::guest_file_read {
			handle: self.handle,
			count: len.and_then(|len| len.try_into().ok()),
		}).await
	}

	pub async fn write_from<R: AsyncRead + Unpin>(&self, mut read: R) -> Result<u64> {
		let mut buf = BytesMut::with_capacity(0x1000);
		let mut total = 0u64;
		loop {
			match read.read_buf(&mut buf).await? {
				0 if buf.is_empty() => break,
				_ => (),
			}
			let written = match self.write_buf(&buf[..]).await?.count {
				w if w <= 0 => return Err(anyhow!("Guest wrote {w} bytes")),
				w => w as usize,
			};
			total = total.saturating_add(written as u64);
			let _ = buf.split_to(written);
		}
		Ok(total)
	}

	pub async fn read_to<W: AsyncWrite + Unpin>(&self, mut write: W) -> Result<u64> {
		let mut total = 0u64;
		loop {
			let read = self.read_buf(None).await?;
			write.write_all(&read.buf_b64).await?;
			total = total.saturating_add(read.count as u64);
			if read.eof || read.count == 0 {
				break
			}
		}
		Ok(total)
	}
}

#[derive(ValueEnum, Copy, Clone, Debug)]
pub enum SeekWhence {
	Beginning,
	End,
}

impl Into<qga::QGASeek> for SeekWhence {
	fn into(self) -> qga::QGASeek {
		match self {
			SeekWhence::Beginning => qga::QGASeek::set,
			SeekWhence::End => qga::QGASeek::end,
		}
	}
}
