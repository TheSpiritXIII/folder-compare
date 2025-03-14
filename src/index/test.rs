use std::collections::HashSet;

use super::checksum::Checksum;
use super::metadata::parent_str;
use super::FileMetadata;
use super::Index;
use super::Metadata;

fn dirs_from_files(files: &[FileMetadata]) -> Vec<Metadata> {
	let mut dir_set = HashSet::new();
	for file in files {
		if let Some(mut parent) = file.meta.parent() {
			dir_set.insert(parent.to_string());
			loop {
				if let Some(inner) = parent_str(parent) {
					parent = inner;
					dir_set.insert(parent.to_string());
				} else {
					break;
				}
			}
		} else {
			dir_set.insert(String::new());
		}
	}
	let mut dir_list = Vec::new();
	for dir in dir_set {
		dir_list.push(Metadata::with_path(dir));
	}
	dir_list
}

fn new_test_index(files: Vec<FileMetadata>) -> Index {
	let mut index = Index::new();
	index.files = files;
	index.dirs = dirs_from_files(&index.files);
	index.normalize();
	index
}

#[test]
fn test_remove_file_empty() {
	let mut index = new_test_index(vec![]);
	index.remove_file("a.txt");
	index.remove_file("foo/a.txt");
	assert_eq!(index.files.len(), 0);
	assert_eq!(index.dirs.len(), 0);
}

#[test]
fn test_remove_file_single() {
	let mut index = new_test_index(vec![
		FileMetadata {
			meta: Metadata::with_path("a.txt"),
			size: 1,
			checksum: Checksum::new(),
		},
	]);
	index.remove_file("a.txt");
	assert_eq!(index.files.len(), 0);
	assert_eq!(index.dirs.len(), 1);
}

#[test]
fn test_remove_file_single_nested() {
	let mut index = new_test_index(vec![
		FileMetadata {
			meta: Metadata::with_path("foo/a.txt"),
			size: 1,
			checksum: Checksum::new(),
		},
	]);
	index.remove_file("foo/a.txt");
	assert_eq!(index.files.len(), 0);
	assert_eq!(index.dirs.len(), 1);
}

#[test]
fn test_remove_file_multiple() {
	let mut index = new_test_index(vec![
		FileMetadata {
			meta: Metadata::with_path("a.txt"),
			size: 1,
			checksum: Checksum::new(),
		},
		FileMetadata {
			meta: Metadata::with_path("b.txt"),
			size: 2,
			checksum: Checksum::new(),
		},
		FileMetadata {
			meta: Metadata::with_path("c.txt"),
			size: 3,
			checksum: Checksum::new(),
		},
	]);
	index.remove_file("b.txt");
	assert_eq!(index.files.len(), 2);
	assert_eq!(index.dirs.len(), 1);
	assert_eq!(index.files[0].meta.path(), "a.txt");
	assert_eq!(index.files[1].meta.path(), "c.txt");
}

#[test]
fn test_remove_file_multiple_nested() {
	let mut index = new_test_index(vec![
		FileMetadata {
			meta: Metadata::with_path("foo/a.txt"),
			size: 1,
			checksum: Checksum::new(),
		},
		FileMetadata {
			meta: Metadata::with_path("foo/b.txt"),
			size: 2,
			checksum: Checksum::new(),
		},
		FileMetadata {
			meta: Metadata::with_path("bar/c.txt"),
			size: 3,
			checksum: Checksum::new(),
		},
	]);
	index.remove_file("foo/b.txt");
	assert_eq!(index.files.len(), 2);
	assert_eq!(index.dirs.len(), 2);
	assert_eq!(index.files[0].meta.path(), "bar/c.txt");
	assert_eq!(index.files[1].meta.path(), "foo/a.txt");
}

#[test]
fn test_remove_file_nonexistent() {
	let mut index = new_test_index(vec![
		FileMetadata {
			meta: Metadata::with_path("a.txt"),
			size: 1,
			checksum: Checksum::new(),
		},
		FileMetadata {
			meta: Metadata::with_path("foo/b.txt"),
			size: 2,
			checksum: Checksum::new(),
		},
	]);
	index.remove_file("c.txt");
	assert_eq!(index.files.len(), 2);
	assert_eq!(index.dirs.len(), 2);
	index.remove_file("foo/c.txt");
	assert_eq!(index.files.len(), 2);
	assert_eq!(index.dirs.len(), 2);
}

#[test]
fn test_remove_file_first() {
	let mut index = new_test_index(vec![
		FileMetadata {
			meta: Metadata::with_path("a.txt"),
			size: 1,
			checksum: Checksum::new(),
		},
		FileMetadata {
			meta: Metadata::with_path("b.txt"),
			size: 2,
			checksum: Checksum::new(),
		},
		FileMetadata {
			meta: Metadata::with_path("c.txt"),
			size: 3,
			checksum: Checksum::new(),
		},
	]);
	index.remove_file("a.txt");
	assert_eq!(index.files.len(), 2);
	assert_eq!(index.dirs.len(), 1);
	assert_eq!(index.files[0].meta.path(), "b.txt");
	assert_eq!(index.files[1].meta.path(), "c.txt");
}

