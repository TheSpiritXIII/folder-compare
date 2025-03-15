use std::fs;
use std::io;
use std::path::Path;

use serde::Deserialize;
use serde::Serialize;

use super::checksum::Checksum;
use super::metadata::Metadata;

#[derive(Serialize, Deserialize, PartialEq, PartialOrd, Eq, Ord, Hash, Clone)]
pub struct File {
	pub meta: Metadata,
	pub size: u64,
	pub checksum: Checksum,
}

impl File {
	pub fn from_path(path: impl AsRef<Path>) -> io::Result<Self> {
		let metadata = fs::metadata(path.as_ref())?;
		Ok(Self {
			meta: Metadata::from_metadata(path.as_ref(), &metadata),
			size: metadata.len(),
			checksum: Checksum::new(),
		})
	}
}

#[derive(Serialize, Deserialize, PartialEq, PartialOrd, Eq, Ord, Hash, Clone)]
pub struct Dir {
	pub meta: Metadata,
}

impl Dir {
	pub fn from_path(path: impl AsRef<Path>) -> io::Result<Self> {
		Ok(Self {
			meta: Metadata::from_path(path)?,
		})
	}
}
