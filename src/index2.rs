use std::collections::VecDeque;
use std::fs;
use std::io::Read;
use std::io::{self};
use std::path::Path;

use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha512;

#[derive(Serialize, Deserialize)]
pub struct Metadata {
	filepath: String,
	checksum: Checksum,
}

#[derive(Serialize, Deserialize)]
pub struct Checksum {
	sha512: String,
}

#[derive(Serialize, Deserialize)]
pub struct Index {
	entries: Vec<Metadata>,
}

impl Index {
	// Recursively finds all files in the given directory and adds them to the index.
	pub fn from_path(path: impl AsRef<std::path::Path>) -> io::Result<Self> {
		let mut index = Self {
			entries: Vec::new(),
		};
		if path.as_ref().is_dir() {
			index.add_dir(path)?;
			return Ok(index);
		} else if path.as_ref().is_file() {
			index.add_file(path);
			return Ok(index);
		}
		// TODO: io::Result doesn't make sense for this.
		Err(io::Error::from(io::ErrorKind::Unsupported))
	}

	pub fn add(&mut self, path: impl AsRef<std::path::Path>) -> io::Result<()> {
		if path.as_ref().is_dir() {
			self.remove_dir(path.as_ref());
			self.add_dir(path)
		} else if path.as_ref().is_file() {
			self.remove_file(path.as_ref());
			self.add_file(path);
			Ok(())
		} else {
			// TODO: io::Result doesn't make sense for this.
			Err(io::Error::from(io::ErrorKind::Unsupported))
		}
	}

	fn add_dir(&mut self, path: impl AsRef<std::path::Path>) -> io::Result<()> {
		let mut queue = VecDeque::new();
		queue.push_back(path.as_ref().to_path_buf());

		while let Some(current_path) = queue.pop_front() {
			for entry in fs::read_dir(current_path)? {
				let entry = entry?;
				let path = entry.path();
				if path.is_dir() {
					queue.push_back(path);
				} else {
					self.add_file(path);
				}
			}
		}

		self.normalize();
		Ok(())
	}

	fn add_file(&mut self, path: impl AsRef<std::path::Path>) {
		self.entries.push(Metadata {
			filepath: path.as_ref().to_string_lossy().into_owned(),
			checksum: Checksum {
				sha512: String::new(),
			},
		});
	}

	// Removes the directory in the given path.
	fn remove_dir(&mut self, path: impl AsRef<std::path::Path>) {
		let path_str = path.as_ref().to_string_lossy().into_owned();
		// TODO: This logic is wrong.
		self.entries.retain(|entry| !entry.filepath.starts_with(&path_str));
	}

	// Removes the file in the given path.
	fn remove_file(&mut self, path: impl AsRef<std::path::Path>) {
		let path_str = path.as_ref().to_string_lossy().into_owned();
		// TODO: Optimize this with binary search.
		self.entries.retain(|entry| entry.filepath != path_str);
	}

	pub fn calculate_all(&mut self) -> io::Result<()> {
		let mut buf = Vec::new();
		for metadata in &self.entries {
			Self::calculate_sha512_checksum(&metadata.filepath, &mut buf)?;
		}
		Ok(())
	}

	fn calculate_sha512_checksum(path: impl AsRef<Path>, buf: &mut Vec<u8>) -> io::Result<String> {
		let mut file = fs::File::open(path)?;
		let mut hasher = Sha512::new();
		file.read_to_end(buf)?;
		hasher.update(&buf);
		Ok(format!("{:x}", hasher.finalize()))
	}

	// TODO: Make this Windows-only.
	// Normalizes the entries by replacing '\' with '/'.
	fn normalize(&mut self) {
		self.entries.sort_by(|a, b| a.filepath.cmp(&b.filepath));
		for entry in &mut self.entries {
			entry.filepath = entry.filepath.replace('\\', "/");
		}
	}

	// TODO: Validate file extension?
	// Stores the index entries as RON on the filesystem.
	pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
		let json =
			ron::ser::to_string_pretty(&self.entries, ron::ser::PrettyConfig::default()).unwrap();
		fs::write(path, json)?;
		Ok(())
	}

	// TODO: Versioning?
	// Opens an Index from a RON file.
	pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
		let json = fs::read_to_string(path)?;
		let entries: Vec<Metadata> = ron::from_str(&json).unwrap();
		Ok(Self {
			entries,
		})
	}
}
