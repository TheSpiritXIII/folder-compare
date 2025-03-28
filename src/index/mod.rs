mod checksum;
mod entry;
mod metadata;
#[cfg(test)]
mod test;

use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::fs::{self};
use std::io::{self};
use std::path::Path;
use std::time::SystemTime;

use checksum::Checksum;
use metadata::normalized_path;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;

// Size of buffer to compare files, optimized for an 8 KiB average file-size.
// Dinneen, Jesse & Nguyen, Ba. (2021). How Big Are Peoples' Computer Files? File Size Distributions
// Among User-managed Collections.
const BUF_SIZE: usize = 1024 * 8;

fn file_size(files: &[entry::File]) -> u128 {
	files.iter().map(|entry| entry.size).map(u128::from).sum()
}

#[derive(PartialEq, Eq, Hash)]
pub struct DirStats {
	pub file_count: usize,
	pub file_size: u128,
	pub dir_count: usize,
}

#[derive(Serialize, Deserialize)]
pub struct Index {
	pub(crate) files: Vec<entry::File>,
	pub(crate) dirs: Vec<entry::Dir>,

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
	fn remove_dir(&mut self, path: impl AsRef<std::path::Path>) -> Option<Vec<entry::File>> {
		if let Some((start, end)) = self.find_dirs(&path) {
			self.dirs.drain(start..end);
			let (start, end) = self.find_dir_files(&path);
			return Some(self.files.drain(start..end).collect());
		}
		None
	}

	// Removes the file in the given path.
	fn remove_file(&mut self, path: impl AsRef<std::path::Path>) -> Option<entry::File> {
		if let Some(index) = self.find_file(path) {
			return Some(self.files.remove(index));
		}
		None
	}

	fn find_dirs(&mut self, path: impl AsRef<std::path::Path>) -> Option<(usize, usize)> {
		let mut p = normalized_path(path);
		if p.is_empty() {
			return Some((0, self.dirs.len()));
		}
		let start = self.dirs.binary_search_by(|entry| entry.meta.path().cmp(&p)).ok()?;
		let mut end = start + 1;
		p.push('/');
		for entry in &self.dirs[end..] {
			if !entry.meta.path().starts_with(&p) {
				break;
			}
			end += 1;
		}
		Some((start, end))
	}

	fn find_dir_children(&self, path: impl AsRef<std::path::Path>) -> Option<(usize, usize)> {
		let p = normalized_path(path);
		if p.is_empty() {
			return Some((0, self.dirs.len()));
		}

		let start = self.dirs.binary_search_by(|entry| entry.meta.path().cmp(&p)).ok()? + 1;
		let mut end = start;
		for entry in &self.dirs[end..] {
			if !entry.meta.path().starts_with(&p) {
				break;
			}
			end += 1;
		}
		Some((start, end))
	}

	fn find_file(&mut self, path: impl AsRef<std::path::Path>) -> Option<usize> {
		let p = normalized_path(path);
		self.files.binary_search_by(|entry| entry.meta.path().cmp(&p)).ok()
	}

