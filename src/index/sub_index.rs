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

	pub fn file_size(&self) -> u128 {
		self.files.iter().map(|entry| entry.size).map(u128::from).sum()
	}
}
