use std::collections::HashMap;
use std::hash::Hash;
use std::time::SystemTime;

use super::Allowlist;
use crate::index::model::Checksum;
use crate::index::model::File;

struct FileAttributeCounter<T: Eq + Hash> {
	attribute_by_index: HashMap<T, usize>,
	attribute_fn: fn(&File) -> T,
}

impl<T> FileAttributeCounter<T>
where
	T: Eq + Hash,
{
	fn new(f: fn(&File) -> T) -> Self {
		Self {
			attribute_by_index: HashMap::new(),
			attribute_fn: f,
		}
	}

	fn record(&mut self, file: &File) {
		let attribute = (self.attribute_fn)(file);
		let count = self.attribute_by_index.entry(attribute).or_default();
		*count += 1;
	}

	fn has_matches(&self, file: &File) -> bool {
		let attribute = (self.attribute_fn)(file);
		let Some(count) = self.attribute_by_index.get(&attribute) else {
			return false;
		};
		*count > 1
	}
}

impl FileAttributeCounter<String> {
	fn with_name_matcher() -> Self {
		Self::new(|file| file.meta.name().to_owned())
	}
}

impl FileAttributeCounter<SystemTime> {
	fn with_modified_matcher() -> Self {
		Self::new(|file| file.meta.modified_time)
	}

	fn with_created_matcher() -> Self {
		Self::new(|file| file.meta.created_time)
	}
}

pub fn potential_file_matches(
	files: &[File],
	allowlist: &Allowlist,
	match_name: bool,
	match_created: bool,
	match_modified: bool,
) -> impl Iterator<Item = usize> {
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
		let mut name_counter = FileAttributeCounter::with_name_matcher();
		let mut created_counter = FileAttributeCounter::with_created_matcher();
		let mut modified_counter = FileAttributeCounter::with_modified_matcher();
		for file_index in path_list {
			let file = &files[*file_index];
			if match_name {
				name_counter.record(file);
			}
			if match_created {
				created_counter.record(file);
			}
			if match_modified {
				modified_counter.record(file);
			}
		}

		for file_index in path_list {
			let file = &files[*file_index];
			if match_name && !name_counter.has_matches(file) {
				continue;
			}
			if match_created && !created_counter.has_matches(file) {
				continue;
			}
			if match_modified && !modified_counter.has_matches(file) {
				continue;
			}

			file_matched[*file_index] = true;
		}
	}

	file_matched.into_iter().enumerate().filter_map(|(file_index, matched)| {
		if !matched {
			return None;
		}
		Some(file_index)
	})
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
