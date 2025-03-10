mod checksum;
mod metadata;

use std::collections::HashMap;
use std::collections::VecDeque;
use std::fs::{self};
use std::io::{self};
use std::path::Path;
use std::time::SystemTime;

use checksum::Checksum;
use metadata::Metadata;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;

use crate::progress::ProgressCounter;

// Size of buffer to compare files, optimized for an 8 KiB average file-size.
// Dinneen, Jesse & Nguyen, Ba. (2021). How Big Are Peoples' Computer Files? File Size Distributions
// Among User-managed Collections.
const BUF_SIZE: usize = 1024 * 8;

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

	#[serde(skip_serializing)]
	#[serde(skip_deserializing)]
	dirty: bool,
}

impl Index {
	pub fn new() -> Self {
		Self {
			files: Vec::new(),
			dirs: Vec::new(),
			dirty: false,
		}
	}

	// Recursively finds all files in the given directory and adds them to the index.
	pub fn from_path<T: ProgressCounter>(
		path: impl AsRef<std::path::Path>,
		progress: &T,
	) -> io::Result<Self> {
		let mut index = Self::new();
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
			self.dirty = true;
			self.remove_dir(path.as_ref());
			self.add_dir(path, progress)
		} else if path.as_ref().is_file() {
			self.dirty = true;
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

		let mut is_root = path.as_ref().parent().is_none();

		let mut count = 0;
		while let Some(current_path) = queue.pop_front() {
			let metadata = Metadata::from_path(&current_path)?;
			self.dirs.push(metadata);

			count += 1;
			progress.update(count);
			for entry in fs::read_dir(current_path)? {
				let entry = entry?;
				let path = entry.path();
				if path.is_dir() {
					if is_root {
						if let Some(name) = path.file_name() {
							if name == "$RECYCLE.BIN" || name == "System Volume Information" {
								continue;
							}
						}
					}

					queue.push_back(path);
				} else {
					self.add_file(path)?;
					count += 1;
					progress.update(count);
				}
			}
			is_root = false;
		}

		self.normalize();
		Ok(())
	}

	fn add_file(&mut self, path: impl AsRef<std::path::Path>) -> io::Result<()> {
		let metadata = fs::metadata(path.as_ref())?;
		self.files.push(FileMetadata {
			meta: Metadata::from_path(path.as_ref())?,
			size: metadata.len(),
			checksum: Checksum::new(),
		});
		Ok(())
	}

	// Removes the directory in the given path.
	fn remove_dir(&mut self, path: impl AsRef<std::path::Path>) {
		let path_str = metadata::normalized_path(path.as_ref());
		// TODO: This logic might be wrong.
		self.files.retain(|entry| !entry.meta.path().starts_with(&path_str));
		self.dirs.retain(|entry| !entry.path().starts_with(&path_str));
	}

	// Removes the file in the given path.
	fn remove_file(&mut self, path: impl AsRef<std::path::Path>) {
		let path_str = metadata::normalized_path(path.as_ref());
		// TODO: Optimize this with binary search.
		self.files.retain(|entry| entry.meta.path() != path_str);
	}

	pub fn calculate_all(&mut self) -> io::Result<()> {
		let mut buf = Vec::with_capacity(BUF_SIZE);
		for metadata in &mut self.files {
			metadata.checksum.calculate(metadata.meta.path(), &mut buf)?;
		}
		self.dirty = true;
		Ok(())
	}

