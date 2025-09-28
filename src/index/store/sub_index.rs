use crate::index::model::Dir;
use crate::index::model::File;
use crate::index::store::SliceIndex;
use crate::index::store::SortedSliceIndex;
use crate::index::store::SortedSliceIndexOpts;

/// Signifies a directory and its contents from an index.
pub struct SubIndex<'a> {
	pub(crate) files: &'a [File],
	pub(crate) dirs: &'a [Dir],
}

impl<'a> SubIndex<'a> {
	// TODO: Make this private.
	pub fn new(files: &'a [File], dirs: &'a [Dir]) -> Self {
		Self {
			files,
			dirs,
		}
	}

	// Returns the sub-index of the given directory index.
	pub fn sub_index(&self, dir_index: usize) -> SubIndex<'_> {
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

impl SortedSliceIndex for SubIndex<'_> {}
