use std::collections::VecDeque;
use std::fs;
use std::io::Read;
use std::io::{self};
use std::path::Path;
use std::time::SystemTime;

use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha512;

use crate::progress::ProgressCounter;

#[derive(Serialize, Deserialize)]
pub struct Metadata {
	filepath: String,
	size: u64,
	modified_time: std::time::SystemTime,
	created_time: std::time::SystemTime,
	checksum: Checksum,
}

#[derive(Serialize, Deserialize)]
pub struct Checksum {
	sha512: String,
}

#[derive(Serialize, Deserialize)]
pub struct Index {
	files: Vec<Metadata>,
}

impl Index {
	// Recursively finds all files in the given directory and adds them to the index.
	pub fn from_path<T: ProgressCounter>(
		path: impl AsRef<std::path::Path>,
		progress: &T,
	) -> io::Result<Self> {
		let mut index = Self {
			files: Vec::new(),
		};
		if path.as_ref().is_dir() {
			index.add_dir(path.as_ref(), progress)?;
			return Ok(index);
		} else if path.as_ref().is_file() {
			index.add_file(path)?;
			return Ok(index);
		}
		// TODO: io::Result doesn't make sense for this.
		Err(io::Error::from(io::ErrorKind::Unsupported))
	}

	pub fn add<T: ProgressCounter>(
		&mut self,
		path: impl AsRef<std::path::Path>,
		progress: &T,
	) -> io::Result<()> {
		if path.as_ref().is_dir() {
			self.remove_dir(path.as_ref());
			self.add_dir(path, progress)
		} else if path.as_ref().is_file() {
			self.remove_file(path.as_ref());
			self.add_file(path)?;
			progress.update(1);
			Ok(())
		} else {
			// TODO: io::Result doesn't make sense for this.
			Err(io::Error::from(io::ErrorKind::Unsupported))
		}
	}

	fn add_dir<T: ProgressCounter>(
		&mut self,
		path: impl AsRef<std::path::Path>,
		progress: &T,
	) -> io::Result<()> {
		let mut queue = VecDeque::new();
		queue.push_back(path.as_ref().to_path_buf());

		let mut count = 0;
		while let Some(current_path) = queue.pop_front() {
			count += 1;
			progress.update(count);
			for entry in fs::read_dir(current_path)? {
				let entry = entry?;
				let path = entry.path();
				if path.is_dir() {
					queue.push_back(path);
				} else {
					self.add_file(path)?;
					count += 1;
					progress.update(count);
				}
			}
		}

		self.normalize();
		Ok(())
	}

	fn add_file(&mut self, path: impl AsRef<std::path::Path>) -> io::Result<()> {
		let metadata = fs::metadata(path.as_ref())?;
		self.files.push(Metadata {
			filepath: path.as_ref().to_string_lossy().into_owned(),
			size: metadata.len(),
			modified_time: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
			created_time: metadata.created().unwrap_or(SystemTime::UNIX_EPOCH),
			checksum: Checksum {
				sha512: String::new(),
			},
		});
		Ok(())
	}

	// Removes the directory in the given path.
	fn remove_dir(&mut self, path: impl AsRef<std::path::Path>) {
		let path_str = path.as_ref().to_string_lossy().into_owned();
		// TODO: This logic is wrong.
		self.files.retain(|entry| !entry.filepath.starts_with(&path_str));
	}

	// Removes the file in the given path.
	fn remove_file(&mut self, path: impl AsRef<std::path::Path>) {
		let path_str = path.as_ref().to_string_lossy().into_owned();
		// TODO: Optimize this with binary search.
		self.files.retain(|entry| entry.filepath != path_str);
	}

	pub fn calculate_all(&mut self) -> io::Result<()> {
		let mut buf = Vec::new();
		for metadata in &self.files {
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
		self.files.sort_by(|a, b| a.filepath.cmp(&b.filepath));
		for entry in &mut self.files {
			entry.filepath = entry.filepath.replace('\\', "/");
		}
	}

	// TODO: Validate file extension?
	// Stores the index entries as RON on the filesystem.
	pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
		let json =
			ron::ser::to_string_pretty(&self.files, ron::ser::PrettyConfig::default()).unwrap();
		fs::write(path, json)?;
		Ok(())
	}

	// TODO: Versioning?
	// Opens an Index from a RON file.
	pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
		let json = fs::read_to_string(path)?;
		let entries: Vec<Metadata> = ron::from_str(&json).unwrap();
		Ok(Self {
			files: entries,
		})
	}
}
