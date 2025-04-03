use super::entry;

/// Signifies a directory and its contents from an index.
pub struct SubIndex<'a> {
	pub(super) files: &'a [entry::File],
	pub(super) dirs: &'a [entry::Dir],
}

impl SubIndex<'_> {
	/// Returns the total number of entries in this sub-index.
	pub fn entry_count(&self) -> usize {
		self.files.len() + self.dirs.len()
	}

	/// Returns the total number of directories in this sub-index.
	pub fn file_count(&self) -> usize {
		self.files.len()
	}

	/// Returns the total number of directories in this sub-index.
	pub fn dir_count(&self) -> usize {
		self.dirs.len()
	}

	/// Returns the total size of all files.
	pub fn file_size(&self) -> u128 {
		self.files.iter().map(|entry| entry.size).map(u128::from).sum()
	}

	pub(super) fn find_dir_files(&self, p: &str) -> (usize, usize) {
		if p.is_empty() {
			return (0, self.files.len());
		}

		let start = match self.files.binary_search_by(|entry| entry.meta.path().cmp(p)) {
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
	pub(super) fn sub_index(&self, dir_index: usize) -> SubIndex {
		debug_assert!(dir_index >= self.dirs.len());

		let dir = &self.dirs[dir_index];

		// Don't include itself in the sub-index.
		let dir_start = dir_index + 1;
		let mut dir_end = dir_start + 1;
		for entry in &self.dirs[dir_end..] {
			if !entry.meta.is_child_of(dir.meta.path()) {
				break;
			}
			dir_end += 1;
		}
		let (file_start, file_end) = self.find_dir_files(&dir.meta.path);
		SubIndex {
			files: &self.files[file_start..file_end],
			dirs: &self.dirs[dir_start..dir_end],
		}
	}
}