	fn find_dir_files(&self, path: impl AsRef<std::path::Path>) -> (usize, usize) {
		let mut p = normalized_path(path);
		if p.is_empty() {
			return (0, self.files.len());
		}
		p.push('/');

		let start = match self.files.binary_search_by(|entry| entry.meta.path().cmp(&p)) {
			Ok(index) | Err(index) => index,
		};
		let mut end = start;
		for entry in &self.files[start..] {
			if !entry.meta.path().starts_with(&p) {
				break;
			}
			end += 1;
		}
		(start, end)
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
		mut notifier: impl FnMut(&str),
		allowlist: Option<&Regex>,
		denylist: Option<&Regex>,
		match_name: bool,
		match_created: bool,
		match_modified: bool,
	) -> io::Result<()> {
		let mut file_index_by_size = HashMap::<u64, Vec<usize>>::new();
		for (file_index, file) in self.files.iter().enumerate() {
			if let Some(filter) = allowlist {
				if !filter.is_match(file.meta.path()) {
					continue;
				}
			}
			if let Some(filter) = denylist {
				if filter.is_match(file.meta.path()) {
					continue;
				}
			}
			file_index_by_size.entry(file.size).or_default().push(file_index);
		}

		let mut file_matched = vec![false; self.files.len()];
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

		let mut buf = Vec::with_capacity(BUF_SIZE);
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

	pub fn duplicates(
		&self,
		allowlist: Option<&Regex>,
		denylist: Option<&Regex>,
	) -> Vec<Vec<String>> {
		let mut path_by_checksum = HashMap::<(Checksum, u64), Vec<String>>::new();
		for file in &self.files {
			if !file.checksum.is_empty() {
				if let Some(filter) = allowlist {
					if !filter.is_match(file.meta.path()) {
						continue;
					}
				}
				if let Some(filter) = denylist {
					if filter.is_match(file.meta.path()) {
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

	pub fn stats(&self) -> DirStats {
		DirStats {
			file_count: self.files.len(),
			file_size: file_size(&self.files),
			dir_count: self.dirs.len(),
		}
	}

	pub fn dir_stats(&self, dir: impl AsRef<Path>) -> DirStats {
		let (start, end) = self.find_dir_files(&dir);
		let file_size = file_size(&self.files[start..end]);

		let dir_count = if let Some((start, end)) = self.find_dir_children(&dir) {
			end - start
		} else {
			0
		};

		DirStats {
			file_count: end - start,
			file_size,
			dir_count,
		}
	}

	#[allow(clippy::too_many_lines)]
	pub fn calculate_dir_matches(
		&mut self,
		mut notifier: impl FnMut(&str),
		allowlist: Option<&Regex>,
		denylist: Option<&Regex>,
		match_name: bool,
		match_created: bool,
		match_modified: bool,
	) -> io::Result<()> {
		let mut dirs_by_stats = HashMap::<DirStats, Vec<usize>>::new();
		for (dir_index, dir) in self.dirs.iter().enumerate() {
			let stats = self.dir_stats(dir.meta.path());
			if stats.dir_count == 0 && stats.file_count == 0 {
				continue;
			}
			if let Some(filter) = allowlist {
				if !filter.is_match(dir.meta.path()) {
					continue;
				}
			}
			if let Some(filter) = denylist {
				if filter.is_match(dir.meta.path()) {
					continue;
				}
			}
			dirs_by_stats.entry(stats).or_default().push(dir_index);
		}

		let mut file_matched = vec![false; self.files.len()];
		for (_, path_list) in dirs_by_stats {
			if path_list.len() < 2 {
				continue;
			}
			let mut name_by_count = HashMap::<Vec<String>, usize>::new();
			let mut created_by_count = HashMap::<Vec<SystemTime>, usize>::new();
			let mut modified_by_count = HashMap::<Vec<SystemTime>, usize>::new();
			let mut name_list = vec![Vec::new(); path_list.len()];
			let mut created_list = vec![Vec::new(); path_list.len()];
			let mut modified_list = vec![Vec::new(); path_list.len()];
			for dir_index in &path_list {
				let dir = &self.dirs[*dir_index];
				let (start, end) = self.find_dir_files(dir.meta.path());
				let file_list = &self.files[start..end];
				if match_name {
					name_list[*dir_index] =
						file_list.iter().map(|entry| entry.meta.path().to_string()).collect();
					name_list[*dir_index].sort();
					name_by_count
						.entry(name_list[*dir_index].clone())
						.and_modify(|count| *count += 1)
						.or_insert(1);
				}
				if match_created {
					created_list[*dir_index] =
						file_list.iter().map(|entry| entry.meta.created_time).collect();
					created_list[*dir_index].sort();
					created_by_count
						.entry(created_list[*dir_index].clone())
						.and_modify(|count| *count += 1)
						.or_insert(1);
				}
				if match_modified {
					modified_list[*dir_index] =
						file_list.iter().map(|entry| entry.meta.created_time).collect();
					modified_list[*dir_index].sort();
					modified_by_count
						.entry(modified_list[*dir_index].clone())
						.and_modify(|count| *count += 1)
						.or_insert(1);
				}
			}

			for dir_index in &path_list {
				let dir = &self.dirs[*dir_index];
				if match_name {
					if let Some(count) = name_by_count.get(&name_list[*dir_index]) {
						if *count < 2 {
							continue;
						}
					}
				}
				if match_created {
					if let Some(count) = created_by_count.get(&created_list[*dir_index]) {
						if *count < 2 {
							continue;
						}
					}
				}
				if match_modified {
					if let Some(count) = modified_by_count.get(&modified_list[*dir_index]) {
						if *count < 2 {
							continue;
						}
					}
				}

				let (start, end) = self.find_dir_files(dir.meta.path());
				for matched in file_matched.iter_mut().take(end).skip(start) {
					*matched = true;
				}
			}
		}

		let mut buf = Vec::with_capacity(BUF_SIZE);
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

	pub fn duplicate_dirs(
		&mut self,
		allowlist: Option<&Regex>,
		denylist: Option<&Regex>,
	) -> Vec<Vec<String>> {
		let mut dirs_by_checksums = HashMap::<(usize, Vec<Checksum>), Vec<String>>::new();
		for dir in &self.dirs {
			if let Some(filter) = allowlist {
				if !filter.is_match(dir.meta.path()) {
					continue;
				}
			}
			if let Some(filter) = denylist {
				if filter.is_match(dir.meta.path()) {
					continue;
				}
			}
			let (start, end) = self.find_dir_files(dir.meta.path());
			let file_list = &self.files[start..end];
			let mut file_checksums = Vec::with_capacity(end - start);
			for file in file_list {
				file_checksums.push(file.checksum.clone());
			}
			file_checksums.sort();

			let children = if let Some((start, end)) = self.find_dir_children(dir.meta.path()) {
				end - start
			} else {
				0
			};
			dirs_by_checksums
				.entry((children, file_checksums))
				.or_default()
				.push(dir.meta.path().to_string());
		}

		let mut matches = Vec::new();
		for (_, path_list) in dirs_by_checksums {
			if path_list.len() > 1 {
				matches.push(path_list);
			}
		}
		matches
	}

	#[allow(clippy::too_many_lines)]
	pub fn diff(
		&mut self,
		other: &mut Index,
		mut notifier: impl FnMut(&str, &str),
		match_name: bool,
		match_created: bool,
		match_modified: bool,
	) -> io::Result<Vec<Diff>> {
		let mut buf = Vec::with_capacity(BUF_SIZE);
		let mut diff_list = Vec::new();
		let mut file_index_self = 0;
		let mut file_index_other = 0;

		let mut file_index_self_by_checksum = HashMap::<(Checksum, u64), Vec<usize>>::new();
		let mut file_index_other_by_checksum = HashMap::<(Checksum, u64), Vec<usize>>::new();
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
					if file_self.checksum.is_empty() {
						diff_list.push(Diff::Added(file_self.meta.path().to_string()));
					} else {
						file_index_self_by_checksum
							.entry((file_self.checksum.clone(), file_self.size))
							.or_default()
							.push(file_index_self);
					}
					file_index_self += 1;
				}
				std::cmp::Ordering::Greater => {
					if file_other.checksum.is_empty() {
						diff_list.push(Diff::Removed(file_other.meta.path().to_string()));
					} else {
						file_index_other_by_checksum
							.entry((file_other.checksum.clone(), file_other.size))
							.or_default()
							.push(file_index_other);
					}
					file_index_other += 1;
				}
				std::cmp::Ordering::Equal => {
					file_index_self += 1;
					file_index_other += 1;

					if file_self.size != file_other.size {
						diff_list.push(Diff::Changed(file_self.meta.path().to_string()));
						continue;
					}

					if match_name {
						continue;
					}
					if match_created
						&& file_self.meta.created_time() == file_other.meta.created_time()
					{
						continue;
					}
					if match_modified
						&& file_self.meta.modified_time() == file_other.meta.modified_time()
					{
						continue;
					}

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
				}
			}
		}

		for (checksum, path_list_self) in file_index_self_by_checksum {
			if let Some(path_list_other) = file_index_other_by_checksum.remove(&checksum) {
				if path_list_self.len() == path_list_other.len() {
					for (file_index_self, file_index_other) in
						path_list_self.iter().zip(path_list_other)
					{
						let file_self = &mut self.files[*file_index_self];
						let file_other = &mut other.files[file_index_other];

						diff_list.push(Diff::Moved(
							file_self.meta.path().to_string(),
							file_other.meta.path().to_string(),
						));
					}
					continue;
				}

				for file_index in path_list_self {
					let file = &mut self.files[file_index];
					diff_list.push(Diff::Added(file.meta.path().to_string()));
				}
				for file_index in path_list_other {
					let file = &mut other.files[file_index];
					diff_list.push(Diff::Removed(file.meta.path().to_string()));
				}
				continue;
			}

			for file_index in path_list_self {
				let file = &mut self.files[file_index];
				diff_list.push(Diff::Added(file.meta.path().to_string()));
			}
		}
		for (_, path_list) in file_index_other_by_checksum {
			for file_index in path_list {
				let file = &mut other.files[file_index];
				diff_list.push(Diff::Removed(file.meta.path().to_string()));
			}
		}

		Ok(diff_list)
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

pub enum Diff {
	Added(String),
	Removed(String),
	Changed(String),
	Moved(String, String),
}
