use crate::index::model::Dir;
use crate::index::model::File;
use crate::index::store::SliceIndex;
use crate::index::store::SortedSliceIndex;
use crate::index::store::SortedSliceIndexOpts;
use crate::index::SubIndex;

/// Mutable version of `SubIndex`.
pub struct SubIndexMut<'a> {
	// TODO: Add dirty flag here. Then we can move operations here.
	// pub(crate) dirty: &'a mut bool,
	pub(crate) files: &'a mut [File],
	pub(crate) dirs: &'a mut [Dir],
}

impl SubIndexMut<'_> {
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

	pub fn files_mut(&mut self) -> &mut [File] {
		self.files
	}
}

impl SliceIndex for SubIndexMut<'_> {
	fn files(&self) -> &[File] {
		self.files
	}

	fn dirs(&self) -> &[Dir] {
		self.dirs
	}
}

impl SortedSliceIndex for SubIndexMut<'_> {}
