use std::fs::{self};
use std::io::{self};
use std::path::Path;
use std::time::SystemTime;

use serde::Deserialize;
use serde::Serialize;

fn normalized_path_str(path: &str) -> String {
	let mut path = path.replace('\\', "/");
	if path.ends_with('/') {
		path.truncate(path.len() - 1);
	}
	path
}

/// Normalizes entries on Windows by replacing '\' with '/'.
// TODO: Make this Windows-only.
pub fn normalized_path(path: impl AsRef<Path>) -> String {
	normalized_path_str(&path.as_ref().to_string_lossy())
}

#[cfg(test)]
pub fn parent_str(path: &str) -> Option<&str> {
	if let Some(index) = path.rfind('/') {
		return Some(&path[..index]);
	}
	None
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Clone)]
pub struct Metadata {
	path: String,
	created_time: std::time::SystemTime,
	modified_time: std::time::SystemTime,
}

impl Metadata {
	#[cfg(test)]
	pub fn with_path(path: impl AsRef<Path>) -> Self {
		Self {
			path: normalized_path(path.as_ref()),
			created_time: SystemTime::UNIX_EPOCH,
			modified_time: SystemTime::UNIX_EPOCH,
		}
	}

	pub fn from_path(path: impl AsRef<Path>) -> io::Result<Self> {
		let metadata = fs::metadata(path.as_ref())?;
		Ok(Self {
			path: normalized_path(path.as_ref()),
			created_time: metadata.created().unwrap_or(SystemTime::UNIX_EPOCH),
			modified_time: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
		})
	}

	pub fn path(&self) -> &str {
		&self.path
	}

	pub fn name(&self) -> &str {
		if let Some(index) = self.path.rfind('/') {
			return &self.path[(index + 1)..];
		}
		&self.path
	}

	#[cfg(test)]
	pub fn parent(&self) -> Option<&str> {
		parent_str(&self.path)
	}

	pub fn created_time(&self) -> SystemTime {
		self.created_time
	}

	pub fn modified_time(&self) -> SystemTime {
		self.modified_time
	}
}