	fn normalize(&mut self) {
		self.files.sort_by(|a, b| a.meta.path().cmp(b.meta.path()));
		self.dirs.sort_by(|a, b| a.path().cmp(b.path()));
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

	pub fn calculate_matches(
		&mut self,
		mut notifier: impl FnMut(&str),
		filter: Option<&Regex>,
		match_name: bool,
		match_created: bool,
		match_modified: bool,
	) -> io::Result<()> {
		let mut file_index_by_size = HashMap::<u64, Vec<usize>>::new();
		for (file_index, file) in self.files.iter().enumerate() {
			if let Some(filter) = filter {
				if !filter.is_match(file.meta.path()) {
					continue;
				}
			}
			file_index_by_size.entry(file.size).or_default().push(file_index);
		}

		let mut file_matched = vec![false; self.files.len()];
		let mut buf = Vec::with_capacity(BUF_SIZE);
		for path_list in file_index_by_size.values() {
			if path_list.len() < 2 {
				continue;
			}
			let mut name_by_count = HashMap::<String, usize>::new();
			let mut created_by_count = HashMap::<SystemTime, usize>::new();
			let mut modified_by_count = HashMap::<SystemTime, usize>::new();
			for file_index in path_list {
				let file = &self.files[*file_index];
				if match_name {
					name_by_count
						.entry(file.meta.name().to_string())
						.and_modify(|count| *count += 1)
						.or_insert(1);
				}
				if match_created {
					created_by_count
						.entry(file.meta.created_time())
						.and_modify(|count| *count += 1)
						.or_insert(1);
				}
				if match_modified {
					modified_by_count
						.entry(file.meta.modified_time())
						.and_modify(|count| *count += 1)
						.or_insert(1);
				}
			}

			for file_index in path_list {
				let file = &self.files[*file_index];
				if match_name {
					if let Some(count) = name_by_count.get(file.meta.name()) {
						if *count < 2 {
							continue;
						}
					}
				}
				if match_created {
					if let Some(count) = created_by_count.get(&file.meta.modified_time()) {
						if *count < 2 {
							continue;
						}
					}
				}
				if match_modified {
					if let Some(count) = modified_by_count.get(&file.meta.modified_time()) {
						if *count < 2 {
							continue;
						}
					}
				}

				file_matched[*file_index] = true;
			}
		}

		for (file_index, matched) in file_matched.iter().enumerate() {
			let file = &mut self.files[file_index];
			notifier(file.meta.path());
			if !matched {
				continue;
			}

			if file.checksum.is_empty() {
				file.checksum.calculate(file.meta.path(), &mut buf)?;
				self.dirty = true;
			}
		}

		Ok(())
	}

	pub fn duplicates(&self, filter: Option<&Regex>) -> Vec<Vec<String>> {
		let mut path_by_checksum = HashMap::<(Checksum, u64), Vec<String>>::new();
		for file in &self.files {
			if !file.checksum.is_empty() {
				if let Some(filter) = filter {
					if !filter.is_match(file.meta.path()) {
						continue;
					}
				}
				path_by_checksum
					.entry((file.checksum.clone(), file.size))
					.or_default()
					.push(file.meta.path().to_string());
			}
		}

		let mut matches = Vec::new();
		for (_, path_list) in path_by_checksum {
			if path_list.len() > 1 {
				matches.push(path_list);
			}
		}
		matches
	}

	pub fn diff(
		&mut self,
		other: &mut Index,
		mut notifier: impl FnMut(&str, &str),
	) -> io::Result<Vec<Diff>> {
		let mut buf = Vec::with_capacity(BUF_SIZE);
		let mut diff_list = Vec::new();
		let mut file_index_self = 0;
		let mut file_index_other = 0;
		loop {
			if file_index_self == self.files.len() {
				for file in &other.files[file_index_other..] {
					diff_list.push(Diff::Removed(file.meta.path().to_string()));
				}
				break;
			}
			if file_index_other == other.files.len() {
				for file in &self.files[file_index_self..] {
					diff_list.push(Diff::Added(file.meta.path().to_string()));
				}
				break;
			}

			let file_self = &mut self.files[file_index_self];
			let file_other = &mut other.files[file_index_other];
			notifier(file_self.meta.path(), file_other.meta.path());

			match file_self.meta.path().cmp(file_other.meta.path()) {
				std::cmp::Ordering::Less => {
					diff_list.push(Diff::Added(file_self.meta.path().to_string()));
					file_index_self += 1;
				}
				std::cmp::Ordering::Greater => {
					diff_list.push(Diff::Removed(file_other.meta.path().to_string()));
					file_index_other += 1;
				}
				std::cmp::Ordering::Equal => {
					if file_self.size == file_other.size {
						if file_self.checksum.is_empty() {
							file_self.checksum.calculate(file_self.meta.path(), &mut buf)?;
							self.dirty = true;
						}
						if file_other.checksum.is_empty() {
							file_other.checksum.calculate(file_self.meta.path(), &mut buf)?;
							other.dirty = true;
						}

						if file_self.checksum != file_other.checksum {
							diff_list.push(Diff::Changed(file_self.meta.path().to_string()));
						}
					} else {
						diff_list.push(Diff::Changed(file_self.meta.path().to_string()));
					}
					file_index_self += 1;
					file_index_other += 1;
				}
			}
		}
		Ok(diff_list)
	}

	pub fn dirty(&self) -> bool {
		self.dirty
	}
}

pub enum Diff {
	Added(String),
	Removed(String),
	Changed(String),
}
