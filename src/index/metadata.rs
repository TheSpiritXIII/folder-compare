use std::fs::{self};
use std::io::{self};
use std::path::Path;
use std::time::SystemTime;

use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Clone)]
pub struct Metadata {
	// TODO: Don't make public.
	pub path: String,
	modified_time: std::time::SystemTime,
	created_time: std::time::SystemTime,
}

impl Metadata {
	pub fn from_path(path: impl AsRef<Path>) -> io::Result<Self> {
		let metadata = fs::metadata(path.as_ref())?;
		Ok(Metadata {
			path: path.as_ref().to_string_lossy().into_owned(),
			modified_time: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
			created_time: metadata.created().unwrap_or(SystemTime::UNIX_EPOCH),
		})
	}

	pub fn normalize(&self) -> Self {
		let path = Path::new(&self.path);
		Self {
			path: path.file_name().unwrap().to_string_lossy().into_owned(),
			modified_time: self.modified_time,
			created_time: self.created_time,
		}
	}

	pub fn path(&self) -> &str {
		&self.path
	}
}
