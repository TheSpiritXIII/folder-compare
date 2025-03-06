use std::collections::HashMap;
use std::collections::VecDeque;
use std::fs::{self};
use std::io::Read;
use std::io::{self};
use std::path::Path;
use std::time::SystemTime;

use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha512;

use crate::matches::MatchKind;
use crate::progress::ProgressCounter;

// Size of buffer to compare files, optimized for an 8 KiB average file-size.
// Dinneen, Jesse & Nguyen, Ba. (2021). How Big Are Peoples' Computer Files? File Size Distributions
// Among User-managed Collections.
const BUF_SIZE: usize = 1024 * 8;

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Clone)]
pub struct Metadata {
	path: String,
	modified_time: std::time::SystemTime,
	created_time: std::time::SystemTime,
}

impl Metadata {
	fn normalize(&self) -> Self {
		let path = Path::new(&self.path);
		Self {
			path: path.file_name().unwrap().to_string_lossy().into_owned(),
			modified_time: self.modified_time,
			created_time: self.created_time,
		}
	}
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Clone)]
pub struct Checksum {
	sha512: String,
}

impl Checksum {
	fn is_empty(&self) -> bool {
		self.sha512.is_empty()
	}
}

#[derive(Serialize, Deserialize)]
pub struct FileMetadata {
	meta: Metadata,
	size: u64,
	checksum: Checksum,
}

#[derive(Serialize, Deserialize)]
pub struct Index {
	files: Vec<FileMetadata>,
	dirs: Vec<Metadata>,
}

fn normalize_path(path: &mut String) {
	*path = path.replace('\\', "/");
}

fn calculate_sha512_checksum(path: impl AsRef<Path>, buf: &mut Vec<u8>) -> io::Result<String> {
	let mut file = fs::File::open(path)?;
	let mut hasher = Sha512::new();
	file.read_to_end(buf)?;
	hasher.update(&buf);
	Ok(format!("{:x}", hasher.finalize()))
}

impl Index {
	// Recursively finds all files in the given directory and adds them to the index.
	pub fn from_path<T: ProgressCounter>(
		path: impl AsRef<std::path::Path>,
		progress: &T,
	) -> io::Result<Self> {
		let mut index = Self {
			files: Vec::new(),
			dirs: Vec::new(),
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
			let metadata = fs::metadata(&path)?;
			self.dirs.push(Metadata {
				path: current_path.to_string_lossy().into_owned(),
				modified_time: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
				created_time: metadata.created().unwrap_or(SystemTime::UNIX_EPOCH),
			});

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
		self.files.push(FileMetadata {
			meta: Metadata {
				path: path.as_ref().to_string_lossy().into_owned(),
				modified_time: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
				created_time: metadata.created().unwrap_or(SystemTime::UNIX_EPOCH),
			},
			size: metadata.len(),
			checksum: Checksum {
				sha512: String::new(),
			},
		});
		Ok(())
	}

	// Removes the directory in the given path.
	fn remove_dir(&mut self, path: impl AsRef<std::path::Path>) {
		let mut path_str = path.as_ref().to_string_lossy().into_owned();
		normalize_path(&mut path_str);
		// TODO: This logic is wrong.
		self.files.retain(|entry| !entry.meta.path.starts_with(&path_str));
		self.dirs.retain(|entry| entry.path != path_str);
	}

	// Removes the file in the given path.
	fn remove_file(&mut self, path: impl AsRef<std::path::Path>) {
		let mut path_str = path.as_ref().to_string_lossy().into_owned();
		normalize_path(&mut path_str);
		// TODO: Optimize this with binary search.
		self.files.retain(|entry| entry.meta.path != path_str);
	}

	pub fn calculate_all(&mut self) -> io::Result<()> {
		let mut buf = Vec::with_capacity(BUF_SIZE);
		for metadata in &mut self.files {
			metadata.checksum.sha512 = calculate_sha512_checksum(&metadata.meta.path, &mut buf)?;
		}
		Ok(())
	}

	// TODO: Make this Windows-only.
	// Normalizes the entries by replacing '\' with '/'.
	fn normalize(&mut self) {
		self.files.sort_by(|a, b| a.meta.path.cmp(&b.meta.path));
		for entry in &mut self.files {
			normalize_path(&mut entry.meta.path);
		}
		self.dirs.sort_by(|a, b| a.path.cmp(&b.path));
		for entry in &mut self.dirs {
			normalize_path(&mut entry.path);
		}
	}

	// TODO: Validate file extension?
	// Stores the index entries as RON on the filesystem.
	pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
		let json = ron::ser::to_string_pretty(&self, ron::ser::PrettyConfig::default()).unwrap();
		fs::write(path, json)?;
		Ok(())
	}

	// TODO: Versioning?
	// Opens an Index from a RON file.
	pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
		let json = fs::read_to_string(path)?;
		let index: Self = ron::from_str(&json).unwrap();
		Ok(index)
	}

	pub fn entry_count(&self) -> usize {
		self.files.len() + self.dirs.len()
	}

	pub fn file_count(&self) -> usize {
		self.files.len()
	}

	pub fn dirs_count(&self) -> usize {
		self.dirs.len()
	}

	pub fn calculate_duplicates<T: ProgressCounter>(
		&self,
		progress: &T,
	) -> Vec<(MatchKind, Vec<String>)> {
		let mut size_map = HashMap::<(String, u64), Vec<String>>::new();
		let mut metadata_map = HashMap::<(Metadata, u64), Vec<String>>::new();
		let mut checksum_map = HashMap::<(Checksum, u64), Vec<String>>::new();

		let mut count = 0;
		for file in &self.files {
			size_map
				.entry((file.meta.normalize().path, file.size))
				.or_default()
				.push(file.meta.path.clone());
			metadata_map
				.entry((file.meta.normalize(), file.size))
				.or_default()
				.push(file.meta.path.clone());
			if !file.checksum.is_empty() {
				checksum_map
					.entry((file.checksum.clone(), file.size))
					.or_default()
					.push(file.meta.path.clone());
			}

			count += 1;
			progress.update(count);
		}

		let mut matches = Vec::<(MatchKind, Vec<String>)>::new();
		for (_, path_list) in size_map {
			if path_list.len() > 1 {
				matches.push((MatchKind::Size, path_list));
			}
		}
		for (_, path_list) in metadata_map {
			if path_list.len() > 1 {
				matches.push((MatchKind::Metadata, path_list));
			}
		}
		for (_, path_list) in checksum_map {
			if path_list.len() > 1 {
				matches.push((MatchKind::Checksums, path_list));
			}
		}
		matches
	}
}
