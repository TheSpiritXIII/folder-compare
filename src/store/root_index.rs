use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::fs::{self};
use std::io::{self};
use std::path::Path;

use regex::Regex;
use serde::Deserialize;
use serde::Serialize;

use super::calculator::calculate_dir_matches;
use super::calculator::calculate_matches;
use super::calculator::diff;
use super::calculator::duplicate_dirs;
use super::calculator::duplicates;
use super::checksum::NativeFileReader;
use super::entry;
use super::metadata::normalized_path;
use super::sub_index::SubIndex;
use super::Diff;

#[derive(Serialize, Deserialize)]
pub struct RootIndex {
	pub(super) files: Vec<entry::File>,
	pub(super) dirs: Vec<entry::Dir>,

	#[serde(skip_serializing)]
	#[serde(skip_deserializing)]
	dirty: bool,
}

impl RootIndex {
	pub fn new() -> Self {
		Self {
			files: Vec::new(),
			dirs: Vec::new(),
			dirty: false,
		}
	}

	// Recursively finds all files in the given directory and adds them to the index.
	pub fn from_path(
		path: impl AsRef<std::path::Path>,
		notifier: impl FnMut(&str),
	) -> io::Result<Self> {
		let mut index = Self::new();
		if path.as_ref().is_dir() {
			index.add_dir(path.as_ref(), notifier)?;
			index.normalize();
			return Ok(index);
		} else if path.as_ref().is_file() {
			index.add_file(path)?;
			return Ok(index);
		}
		// TODO: io::Result doesn't make sense for this.
		Err(io::Error::from(io::ErrorKind::Unsupported))
	}

	pub fn add(
		&mut self,
		path: impl AsRef<std::path::Path>,
		mut notifier: impl FnMut(&str),
	) -> io::Result<()> {
		if path.as_ref().is_dir() {
			self.dirty = true;
			let removed = self.remove_dir(path.as_ref());
			let added = self.add_dir(path, notifier)?;
			if let Some(entry_list) = removed {
				let mut meta_map = HashMap::new();
				for entry in entry_list {
					meta_map.insert(entry.meta, entry.checksum);
				}
				for entry in added {
					if let Some(checksum) = meta_map.get(&entry.meta) {
						entry.checksum = checksum.clone();
					}
				}
			}
		} else if path.as_ref().is_file() {
			self.dirty = true;
			let removed = self.remove_file(path.as_ref());
			let added = self.add_file(path)?;
			if let Some(entry) = removed {
				if entry.meta == added.meta {
					added.checksum = entry.checksum;
				}
			}
			notifier(added.meta.path());
		} else {
			// TODO: io::Result doesn't make sense for this.
			return Err(io::Error::from(io::ErrorKind::Unsupported));
		}
		self.normalize();
		Ok(())
	}

