use std::collections::LinkedList;
use std::io;

use crate::index::calculator;
use crate::index::model::Dir;
use crate::index::model::File;
use crate::index::model::NativeFileReader;
use crate::index::store::SliceIndex;
use crate::index::store::SortedSliceIndex;
use crate::index::Allowlist;
use crate::index::RootIndex;
use crate::index::SubIndex;
use crate::index::BUF_SIZE;

/// Mutable version of `SubIndex`.
pub struct SubIndexMut<'a> {
	pub(super) root: &'a mut RootIndex,
	pub(super) file_start: usize,
	pub(super) file_end: usize,
	pub(super) dir_start: usize,
	pub(super) dir_end: usize,
}

impl SubIndexMut<'_> {
	// TODO: Make this conversion automatic.
	pub fn all(&self) -> SubIndex<'_> {
		SubIndex {
			files: self.files(),
			dirs: self.dirs(),
		}
	}

	pub fn root_mut(&mut self) -> &mut RootIndex {
		self.root
	}
}

impl SliceIndex for SubIndexMut<'_> {
	fn files(&self) -> &[File] {
		&self.root.files[self.file_start..self.file_end]
	}

	fn dirs(&self) -> &[Dir] {
		&self.root.dirs[self.dir_start..self.dir_end]
	}
}

impl SortedSliceIndex for SubIndexMut<'_> {}

pub struct ChecksumCalculator<'a> {
	#[allow(clippy::linkedlist)]
	queue: LinkedList<usize>,
	index: &'a mut SubIndexMut<'a>,
	buf: Vec<u8>,
}

impl<'a> ChecksumCalculator<'a> {
	pub fn with_file_match(
		index: &'a mut SubIndexMut<'a>,
		allowlist: &Allowlist,
		match_name: bool,
		match_created: bool,
		match_modified: bool,
	) -> Self {
		Self {
			queue: calculator::potential_file_matches(
				index.files(),
				allowlist,
				match_name,
				match_created,
				match_modified,
			)
			.collect(),
			index,
			buf: Vec::with_capacity(BUF_SIZE),
		}
	}

	pub fn with_dir_match(
		index: &'a mut SubIndexMut<'a>,
		allowlist: &Allowlist,
		match_name: bool,
		match_created: bool,
		match_modified: bool,
	) -> Self {
		Self {
			queue: calculator::potential_dir_matches(
				&index.all(),
				allowlist,
				match_name,
				match_created,
				match_modified,
			)
			.collect(),
			index,
			buf: Vec::with_capacity(BUF_SIZE),
		}
	}

	pub fn index_mut(&mut self) -> &mut SubIndexMut<'a> {
		self.index
	}

	// TODO: Use a lending iterator if ever added.
	pub fn next(&mut self) -> Option<io::Result<&File>> {
		let index = self.queue.pop_back()?;
		let file = &mut self.index.root.files[self.index.file_start + index];
		if file.checksum.is_empty() {
			if let Err(e) =
				file.checksum.calculate(&NativeFileReader, file.meta.path(), &mut self.buf)
			{
				return Some(Err(e));
			}
			self.index.root.dirty = true;
		}
		Some(Ok(file))
	}
}
