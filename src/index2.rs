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
	fn remove_dir(self: &mut Self, path: impl AsRef<std::path::Path>) {
		let path_str = path.as_ref().to_string_lossy().into_owned();
		self.entries.retain(|entry| !entry.filepath.starts_with(&path_str));
	}

	// Recursively finds all files in the given directory and adds them to the index.
	pub fn from_dir(path: impl AsRef<std::path::Path>) -> io::Result<Self> {
		if !path.as_ref().is_dir() {
			return Err(io::Error::from(io::ErrorKind::NotADirectory));
		}

		let mut entries = Vec::new();
		Self::read_dir_recursive(path.as_ref(), &mut entries)?;

		let mut index = Self {
			entries,
		};
		index.normalize();
		Ok(index)
	}

	pub fn add_dir(self: &mut Self, path: impl AsRef<std::path::Path>) -> io::Result<()> {
		if !path.as_ref().is_dir() {
			return Err(io::Error::from(io::ErrorKind::NotADirectory));
		}
		self.remove_dir(path.as_ref());

		Self::read_dir_recursive(path.as_ref(), &mut self.entries)?;
		self.normalize();
		Ok(())
	}

	fn read_dir_recursive(path: &Path, entries: &mut Vec<Metadata>) -> io::Result<()> {
		for entry in fs::read_dir(path)? {
			let entry = entry?;
			let path = entry.path();
			if path.is_dir() {
				Self::read_dir_recursive(&path, entries)?;
			} else {
				let checksum = Self::calculate_sha512_checksum(&path)?;
				entries.push(Metadata {
					filepath: path.to_string_lossy().into_owned(),
					checksum: Checksum {
						sha512: checksum,
					},
				});
			}
		}
		Ok(())
	}

	fn calculate_sha512_checksum(path: &Path) -> io::Result<String> {
		let mut file = fs::File::open(path)?;
		let mut hasher = Sha512::new();
		let mut buffer = Vec::new();
		file.read_to_end(&mut buffer)?;
		hasher.update(&buffer);
		Ok(format!("{:x}", hasher.finalize()))
	}

	// Normalizes the entries by replacing '\' with '/'.
	fn normalize(&mut self) {
		self.entries.sort_by(|a, b| a.filepath.cmp(&b.filepath));
		for entry in &mut self.entries {
			entry.filepath = entry.filepath.replace('\\', "/");
		}
	}

	// Stores the index entries as JSON on the filesystem.
	pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
		let json =
			ron::ser::to_string_pretty(&self.entries, ron::ser::PrettyConfig::default()).unwrap();
		fs::write(path, json)?;
		Ok(())
	}

	// Opens an Index from a JSON file.
	pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
		let json = fs::read_to_string(path)?;
		let entries: Vec<Metadata> = ron::from_str(&json).unwrap();
		Ok(Self {
			entries,
		})
	}
}
