use std::collections::HashMap;
use std::io;
use std::time::SystemTime;

use super::Allowlist;
use crate::index::model::Checksum;
use crate::index::model::File;
use crate::index::model::NativeFileReader;
use crate::index::BUF_SIZE;

#[allow(clippy::too_many_arguments)]
pub fn calculate_matches(
	files: &mut [File],
	dirty: &mut bool,
	mut notifier: impl FnMut(&str),
	allowlist: &Allowlist,
	match_name: bool,
	match_created: bool,
	match_modified: bool,
) -> io::Result<()> {
	let mut file_index_by_size = HashMap::<u64, Vec<usize>>::new();
	for (file_index, file) in files.iter().enumerate() {
		if !allowlist.is_allowed(&file.meta.path) {
			continue;
		}
		file_index_by_size.entry(file.size).or_default().push(file_index);
	}

	let mut file_matched = vec![false; files.len()];
	for path_list in file_index_by_size.values() {
		if path_list.len() < 2 {
			continue;
		}
		let mut name_by_count = HashMap::<String, usize>::new();
		let mut created_by_count = HashMap::<SystemTime, usize>::new();
		let mut modified_by_count = HashMap::<SystemTime, usize>::new();
		for file_index in path_list {
			let file = &files[*file_index];
			if match_name {
				name_by_count
					.entry(file.meta.name().to_string())
					.and_modify(|count| *count += 1)
					.or_insert(1);
			}
			if match_created {
				created_by_count
					.entry(file.meta.created_time())
					.and_modify(|count| *count += 1)
					.or_insert(1);
			}
			if match_modified {
				modified_by_count
					.entry(file.meta.modified_time())
					.and_modify(|count| *count += 1)
					.or_insert(1);
			}
		}

		for file_index in path_list {
			let file = &files[*file_index];
			if match_name {
				if let Some(count) = name_by_count.get(file.meta.name()) {
					if *count < 2 {
						continue;
					}
				}
			}
			if match_created {
				if let Some(count) = created_by_count.get(&file.meta.modified_time()) {
					if *count < 2 {
						continue;
					}
				}
			}
			if match_modified {
				if let Some(count) = modified_by_count.get(&file.meta.modified_time()) {
					if *count < 2 {
						continue;
					}
				}
			}

			file_matched[*file_index] = true;
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

pub fn duplicates(files: &[File], allowlist: &Allowlist) -> Vec<Vec<String>> {
	let mut path_by_checksum = HashMap::<(Checksum, u64), Vec<String>>::new();
	for file in files {
		if !file.checksum.is_empty() {
			if !allowlist.is_allowed(&file.meta.path) {
				continue;
			}
			path_by_checksum
				.entry((file.checksum.clone(), file.size))
				.or_default()
				.push(file.meta.path().to_string());
		}
	}

	let mut matches = Vec::new();
	for (_, mut path_list) in path_by_checksum {
		if path_list.len() > 1 {
			path_list.sort();
			matches.push(path_list);
		}
	}
	matches.sort();
	matches
}
