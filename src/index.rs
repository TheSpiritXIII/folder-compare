use std::fs;
use std::fs::File;
use std::io::Read;
use std::io::{self};

use crate::progress::ProgressCounter;

struct Metadata {
	path: std::path::PathBuf,
}

impl Metadata {
	fn new(path: impl AsRef<std::path::Path>) -> Self {
		Metadata {
			path: path.as_ref().into(),
		}
	}
}

enum Entry {
	File,
	Dir(Dir),
}

impl Entry {
	fn is_file(&self) -> bool {
		matches!(self, Self::File)
	}
}

struct Dir {
	entry_handles: Option<Vec<usize>>,
}

impl Dir {
	fn new() -> Self {
		Self {
			entry_handles: None,
		}
	}
}

type Item = (Metadata, Entry);

pub struct Index {
	entries: Vec<Item>,
}

impl Index {
	pub fn with(path: impl AsRef<std::path::Path>) -> io::Result<Self> {
		let p = path.as_ref().canonicalize()?;
		let metadata = Metadata::new(&p);
		if !p.is_dir() {
			// TODO: https://github.com/rust-lang/rust/pull/128316 use ErrorKind::NotADirectory.
			return Err(io::Error::new(io::ErrorKind::Other, "not a directory"));
		}
		assert!(p.is_dir(), "only works on dirs for now");
		Ok(Self {
			entries: vec![(metadata, Entry::Dir(Dir::new()))],
		})
	}

	pub fn expand_all<T: ProgressCounter>(&mut self, progress: &T) {
		let mut queue = vec![0];
		while let Some(entry_index) = queue.pop() {
			let (metadata, _) = &self.entries[entry_index];
			let paths = fs::read_dir(&metadata.path).unwrap();
			let mut handles = Vec::new();
			for path in paths {
				let dir_entry = path.unwrap();
				let fs_metadata = dir_entry.metadata().unwrap();
				let metadata = Metadata {
					path: dir_entry.path(),
				};
				if fs_metadata.is_file() {
					handles.push(self.entries.len());
					self.entries.push((metadata, Entry::File));
				} else if fs_metadata.is_dir() {
					handles.push(self.entries.len());
					queue.push(self.entries.len());
					self.entries.push((metadata, Entry::Dir(Dir::new())));
				}
			}
			let Entry::Dir(dir) = &mut self.entries[entry_index].1 else {
				unreachable!()
			};
			dir.entry_handles = Some(handles);
			progress.update(self.entries.len());
		}
	}

	pub fn entry_count(&self) -> usize {
		self.entries.len() + 1
	}

	pub fn file_count(&self) -> usize {
		self.entries.iter().filter(|(_, entry)| entry.is_file()).count()
	}

	pub fn diff<T: ProgressCounter>(&self, other: &Index, progress: &T) -> Vec<Diff> {
		// Size of buffer to compare files, optimized for an 8 KiB average file-size.
		// Dinneen, Jesse & Nguyen, Ba. (2021). How Big Are Peoples' Computer Files? File Size
		// Distributions Among User-managed Collections.
		const BUF_SIZE: usize = 1024 * 8;

		let root_path = self.root_path();
		let root_path_other = other.root_path();
		let mut diff_list = Vec::new();
		let mut processed_list = vec![false; other.entries.len()];

		// Ensure we use heap memory so we don't overflow the stack.
		let mut buf_self = vec![0; BUF_SIZE];
		let mut buf_other = vec![0; BUF_SIZE];

		for (index, (metadata, entry)) in self.entries.iter().enumerate() {
			progress.update(index);
			let relative_name = metadata.path.strip_prefix(root_path).unwrap();
			let other_index = other.find_by_name(relative_name);
			if let Some(index) = other_index {
				processed_list[index] = true;
				if !entry.is_file() {
					if other.entries[index].1.is_file() {
						let name = relative_name.to_string_lossy().into_owned();
						diff_list.push(Diff::Changed(name));
					}
					continue;
				}
				if !self.contents_same(other, relative_name, &mut buf_self, &mut buf_other).unwrap()
				{
					let name = relative_name.to_string_lossy().into_owned();
					diff_list.push(Diff::Changed(name));
				}
			} else {
				let name = relative_name.to_string_lossy().into_owned();
				diff_list.push(Diff::Added(name));
			}
		}
		for (processed_index, processed) in processed_list.iter().enumerate() {
			progress.update(self.entry_count() + processed_index);
			if !processed {
				let name = other.entries[processed_index]
					.0
					.path
					.strip_prefix(root_path_other)
					.unwrap()
					.to_string_lossy()
					.into_owned();
				diff_list.push(Diff::Removed(name));
			}
		}
		diff_list
	}

	fn find_by_name(&self, name: impl AsRef<std::path::Path>) -> Option<usize> {
		let root_path = self.root_path();
		for (index, (metadata, _)) in self.entries.iter().enumerate() {
			if metadata.path.strip_prefix(root_path).unwrap() == name.as_ref() {
				return Some(index);
			}
		}
		None
	}

	fn root_path(&self) -> &std::path::Path {
		self.entries[0].0.path.parent().unwrap()
	}

	fn contents_same(
		&self,
		other: &Index,
		path: &std::path::Path,
		buf1: &mut [u8],
		buf2: &mut [u8],
	) -> io::Result<bool> {
		debug_assert!(buf1.len() == buf2.len());
		let root_path_self = self.root_path();
		let root_path_other = other.root_path();
		let mut file_self = File::open(root_path_self.join(path))?;
		let mut file_other = File::open(root_path_other.join(path))?;
		loop {
			let amount_self = file_self.read(buf1)?;
			let amount_other = file_other.read(buf2)?;
			if amount_self != amount_other {
				return Ok(false);
			}
			if amount_self == 0 {
				return Ok(true);
			}
			if buf1 != buf2 {
				return Ok(false);
			}
		}
	}
}

pub enum Diff {
	Added(String),
	Removed(String),
	Changed(String),
}
