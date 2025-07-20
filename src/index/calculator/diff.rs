use std::collections::HashMap;
use std::io;

use crate::index::model::Checksum;
use crate::index::model::File;
use crate::index::model::NativeFileReader;
use crate::index::BUF_SIZE;

pub enum Diff {
	Added(String),
	Removed(String),
	Changed(String),
	Moved(String, String),
}

#[allow(
	clippy::too_many_lines,
	clippy::too_many_arguments
)]
pub fn diff(
	self_files: &mut [File],
	self_dirty: &mut bool,
	other_files: &mut [File],
	other_dirty: &mut bool,
	mut notifier: impl FnMut(&str, &str),
	match_name: bool,
	match_created: bool,
	match_modified: bool,
) -> io::Result<Vec<Diff>> {
	let mut buf = Vec::with_capacity(BUF_SIZE);
	let mut diff_list = Vec::new();
	let mut file_index_self = 0;
	let mut file_index_other = 0;

	let mut file_index_self_by_checksum = HashMap::<(Checksum, u64), Vec<usize>>::new();
	let mut file_index_other_by_checksum = HashMap::<(Checksum, u64), Vec<usize>>::new();
	loop {
		if file_index_self == self_files.len() {
			for file in &other_files[file_index_other..] {
				diff_list.push(Diff::Removed(file.meta.path().to_string()));
			}
			break;
		}
		if file_index_other == other_files.len() {
			for file in &self_files[file_index_self..] {
				diff_list.push(Diff::Added(file.meta.path().to_string()));
			}
			break;
		}

		let file_self = &mut self_files[file_index_self];
		let file_other = &mut other_files[file_index_other];
		notifier(file_self.meta.path(), file_other.meta.path());

		match file_self.meta.path().cmp(file_other.meta.path()) {
			std::cmp::Ordering::Less => {
				if file_self.checksum.is_empty() {
					diff_list.push(Diff::Added(file_self.meta.path().to_string()));
				} else {
					file_index_self_by_checksum
						.entry((file_self.checksum.clone(), file_self.size))
						.or_default()
						.push(file_index_self);
				}
				file_index_self += 1;
			}
			std::cmp::Ordering::Greater => {
				if file_other.checksum.is_empty() {
					diff_list.push(Diff::Removed(file_other.meta.path().to_string()));
				} else {
					file_index_other_by_checksum
						.entry((file_other.checksum.clone(), file_other.size))
						.or_default()
						.push(file_index_other);
				}
				file_index_other += 1;
			}
			std::cmp::Ordering::Equal => {
				file_index_self += 1;
				file_index_other += 1;

				if file_self.size != file_other.size {
					diff_list.push(Diff::Changed(file_self.meta.path().to_string()));
					continue;
				}

				if match_name {
					continue;
				}
				if match_created && file_self.meta.created_time() == file_other.meta.created_time()
				{
					continue;
				}
				if match_modified
					&& file_self.meta.modified_time() == file_other.meta.modified_time()
				{
					continue;
				}

				if file_self.checksum.is_empty() {
					file_self.checksum.calculate(
						&NativeFileReader,
						file_self.meta.path(),
						&mut buf,
					)?;
					*self_dirty = true;
				}
				if file_other.checksum.is_empty() {
					file_other.checksum.calculate(
						&NativeFileReader,
						file_self.meta.path(),
						&mut buf,
					)?;
					*other_dirty = true;
				}

				if file_self.checksum != file_other.checksum {
					diff_list.push(Diff::Changed(file_self.meta.path().to_string()));
				}
			}
		}
	}

	for (checksum, path_list_self) in file_index_self_by_checksum {
		if let Some(path_list_other) = file_index_other_by_checksum.remove(&checksum) {
			if path_list_self.len() == path_list_other.len() {
				for (file_index_self, file_index_other) in
					path_list_self.iter().zip(path_list_other)
				{
					let file_self = &mut self_files[*file_index_self];
					let file_other = &mut other_files[file_index_other];

					diff_list.push(Diff::Moved(
						file_self.meta.path().to_string(),
						file_other.meta.path().to_string(),
					));
				}
				continue;
			}

			for file_index in path_list_self {
				let file = &mut self_files[file_index];
				diff_list.push(Diff::Added(file.meta.path().to_string()));
			}
			for file_index in path_list_other {
				let file = &mut other_files[file_index];
				diff_list.push(Diff::Removed(file.meta.path().to_string()));
			}
			continue;
		}

		for file_index in path_list_self {
			let file = &mut self_files[file_index];
			diff_list.push(Diff::Added(file.meta.path().to_string()));
		}
	}
	for (_, path_list) in file_index_other_by_checksum {
		for file_index in path_list {
			let file = &mut other_files[file_index];
			diff_list.push(Diff::Removed(file.meta.path().to_string()));
		}
	}

	Ok(diff_list)
}
