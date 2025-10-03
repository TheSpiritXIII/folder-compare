use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::fs::{self};
use std::io::{self};
use std::path::Path;

use serde::Deserialize;
use serde::Serialize;

use super::sub_index::SubIndex;
use crate::index::calculator::diff;
use crate::index::calculator::duplicate_dirs;
use crate::index::calculator::duplicates;
use crate::index::calculator::Diff;
use crate::index::model::normalized_path;
use crate::index::model::Dir;
use crate::index::model::File;
use crate::index::model::NativeFileReader;
use crate::index::store::SliceIndex;
use crate::index::store::SortedSliceIndex;
use crate::index::store::SortedSliceIndexOpts;
use crate::index::store::SubIndexMut;
use crate::index::Allowlist;
use crate::index::BUF_SIZE;

#[derive(Serialize, Deserialize)]
pub struct RootIndex {
	// TODO: Make this private.
	pub files: Vec<File>,
	// TODO: Make this private.
	pub dirs: Vec<Dir>,

	#[serde(skip_serializing)]
	#[serde(skip_deserializing)]
	pub(super) dirty: bool,
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
			let file = File::from_path(path)?;
			index.add_file(file);
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
			let file = File::from_path(path)?;
			let added = self.add_file(file);
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
	) -> io::Result<&mut [File]> {
		let mut queue = VecDeque::new();
		queue.push_back(path.as_ref().to_path_buf());

		let start_index = self.files.len();

		// Windows: If the directory we're adding is a drive, it could incorrectly be marked as
		// hidden. Add whatever we add regardless of whether it is marked as hidden.
		let mut root = true;

		while let Some(current_path) = queue.pop_front() {
			let dir = Dir::from_path(&current_path)?;
			if !root && dir.meta.hidden {
				continue;
			}
			root = false;

			self.dirs.push(dir);
			notifier(self.dirs.last().unwrap().meta.path());

			for entry in fs::read_dir(current_path)? {
				let entry = entry?;
				let path = entry.path();

				if path.is_dir() {
					queue.push_back(path);
				} else {
					let file = File::from_path(path)?;
					if file.meta.hidden {
						println!("Skipping hidden file: {}", file.meta.path());
						continue;
					}
					let entry = self.add_file(file);
					notifier(entry.meta.path());
				}
			}
		}

		Ok(&mut self.files[start_index..])
	}

	fn add_file(&mut self, file: File) -> &mut File {
		self.files.push(file);
		self.files.last_mut().unwrap()
	}

	// Removes the directory in the given path.
	pub(super) fn remove_dir(&mut self, path: impl AsRef<std::path::Path>) -> Option<Vec<File>> {
		let p = normalized_path(path);
		if p.is_empty() {
			self.dirs.clear();
			return Some(self.files.drain(0..self.files.len()).collect());
		}
		let start = self.dir_index(&p)?;
		let (_, end) = self.dir_children_indices(start);
		self.dirs.drain(start..end);

		let (start, end) = self.dir_file_indices(&p);
		Some(self.files.drain(start..end).collect())
	}

	// Removes the file in the given path.
	pub(super) fn remove_file(&mut self, path: impl AsRef<std::path::Path>) -> Option<File> {
		let p = normalized_path(path);
		if let Some(index) = self.file_index(&p) {
			return Some(self.files.remove(index));
		}
		None
	}

	pub fn calculate_all(&mut self) -> io::Result<()> {
		let mut buf = Vec::with_capacity(BUF_SIZE);
		for metadata in &mut self.files {
			metadata.checksum.calculate(&NativeFileReader, metadata.meta.path(), &mut buf)?;
		}
		self.dirty = true;
		Ok(())
	}

	// TODO: Make this private.
	pub fn normalize(&mut self) {
		self.files.sort_by(|a, b| a.meta.path().cmp(b.meta.path()));
		self.dirs.sort_by(|a, b| a.meta.path().cmp(b.meta.path()));
		debug_assert!(self.validate());
	}

	// TODO: Validate file extension?
	// Stores the index entries as RON on the filesystem.
	pub fn save(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
		debug_assert!(self.validate());
		let json = ron::ser::to_string_pretty(&self, ron::ser::PrettyConfig::default()).unwrap();
		fs::write(path, json)?;
		self.dirty = false;
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

	// TODO: Can we make this conversion automatic?
	pub fn all_mut(&mut self) -> SubIndexMut<'_> {
		let file_len = self.files.len();
		let dir_len = self.dirs.len();

		SubIndexMut {
			root: self,
			file_start: 0,
			file_end: file_len,
			dir_start: 0,
			dir_end: dir_len,
		}
	}

	pub fn duplicates(&self, allowlist: &Allowlist) -> Vec<Vec<String>> {
		duplicates(&self.files, allowlist)
	}

	pub fn all(&self) -> SubIndex<'_> {
		SubIndex {
			files: &self.files,
			dirs: &self.dirs,
		}
	}

	pub fn sub_index(&self, dir: impl AsRef<Path>) -> Option<SubIndex<'_>> {
		let p = normalized_path(dir);
		let dir_index = self.dir_index(&p)?;
		let (dir_start, dir_end) = self.dir_children_indices(dir_index);
		let (file_start, file_end) = self.dir_file_indices(&p);
		println!(
			"start {} end {}, real {}",
			file_start,
			file_end,
			self.files[file_start].meta.path()
		);
		Some(SubIndex {
			files: &self.files[file_start..file_end],
			dirs: &self.dirs[dir_start..dir_end],
		})
	}

	pub fn duplicate_dirs(&self, allowlist: &Allowlist) -> Vec<Vec<String>> {
		duplicate_dirs(&self.all(), allowlist)
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

impl SliceIndex for RootIndex {
	fn files(&self) -> &[File] {
		&self.files
	}

	fn dirs(&self) -> &[Dir] {
		&self.dirs
	}
}

impl SortedSliceIndex for RootIndex {}