#[test]
fn test_remove_file_last() {
	let mut index = new_test_index(vec![
		FileMetadata {
			meta: Metadata::with_path("a.txt"),
			size: 1,
			checksum: Checksum::new(),
		},
		FileMetadata {
			meta: Metadata::with_path("b.txt"),
			size: 2,
			checksum: Checksum::new(),
		},
		FileMetadata {
			meta: Metadata::with_path("c.txt"),
			size: 3,
			checksum: Checksum::new(),
		},
	]);
	index.remove_file("c.txt");
	assert_eq!(index.files.len(), 2);
	assert_eq!(index.dirs.len(), 1);
	assert_eq!(index.files[0].meta.path(), "a.txt");
	assert_eq!(index.files[1].meta.path(), "b.txt");
}

#[test]
fn test_remove_dir_empty() {
	let mut index = new_test_index(vec![]);
	index.remove_dir("foo");
	assert_eq!(index.files.len(), 0);
	assert_eq!(index.dirs.len(), 0);
}

#[test]
fn test_remove_dir_single() {
	let mut index = new_test_index(vec![
		FileMetadata {
			meta: Metadata::with_path("foo/a.txt"),
			size: 1,
			checksum: Checksum::new(),
		},
	]);
	index.remove_dir("foo");
	assert_eq!(index.files.len(), 0);
	assert_eq!(index.dirs.len(), 0);
}

#[test]
fn test_remove_dir_multiple() {
	let mut index = new_test_index(vec![
		FileMetadata {
			meta: Metadata::with_path("foo/a.txt"),
			size: 1,
			checksum: Checksum::new(),
		},
		FileMetadata {
			meta: Metadata::with_path("foo/b.txt"),
			size: 2,
			checksum: Checksum::new(),
		},
		FileMetadata {
			meta: Metadata::with_path("bar/c.txt"),
			size: 3,
			checksum: Checksum::new(),
		},
	]);
	index.remove_dir("foo");
	assert_eq!(index.files.len(), 1);
	assert_eq!(index.dirs.len(), 1);
	assert_eq!(index.files[0].meta.path(), "bar/c.txt");
}

#[test]
fn test_remove_dir_nonexistent() {
	let mut index = new_test_index(vec![
		FileMetadata {
			meta: Metadata::with_path("foo/a.txt"),
			size: 1,
			checksum: Checksum::new(),
		},
		FileMetadata {
			meta: Metadata::with_path("bar/b.txt"),
			size: 2,
			checksum: Checksum::new(),
		},
	]);
	index.remove_dir("baz");
	assert_eq!(index.files.len(), 2);
	assert_eq!(index.dirs.len(), 2);
}

#[test]
fn test_remove_dir_root() {
	let mut index = new_test_index(vec![
		FileMetadata {
			meta: Metadata::with_path("a.txt"),
			size: 1,
			checksum: Checksum::new(),
		},
		FileMetadata {
			meta: Metadata::with_path("b.txt"),
			size: 2,
			checksum: Checksum::new(),
		},
		FileMetadata {
			meta: Metadata::with_path("foo/c.txt"),
			size: 3,
			checksum: Checksum::new(),
		},
	]);
	index.remove_dir("");
	assert_eq!(index.files.len(), 0);
	assert_eq!(index.dirs.len(), 0);
}

#[test]
fn test_remove_dir_nested() {
	let mut index = new_test_index(vec![
		FileMetadata {
			meta: Metadata::with_path("foo/bar/a.txt"),
			size: 1,
			checksum: Checksum::new(),
		},
		FileMetadata {
			meta: Metadata::with_path("foo/b.txt"),
			size: 2,
			checksum: Checksum::new(),
		},
		FileMetadata {
			meta: Metadata::with_path("c.txt"),
			size: 3,
			checksum: Checksum::new(),
		},
	]);
	index.remove_dir("foo/bar");
	assert_eq!(index.files.len(), 2);
	assert_eq!(index.dirs.len(), 2);
	assert_eq!(index.files[0].meta.path(), "c.txt");
	assert_eq!(index.files[1].meta.path(), "foo/b.txt");
}

#[test]
fn test_remove_dir_nested_children() {
	let mut index = new_test_index(vec![
		FileMetadata {
			meta: Metadata::with_path("foo/bar/baz/a.txt"),
			size: 1,
			checksum: Checksum::new(),
		},
		FileMetadata {
			meta: Metadata::with_path("foo/bar/b.txt"),
			size: 2,
			checksum: Checksum::new(),
		},
		FileMetadata {
			meta: Metadata::with_path("c.txt"),
			size: 3,
			checksum: Checksum::new(),
		},
	]);
	index.remove_dir("foo");
	assert_eq!(index.files.len(), 1);
	assert_eq!(index.dirs.len(), 1);
	assert_eq!(index.files[0].meta.path(), "c.txt");
}
