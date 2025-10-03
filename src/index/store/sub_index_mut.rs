use std::collections::LinkedList;
use std::io;

use crate::index::calculator;
use crate::index::model::Dir;
use crate::index::model::File;
use crate::index::model::NativeFileReader;
use crate::index::store::SliceIndex;
use crate::index::store::SortedSliceIndex;
use crate::index::store::SortedSliceIndexOpts;
use crate::index::Allowlist;
use crate::index::SubIndex;
use crate::index::BUF_SIZE;

/// Mutable version of `SubIndex`.
pub struct SubIndexMut<'a> {
	// TODO: Add dirty flag here. Then we can move operations here.
	// pub(super) dirty: &'a mut bool,
	pub(super) files: &'a mut [File],
	pub(super) dirs: &'a mut [Dir],
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

pub struct FileMatchChecksumCalculator<'a> {
	#[allow(clippy::linkedlist)]
	queue: LinkedList<usize>,
	files: &'a mut [File],
	dirty: bool,
	buf: Vec<u8>,
}

impl<'a> FileMatchChecksumCalculator<'a> {
	pub fn new(
		index: &'a mut SubIndexMut<'a>,
		allowlist: &Allowlist,
		match_name: bool,
		match_created: bool,
		match_modified: bool,
	) -> Self {
		Self {
			queue: calculator::potential_file_matches(
				index.files,
				allowlist,
				match_name,
				match_created,
				match_modified,
			)
			.collect(),
			files: index.files,
			dirty: false,
			buf: Vec::with_capacity(BUF_SIZE),
		}
	}

	// TODO: Store reference to bool.
	pub(super) fn dirty(&self) -> bool {
		self.dirty
	}

	// TODO: Use a lending iterator if ever added.
	pub fn next(&mut self) -> Option<io::Result<&File>> {
		let index = self.queue.pop_back()?;
		let file = &mut self.files[index];
		if file.checksum.is_empty() {
			if let Err(e) =
				file.checksum.calculate(&NativeFileReader, file.meta.path(), &mut self.buf)
			{
				return Some(Err(e));
			}
			self.dirty = true;
		}
		Some(Ok(file))
	}
}
