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
			index.add_file(path)?;
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
			self.add_file(path)
		} else {
			// TODO: io::Result doesn't make sense for this.
			Err(io::Error::from(io::ErrorKind::Unsupported))
		}
	}

	fn add_dir(&mut self, path: impl AsRef<std::path::Path>) -> io::Result<()> {
		self.read_dir_recursive(path.as_ref())?;
		self.normalize();
		Ok(())
	}

	fn add_file(&mut self, path: impl AsRef<std::path::Path>) -> io::Result<()> {
		let checksum = Self::calculate_sha512_checksum(path.as_ref())?;
		self.entries.push(Metadata {
			filepath: path.as_ref().to_string_lossy().into_owned(),
			checksum: Checksum {
				sha512: checksum,
			},
		});
		Ok(())
	}

	// Removes the directory in the given path.
	fn remove_dir(&mut self, path: impl AsRef<std::path::Path>) {
		let path_str = path.as_ref().to_string_lossy().into_owned();
		self.entries.retain(|entry| !entry.filepath.starts_with(&path_str));
	}

	// Removes the file in the given path.
	fn remove_file(&mut self, path: impl AsRef<std::path::Path>) {
		let path_str = path.as_ref().to_string_lossy().into_owned();
		// TODO: Optimize this with binary search.
		self.entries.retain(|entry| entry.filepath != path_str);
	}

	fn read_dir_recursive(&mut self, path: &Path) -> io::Result<()> {
		for entry in fs::read_dir(path)? {
			let entry = entry?;
			let path = entry.path();
			if path.is_dir() {
				self.read_dir_recursive(&path)?;
			} else {
				self.add_file(path)?;
			}
		}
		Ok(())
	}

	fn calculate_sha512_checksum(path: impl AsRef<Path>) -> io::Result<String> {
		let mut file = fs::File::open(path)?;
		let mut hasher = Sha512::new();
		let mut buffer = Vec::new();
		file.read_to_end(&mut buffer)?;
		hasher.update(&buffer);
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
