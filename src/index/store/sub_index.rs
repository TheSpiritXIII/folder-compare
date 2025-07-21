use crate::index::model::Dir;
use crate::index::model::File;
use crate::index::store::root_index::SliceIndex;

/// Signifies a directory and its contents from an index.
pub struct SubIndex<'a> {
	// TODO: Make this private.
	pub files: &'a [File],
	// TODO: Make this private.
	pub dirs: &'a [Dir],
}

impl SubIndex<'_> {
	pub(super) fn dir_index(&self, path: &str) -> Option<usize> {
		if path.is_empty() {
			return None;
		}
		self.dirs.binary_search_by(|entry| entry.meta.path().cmp(path)).ok()
	}

	pub(super) fn dir_children_indices(&self, dir_index: usize) -> (usize, usize) {
		let dir = &self.dirs[dir_index];
		// Don't include itself in the sub-index.
		let dir_start = dir_index + 1;
		let mut dir_end = dir_start;
		for entry in &self.dirs[dir_start..] {
			if !entry.meta.is_child_of(dir.meta.path()) {
				break;
			}
			dir_end += 1;
		}
		(dir_start, dir_end)
	}

	pub(super) fn file_index(&self, p: &str) -> Option<usize> {
		self.files.binary_search_by(|entry| entry.meta.path().cmp(p)).ok()
	}

	// TODO: Make this private.
	pub fn dir_file_indices(&self, p: &str) -> (usize, usize) {
		if p.is_empty() {
			return (0, self.files.len());
		}

		let mut path_normal = p.to_owned();
		path_normal.push('/');
		let start = match self.files.binary_search_by(|entry| entry.meta.path().cmp(&path_normal)) {
			Ok(index) | Err(index) => index,
		};
		let mut end = start;
		for entry in &self.files[start..] {
			if !entry.meta.is_child_of(p) {
				break;
			}
			end += 1;
		}
		(start, end)
	}

	// Returns the sub-index of the given directory index.
	pub fn sub_index(&self, dir_index: usize) -> SubIndex {
		debug_assert!(dir_index < self.dirs.len());

		let dir = &self.dirs[dir_index];

		let (dir_start, dir_end) = self.dir_children_indices(dir_index);
		let (file_start, file_end) = self.dir_file_indices(&dir.meta.path);
		SubIndex {
			files: &self.files[file_start..file_end],
			dirs: &self.dirs[dir_start..dir_end],
		}
	}
}

impl SliceIndex for SubIndex<'_> {
	fn files(&self) -> &[File] {
		self.files
	}

	fn dirs(&self) -> &[Dir] {
		self.dirs
	}
}
