use crate::index::model::Dir;
use crate::index::model::File;

// File index methods.
pub trait Index {
	/// Returns the total number of filesystem entries.
	fn entry_count(&self) -> usize;

	/// Returns the total number of files.
	fn file_count(&self) -> usize;

	/// Returns the total number of directories.
	fn dir_count(&self) -> usize;

	/// Returns the total size of all files.
	fn file_size(&self) -> u128;
}

// A file index which stores slices of files and directories.
pub trait SliceIndex {
	// Returns the list of files. These files are not guaranteed to be in any order.
	fn files(&self) -> &[File];
	// Returns the list of directories. These directories are not guaranteed to be in any order.
	fn dirs(&self) -> &[Dir];
}

impl<T> Index for T
where
	T: SliceIndex,
{
	fn entry_count(&self) -> usize {
		self.files().len() + self.dirs().len()
	}

	fn file_count(&self) -> usize {
		self.files().len()
	}

	fn dir_count(&self) -> usize {
		self.dirs().len()
	}

	fn file_size(&self) -> u128 {
		self.files().iter().map(|entry| entry.size).map(u128::from).sum()
	}
}

// Marker for a slice index which has files and directories sorted by path.
// TODO: Make this private.
pub trait SortedSliceIndex: SliceIndex {}

// TODO: Make this private.
pub trait SortedSliceIndexOpts {
	fn dir_index(&self, path: &str) -> Option<usize>;
	fn file_index(&self, p: &str) -> Option<usize>;
	fn dir_children_indices(&self, dir_index: usize) -> (usize, usize);
	fn dir_file_indices(&self, p: &str) -> (usize, usize);
}

impl<T> SortedSliceIndexOpts for T
where
	T: SortedSliceIndex,
{
	fn dir_index(&self, path: &str) -> Option<usize> {
		if path.is_empty() {
			return None;
		}
		self.dirs().binary_search_by(|entry| entry.meta.path().cmp(path)).ok()
	}

	fn file_index(&self, p: &str) -> Option<usize> {
		self.files().binary_search_by(|entry| entry.meta.path().cmp(p)).ok()
	}

	fn dir_children_indices(&self, dir_index: usize) -> (usize, usize) {
		let dir = &self.dirs()[dir_index];
		// Don't include itself in the sub-index.
		let dir_start = dir_index + 1;
		let mut dir_end = dir_start;
		for entry in &self.dirs()[dir_start..] {
			if !entry.meta.is_child_of(dir.meta.path()) {
				break;
			}
			dir_end += 1;
		}
		(dir_start, dir_end)
	}

	// TODO: Make this private.
	fn dir_file_indices(&self, p: &str) -> (usize, usize) {
		if p.is_empty() {
			return (0, self.files().len());
		}

		let mut path_normal = p.to_owned();
		path_normal.push('/');
		let start = match self.files().binary_search_by(|entry| entry.meta.path().cmp(&path_normal))
		{
			Ok(index) | Err(index) => index,
		};
		let mut end = start;
		for entry in &self.files()[start..] {
			if !entry.meta.is_child_of(p) {
				break;
			}
			end += 1;
		}
		(start, end)
	}
}
