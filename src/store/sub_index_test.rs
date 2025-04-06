use std::time::SystemTime;

use crate::store::checksum::Checksum;
use crate::store::entry::Dir;
use crate::store::entry::File;
use crate::store::metadata::Metadata;
use crate::store::sub_index::SubIndex;

fn metadata_with_path(path: &str) -> Metadata {
	Metadata {
		path: path.to_string(),
		created_time: SystemTime::UNIX_EPOCH,
		modified_time: SystemTime::UNIX_EPOCH,
	}
}

fn dir_with_path(path: &str) -> Dir {
	Dir {
		meta: metadata_with_path(path),
	}
}

fn file_with_path(path: &str) -> File {
	File {
		meta: metadata_with_path(path),
		size: 1,
		checksum: Checksum::new(),
	}
}

struct TestStore {
	dirs: Vec<Dir>,
	files: Vec<File>,
}

impl TestStore {
	fn index(&self) -> SubIndex {
		SubIndex {
			files: &self.files,
			dirs: &self.dirs,
		}
	}
}

fn test_store() -> TestStore {
	return TestStore {
		dirs: vec![
			dir_with_path("abc"),
			dir_with_path("abc/xyz"),
			dir_with_path("foo"),
			dir_with_path("foo/bar"),
			dir_with_path("foo/bar/nested"),
			dir_with_path("foo/empty"),
			dir_with_path("vw"),
		],
		files: vec![
			file_with_path("foo/bar/f.txt"),
			file_with_path("foo/bar/g.txt"),
			file_with_path("foo/bar/h.txt"),
			file_with_path("foo/bar/nested/d.txt"),
			file_with_path("foo/bar/nested/e.txt"),
			file_with_path("i.txt"),
		],
	};
}

#[test]
fn test_sub_index_empty_nested_root() {
	let store = test_store();
	let index = store.index();
	let dir_index = index.dir_index("abc").unwrap();
	let sub_index = index.sub_index(dir_index);
	assert_eq!(sub_index.dir_count(), 1);
	assert_eq!(sub_index.file_count(), 0);
}

#[test]
fn test_sub_index_empty_nested_leaf() {
	let store = test_store();
	let index = store.index();
	let dir_index = index.dir_index("abc/xyz").unwrap();
	let sub_index = index.sub_index(dir_index);
	assert_eq!(sub_index.dir_count(), 0);
	assert_eq!(sub_index.file_count(), 0);
}

#[test]
fn test_sub_index_deeply_nested_root() {
	let store = test_store();
	let index = store.index();
	let dir_index = index.dir_index("foo").unwrap();
	let sub_index = index.sub_index(dir_index);
	assert_eq!(sub_index.dir_count(), 3);
	assert_eq!(sub_index.file_count(), 5);
}

#[test]
fn test_sub_index_deeply_nested_child() {
	let store = test_store();
	let index = store.index();
	let dir_index = index.dir_index("foo/bar").unwrap();
	let sub_index = index.sub_index(dir_index);
	assert_eq!(sub_index.dir_count(), 1);
	assert_eq!(sub_index.file_count(), 5);
}

#[test]
fn test_sub_index_deeply_nested_leaf() {
	let store = test_store();
	let index = store.index();
	let dir_index = index.dir_index("foo/bar/nested").unwrap();
	let sub_index = index.sub_index(dir_index);
	assert_eq!(sub_index.dir_count(), 0);
	assert_eq!(sub_index.file_count(), 2);
}

#[test]
fn test_sub_index_nested_child_empty() {
	let store = test_store();
	let index = store.index();
	let dir_index = index.dir_index("foo/empty").unwrap();
	let sub_index = index.sub_index(dir_index);
	assert_eq!(sub_index.dir_count(), 0);
	assert_eq!(sub_index.file_count(), 0);
}

#[test]
fn test_sub_index_empty_root() {
	let store = test_store();
	let index = store.index();
	let dir_index = index.dir_index("vw").unwrap();
	let sub_index = index.sub_index(dir_index);
	assert_eq!(sub_index.dir_count(), 0);
	assert_eq!(sub_index.file_count(), 0);
}

#[test]
fn test_sub_index_file_same_name_as_dir() {
	let store = TestStore {
		dirs: vec![
			dir_with_path("a"),
			dir_with_path("a/b"),
		],
		files: vec![
			file_with_path("a.txt"),
			file_with_path("a/a.txt"),
			file_with_path("a/b.txt"),
			file_with_path("a/b/a.txt"),
			file_with_path("a/b/b.txt"),
			file_with_path("b.txt"),
		],
	};
	let index = store.index();
	let dir_index = index.dir_index("a").unwrap();
	let sub_index = index.sub_index(dir_index);
	assert_eq!(sub_index.dir_count(), 1);
	assert_eq!(sub_index.file_count(), 4);
}
