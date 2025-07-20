use super::RootIndex;
use crate::index::builder::PathIndexBuilder;

fn test_store() -> RootIndex {
	let mut builder = PathIndexBuilder::new();
	builder.add_dir("abc");
	builder.add_dir("abc/xyz");
	builder.add_dir("foo");
	builder.add_dir("foo/bar");
	builder.add_dir("foo/bar/nested");
	builder.add_dir("foo/empty");
	builder.add_dir("vw");
	builder.add_file("foo/bar/f.txt");
	builder.add_file("foo/bar/g.txt");
	builder.add_file("foo/bar/h.txt");
	builder.add_file("foo/bar/nested/d.txt");
	builder.add_file("foo/bar/nested/e.txt");
	builder.add_file("i.txt");
	builder.build()
}

#[test]
fn test_sub_index_empty_nested_root() {
	let store = test_store();
	let index = store.all();
	let dir_index = index.dir_index("abc").unwrap();
	let sub_index = index.sub_index(dir_index);
	assert_eq!(sub_index.dir_count(), 1);
	assert_eq!(sub_index.file_count(), 0);
}

#[test]
fn test_sub_index_empty_nested_leaf() {
	let store = test_store();
	let index = store.all();
	let dir_index = index.dir_index("abc/xyz").unwrap();
	let sub_index = index.sub_index(dir_index);
	assert_eq!(sub_index.dir_count(), 0);
	assert_eq!(sub_index.file_count(), 0);
}

#[test]
fn test_sub_index_deeply_nested_root() {
	let store = test_store();
	let index = store.all();
	let dir_index = index.dir_index("foo").unwrap();
	let sub_index = index.sub_index(dir_index);
	assert_eq!(sub_index.dir_count(), 3);
	assert_eq!(sub_index.file_count(), 5);
}

#[test]
fn test_sub_index_deeply_nested_child() {
	let store = test_store();
	let index = store.all();
	let dir_index = index.dir_index("foo/bar").unwrap();
	let sub_index = index.sub_index(dir_index);
	assert_eq!(sub_index.dir_count(), 1);
	assert_eq!(sub_index.file_count(), 5);
}

#[test]
fn test_sub_index_deeply_nested_leaf() {
	let store = test_store();
	let index = store.all();
	let dir_index = index.dir_index("foo/bar/nested").unwrap();
	let sub_index = index.sub_index(dir_index);
	assert_eq!(sub_index.dir_count(), 0);
	assert_eq!(sub_index.file_count(), 2);
}

#[test]
fn test_sub_index_nested_child_empty() {
	let store = test_store();
	let index = store.all();
	let dir_index = index.dir_index("foo/empty").unwrap();
	let sub_index = index.sub_index(dir_index);
	assert_eq!(sub_index.dir_count(), 0);
	assert_eq!(sub_index.file_count(), 0);
}

#[test]
fn test_sub_index_empty_root() {
	let store = test_store();
	let index = store.all();
	let dir_index = index.dir_index("vw").unwrap();
	let sub_index = index.sub_index(dir_index);
	assert_eq!(sub_index.dir_count(), 0);
	assert_eq!(sub_index.file_count(), 0);
}

#[test]
fn test_sub_index_file_same_name_as_dir() {
	let mut builder = PathIndexBuilder::new();
	builder.add_dir("a");
	builder.add_dir("a/b");
	builder.add_file("a.txt");
	builder.add_file("a/a.txt");
	builder.add_file("a/b.txt");
	builder.add_file("a/b/a.txt");
	builder.add_file("a/b/b.txt");
	builder.add_file("b.txt");
	let store = builder.build();
	let index = store.all();
	let dir_index = index.dir_index("a").unwrap();
	let sub_index = index.sub_index(dir_index);
	assert_eq!(sub_index.dir_count(), 1);
	assert_eq!(sub_index.file_count(), 4);
}
