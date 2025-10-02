use std::fs::File;
use std::io::Read;
use std::io::{self};
use std::path::Path;

use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha512;

pub trait FileReader {
	fn read(&self, path: impl AsRef<Path>, buf: &mut Vec<u8>) -> io::Result<()>;
}

pub struct NativeFileReader;

impl FileReader for NativeFileReader {
	fn read(&self, path: impl AsRef<Path>, buf: &mut Vec<u8>) -> io::Result<()> {
		let mut file = File::open(path)?;
		file.read_to_end(buf)?;
		Ok(())
	}
}

fn sha512_checksum(
	reader: &impl FileReader,
	path: impl AsRef<Path>,
	buf: &mut Vec<u8>,
) -> io::Result<String> {
	buf.clear();
	reader.read(path, buf)?;
	let mut hasher = Sha512::new();
	hasher.update(&buf);
	Ok(format!("{:x}", hasher.finalize()))
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Clone, Default, PartialOrd, Ord)]
pub struct Checksum {
	pub sha512: String,
}

impl Checksum {
	pub fn new() -> Self {
		Self {
			sha512: String::new(),
		}
	}

	pub fn is_empty(&self) -> bool {
		self.sha512.is_empty()
	}

	pub fn calculate(
		&mut self,
		reader: &impl FileReader,
		path: impl AsRef<Path>,
		buf: &mut Vec<u8>,
	) -> io::Result<()> {
		self.sha512 = sha512_checksum(reader, path, buf)?;
		Ok(())
	}
}
