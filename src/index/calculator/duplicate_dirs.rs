use std::collections::HashMap;
use std::io;
use std::time::SystemTime;

use super::Allowlist;
use crate::index::model::Checksum;
use crate::index::model::Dir;
use crate::index::model::File;
use crate::index::model::NativeFileReader;
use crate::index::SubIndex;
use crate::index::BUF_SIZE;

#[derive(PartialEq, Eq, Hash)]
pub struct DirStats {
	pub file_count: usize,
	pub file_size: u128,
	pub dir_count: usize,
}

fn dir_stats(index: &SubIndex) -> DirStats {
	DirStats {
		file_count: index.file_count(),
		file_size: index.file_size(),
		dir_count: index.dir_count(),
	}
}

#[allow(
	clippy::too_many_lines,
	clippy::too_many_arguments
)]
pub fn calculate_dir_matches(
	files: &mut [File],
	dirs: &mut [Dir],
	dirty: &mut bool,
	mut notifier: impl FnMut(&str),
	allowlist: &Allowlist,
	match_name: bool,
	match_created: bool,
	match_modified: bool,
) -> io::Result<()> {
	let index = SubIndex {
		files,
		dirs,
	};
	let mut dirs_by_stats = HashMap::<DirStats, Vec<usize>>::new();
	for (dir_index, dir) in index.dirs.iter().enumerate() {
		let sub_index = index.sub_index(dir_index);
		let stats = dir_stats(&sub_index);
		if stats.file_size == 0 {
			continue;
		}
		if !allowlist.is_allowed(&dir.meta.path) {
			continue;
		}
		dirs_by_stats.entry(stats).or_default().push(dir_index);
	}

	let mut file_matched = vec![false; index.files.len()];
	for (_, path_list) in dirs_by_stats {
		if path_list.len() < 2 {
			continue;
		}
		let mut name_by_count = HashMap::<Vec<String>, usize>::new();
		let mut created_by_count = HashMap::<Vec<SystemTime>, usize>::new();
		let mut modified_by_count = HashMap::<Vec<SystemTime>, usize>::new();
		let mut name_list = vec![Vec::new(); path_list.len()];
		let mut created_list = vec![Vec::new(); path_list.len()];
		let mut modified_list = vec![Vec::new(); path_list.len()];
		for dir_index in &path_list {
			let sub_index = index.sub_index(*dir_index);
			let file_list = sub_index.files;
			if match_name {
				name_list[*dir_index] =
					file_list.iter().map(|entry| entry.meta.path().to_string()).collect();
				name_list[*dir_index].sort();
				name_by_count
					.entry(name_list[*dir_index].clone())
					.and_modify(|count| *count += 1)
					.or_insert(1);
			}
			if match_created {
				created_list[*dir_index] =
					file_list.iter().map(|entry| entry.meta.created_time).collect();
				created_list[*dir_index].sort();
				created_by_count
					.entry(created_list[*dir_index].clone())
					.and_modify(|count| *count += 1)
					.or_insert(1);
			}
			if match_modified {
				modified_list[*dir_index] =
					file_list.iter().map(|entry| entry.meta.created_time).collect();
				modified_list[*dir_index].sort();
				modified_by_count
					.entry(modified_list[*dir_index].clone())
					.and_modify(|count| *count += 1)
					.or_insert(1);
			}
		}

		for dir_index in &path_list {
			let dir = &index.dirs[*dir_index];
			if match_name {
				if let Some(count) = name_by_count.get(&name_list[*dir_index]) {
					if *count < 2 {
						continue;
					}
				}
			}
			if match_created {
				if let Some(count) = created_by_count.get(&created_list[*dir_index]) {
					if *count < 2 {
						continue;
					}
				}
			}
			if match_modified {
				if let Some(count) = modified_by_count.get(&modified_list[*dir_index]) {
					if *count < 2 {
						continue;
					}
				}
			}

			let (start, end) = index.dir_file_indices(dir.meta.path());
			for matched in file_matched.iter_mut().take(end).skip(start) {
				*matched = true;
			}
		}
	}

	let mut buf = Vec::with_capacity(BUF_SIZE);
	for (file_index, matched) in file_matched.iter().enumerate() {
		let file = &mut files[file_index];
		notifier(file.meta.path());
		if !matched {
			continue;
		}

		if file.checksum.is_empty() {
			file.checksum.calculate(&NativeFileReader, file.meta.path(), &mut buf)?;
			*dirty = true;
		}
	}
	Ok(())
}

pub fn duplicate_dirs(index: &SubIndex, allowlist: &Allowlist) -> Vec<Vec<String>> {
	let mut dirs_by_checksums = HashMap::<(DirStats, Vec<Checksum>), Vec<String>>::new();
	for (dir_index, dir) in index.dirs.iter().enumerate() {
		if !allowlist.is_allowed(&dir.meta.path) {
			continue;
		}

		let sub_index = index.sub_index(dir_index);
		let stats = dir_stats(&sub_index);
		if stats.file_size == 0 {
			continue;
		}

		let mut file_checksums: Vec<_> =
			sub_index.files.iter().map(|entry| entry.checksum.clone()).collect();
		file_checksums.sort();

		dirs_by_checksums
			.entry((stats, file_checksums))
			.or_default()
			.push(dir.meta.path().to_string());
	}

	let mut matches = Vec::new();
	for (_, mut path_list) in dirs_by_checksums {
		if path_list.len() > 1 {
			path_list.sort();
			matches.push(path_list);
		}
	}
	matches.sort();
	matches
}
