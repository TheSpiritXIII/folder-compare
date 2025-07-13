use std::time::SystemTime;

use crate::store::checksum::Checksum;
use crate::store::entry;
use crate::store::metadata::normalized_path;
use crate::store::metadata::Metadata;
use crate::store::RootIndex;

// An filesystem index storing only paths, without access to metadata or file contents. Primarily
// used for testing.
pub struct PathIndexBuilder {
	pub(super) files: Vec<entry::File>,
	pub(super) dirs: Vec<entry::Dir>,
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
		for dir in path.rsplit('/').skip(1) {
			self.add_dir(dir);
		}
		let file = entry::File {
			meta: Metadata {
				path,
				created_time: SystemTime::UNIX_EPOCH,
				modified_time: SystemTime::UNIX_EPOCH,
				hidden: false,
			},
			size: 0,
			checksum: Checksum::new(),
		};
		self.files.push(file);
	}

	// Adds the given directory, adding the necessary sub-directories. If the directory was already
	// added, this will do nothing.
	pub fn add_dir(&mut self, path: impl AsRef<std::path::Path>) {
		let path = normalized_path(path);
		if self.dir_exists(&path) {
			return;
		}
		for dir in path.rsplit('/').skip(1) {
			self.add_dir(dir);
		}
		let dir = entry::Dir {
			meta: Metadata {
				path,
				created_time: SystemTime::UNIX_EPOCH,
				modified_time: SystemTime::UNIX_EPOCH,
				hidden: false,
			},
		};
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
