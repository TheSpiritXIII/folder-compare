use std::time::SystemTime;

use crate::index::model::normalized_path;
use crate::index::model::Checksum;
use crate::index::model::Dir;
use crate::index::model::File;
use crate::index::model::Metadata;
use crate::index::RootIndex;

// A filesystem index storing only paths, without access to metadata or file contents. Primarily
// used for testing.
pub struct PathIndexBuilder {
	files: Vec<File>,
	dirs: Vec<Dir>,
}

impl PathIndexBuilder {
	// Creates an empty builder.
	pub fn new() -> Self {
		Self {
			files: Vec::new(),
			dirs: Vec::new(),
		}
	}

	// Adds the given file, adding the necessary directories. If the file was already added, this
	// will do nothing.
	pub fn add_file(&mut self, path: impl AsRef<std::path::Path>) {
		let path = normalized_path(path);
		if self.file_exists(&path) {
			return;
		}

		let file = File {
			meta: Metadata {
				path,
				created_time: SystemTime::UNIX_EPOCH,
				modified_time: SystemTime::UNIX_EPOCH,
				hidden: false,
			},
			size: 0,
			checksum: Checksum::new(),
		};
		if let Some(parent) = file.meta.parent() {
			self.add_dir(parent);
		} else {
			self.add_dir("");
		}
		self.files.push(file);
	}

	// Adds the given directory, adding the necessary sub-directories. If the directory was already
	// added, this will do nothing.
	pub fn add_dir(&mut self, path: impl AsRef<std::path::Path>) {
		let path = normalized_path(path);
		if self.dir_exists(&path) {
			return;
		}

		let dir = Dir {
			meta: Metadata {
				path,
				created_time: SystemTime::UNIX_EPOCH,
				modified_time: SystemTime::UNIX_EPOCH,
				hidden: false,
			},
		};
		if let Some(parent) = dir.meta.parent() {
			self.add_dir(parent);
		}
		self.dirs.push(dir);
	}

	// Builds the index from the collected files and directories.
	pub fn build(self) -> RootIndex {
		let mut index = RootIndex::new();
		index.files = self.files;
		index.dirs = self.dirs;
		index.normalize();
		index
	}

	fn file_exists(&self, path: &str) -> bool {
		self.files.iter().any(|f| f.meta.path == path)
	}

	fn dir_exists(&self, path: &str) -> bool {
		self.dirs.iter().any(|d| d.meta.path == path)
	}
}
