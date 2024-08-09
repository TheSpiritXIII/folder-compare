use std::fs;
use std::io;
use std::sync::atomic;

struct Metadata {
	path: std::path::PathBuf,
}

impl Metadata {
	fn new(path: impl AsRef<std::path::Path>) -> Metadata {
		return Metadata {
			path: path.as_ref().into(),
		};
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

	pub fn diff(&self, other: &Index) -> Vec<Diff> {
		let root_path = self.root_path();
		let root_path_other = other.root_path();
		let mut diff_list = Vec::new();
		let mut processed_list = vec![false; other.entries.len()];
		for (metadata, entry) in &self.entries {
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
				if !self.contents_same(other, relative_name).unwrap() {
					let name = relative_name.to_string_lossy().into_owned();
					diff_list.push(Diff::Changed(name));
				}
			} else {
				let name = relative_name.to_string_lossy().into_owned();
				diff_list.push(Diff::Added(name));
			}
		}
		for (processed_index, processed) in processed_list.iter().enumerate() {
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

	fn contents_same(&self, other: &Index, path: &std::path::Path) -> io::Result<bool> {
		let root_path_self = self.root_path();
		let root_path_other = other.root_path();
		let contents_self = fs::read(root_path_self.join(path))?;
		let contents_other = fs::read(root_path_other.join(path))?;
		Ok(contents_self == contents_other)
	}
}

pub enum Diff {
	Added(String),
	Removed(String),
	Changed(String),
}

pub trait ProgressCounter {
	fn update(&self, count: usize);
}

pub struct NopProgressCounter;

impl ProgressCounter for NopProgressCounter {
	fn update(&self, _count: usize) {}
}

pub struct AtomicProgressCounter {
	counter: atomic::AtomicUsize,
}

impl AtomicProgressCounter {
	pub fn new() -> Self {
		Self {
			counter: atomic::AtomicUsize::new(0),
		}
	}

	pub fn value(&self) -> usize {
		self.counter.load(atomic::Ordering::Relaxed)
	}
}

impl ProgressCounter for AtomicProgressCounter {
	fn update(&self, count: usize) {
		self.counter.store(count, atomic::Ordering::Release);
	}
}