	fn add_dir(
		&mut self,
		path: impl AsRef<std::path::Path>,
		mut notifier: impl FnMut(&str),
	) -> io::Result<&mut [entry::File]> {
		let mut queue = VecDeque::new();
		queue.push_back(path.as_ref().to_path_buf());

		let mut is_root = path.as_ref().parent().is_none();
		let start_index = self.files.len();

		while let Some(current_path) = queue.pop_front() {
			self.dirs.push(entry::Dir::from_path(&current_path)?);
			notifier(self.dirs.last().unwrap().meta.path());

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
					let entry = self.add_file(path)?;
					notifier(entry.meta.path());
				}
			}
			is_root = false;
		}

		Ok(&mut self.files[start_index..])
	}

	fn add_file(&mut self, path: impl AsRef<std::path::Path>) -> io::Result<&mut entry::File> {
		self.files.push(entry::File::from_path(path)?);
		Ok(self.files.last_mut().unwrap())
	}

	// Removes the directory in the given path.
	pub(super) fn remove_dir(
		&mut self,
		path: impl AsRef<std::path::Path>,
	) -> Option<Vec<entry::File>> {
		let p = normalized_path(path);
		if p.is_empty() {
			self.dirs.clear();
			return Some(self.files.drain(0..self.files.len()).collect());
		}
		let start = self.all().dir_index(&p)?;
		let (_, end) = self.all().dir_children_indices(start);
		self.dirs.drain(start..end);

		let (start, end) = self.all().dir_file_indices(&p);
		Some(self.files.drain(start..end).collect())
	}

	// Removes the file in the given path.
	pub(super) fn remove_file(&mut self, path: impl AsRef<std::path::Path>) -> Option<entry::File> {
		let p = normalized_path(path);
		if let Some(index) = self.all().file_index(&p) {
			return Some(self.files.remove(index));
		}
		None
	}

	pub fn calculate_all(&mut self) -> io::Result<()> {
		let mut buf = Vec::with_capacity(super::BUF_SIZE);
		for metadata in &mut self.files {
			metadata.checksum.calculate(&NativeFileReader, metadata.meta.path(), &mut buf)?;
		}
		self.dirty = true;
		Ok(())
	}

	pub(super) fn normalize(&mut self) {
		self.files.sort_by(|a, b| a.meta.path().cmp(b.meta.path()));
		self.dirs.sort_by(|a, b| a.meta.path().cmp(b.meta.path()));
		debug_assert!(self.validate());
	}

	// TODO: Validate file extension?
	// Stores the index entries as RON on the filesystem.
	pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
		debug_assert!(self.validate());
		let json = ron::ser::to_string_pretty(&self, ron::ser::PrettyConfig::default()).unwrap();
		fs::write(path, json)?;
		Ok(())
	}

	// TODO: Versioning?
	// Opens an Index from a RON file.
	pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
		let json = fs::read_to_string(path)?;
		let index: Self = ron::from_str(&json).unwrap();
		// TODO: Assert in release mode.
		debug_assert!(index.validate());
		Ok(index)
	}

	pub fn entry_count(&self) -> usize {
		self.files.len() + self.dirs.len()
	}

	pub fn file_count(&self) -> usize {
		self.files.len()
	}

	pub fn calculate_matches(
		&mut self,
		notifier: impl FnMut(&str),
		allowlist: Option<&Regex>,
		denylist: Option<&Regex>,
		match_name: bool,
		match_created: bool,
		match_modified: bool,
	) -> io::Result<()> {
		calculate_matches(
			&mut self.files,
			&mut self.dirty,
			notifier,
			allowlist,
			denylist,
			match_name,
			match_created,
			match_modified,
		)
	}

	pub fn duplicates(
		&self,
		allowlist: Option<&Regex>,
		denylist: Option<&Regex>,
	) -> Vec<Vec<String>> {
		duplicates(&self.files, allowlist, denylist)
	}

	pub fn all(&self) -> SubIndex {
		SubIndex {
			files: &self.files,
			dirs: &self.dirs,
		}
	}

	pub fn sub_index(&self, dir: impl AsRef<Path>) -> Option<SubIndex> {
		let p = normalized_path(dir);
		let dir_index = self.all().dir_index(&p)?;
		let (dir_start, dir_end) = self.all().dir_children_indices(dir_index);
		let (file_start, file_end) = self.all().dir_file_indices(&p);
		Some(SubIndex {
			files: &self.files[file_start..file_end],
			dirs: &self.dirs[dir_start..dir_end],
		})
	}

	#[allow(clippy::too_many_lines)]
	pub fn calculate_dir_matches(
		&mut self,
		notifier: impl FnMut(&str),
		allowlist: Option<&Regex>,
		denylist: Option<&Regex>,
		match_name: bool,
		match_created: bool,
		match_modified: bool,
	) -> io::Result<()> {
		calculate_dir_matches(
			&mut self.files,
			&mut self.dirs,
			&mut self.dirty,
			notifier,
			allowlist,
			denylist,
			match_name,
			match_created,
			match_modified,
		)
	}

	pub fn duplicate_dirs(
		&self,
		allowlist: Option<&Regex>,
		denylist: Option<&Regex>,
	) -> Vec<Vec<String>> {
		duplicate_dirs(&self.all(), allowlist, denylist)
	}

	pub fn diff(
		&mut self,
		other: &mut RootIndex,
		notifier: impl FnMut(&str, &str),
		match_name: bool,
		match_created: bool,
		match_modified: bool,
	) -> io::Result<Vec<Diff>> {
		diff(
			&mut self.files,
			&mut self.dirty,
			&mut other.files,
			&mut other.dirty,
			notifier,
			match_name,
			match_created,
			match_modified,
		)
	}

	pub fn dirty(&self) -> bool {
		self.dirty
	}

	fn validate(&self) -> bool {
		for i in 1..self.files.len() {
			let file_before = &self.files[i - 1];
			let file_after = &self.files[i];
			let path_before = file_before.meta.path();
			let path_after = file_after.meta.path();
			if path_before.cmp(path_after) != Ordering::Less {
				println!("File order is wrong: {path_before} vs {path_after}");
				return false;
			}
		}
		for i in 1..self.dirs.len() {
			let dir_before = &self.dirs[i - 1];
			let dir_after = &self.dirs[i];
			let path_before = dir_before.meta.path();
			let path_after = dir_after.meta.path();
			if path_before.cmp(path_after) != Ordering::Less {
				println!("File order is wrong: {path_before} vs {path_after}");
				return false;
			}
		}
		// TODO: Ensure that each dir is represented. Extract from files.
		true
	}
}
