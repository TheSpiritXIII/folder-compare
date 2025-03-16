use std::fs::{self};
use std::io::Read;
use std::io::{self};
use std::path::Path;

use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha512;

fn sha512_checksum(path: impl AsRef<Path>, buf: &mut Vec<u8>) -> io::Result<String> {
	buf.clear();
	let mut file = fs::File::open(path)?;
	let mut hasher = Sha512::new();
	file.read_to_end(buf)?;
	hasher.update(&buf);
	Ok(format!("{:x}", hasher.finalize()))
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Clone, Default, PartialOrd, Ord)]
pub struct Checksum {
	sha512: String,
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

	pub fn reset(&mut self) {
		self.sha512.clear();
	}

	pub fn calculate(&mut self, path: impl AsRef<Path>, buf: &mut Vec<u8>) -> io::Result<()> {
		self.sha512 = sha512_checksum(path, buf)?;
		Ok(())
	}
}
