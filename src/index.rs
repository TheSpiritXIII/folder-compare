use std::fs;

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
	pub fn with(path: impl AsRef<std::path::Path>) -> Self {
		let p = std::path::absolute(path).unwrap();
		let metadata = Metadata::new(&p);
		Self {
			entries: vec![(metadata, Entry::Dir(Dir::new()))],
		}
	}

	pub fn expand_all(&mut self) {
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
					self.entries.push((metadata, Entry::File))
				} else if fs_metadata.is_dir() {
					handles.push(self.entries.len());
					queue.push(self.entries.len());
					self.entries.push((metadata, Entry::Dir(Dir::new())))
				}
			}
			let dir = if let Entry::Dir(dir) = &mut self.entries[entry_index].1 {
				dir
			} else {
				unreachable!()
			};
			dir.entry_handles = Some(handles);
		}
	}

	pub fn entry_count(&self) -> usize {
		self.entries.len() + 1
	}

	pub fn file_count(&self) -> usize {
		self.entries.iter().filter(|(_, entry)| entry.is_file()).count()
	}
}
