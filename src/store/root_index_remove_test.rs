use super::RootIndex;
use crate::store::PathIndexBuilder;

fn new_test_index(file_slice: &[&'static str]) -> RootIndex {
	let mut builder = PathIndexBuilder::new();
	for file in file_slice {
		builder.add_file(file);
	}
	builder.build()
}

#[test]
fn test_remove_file_empty() {
	let mut index = new_test_index(&vec![]);
	index.remove_file("a.txt");
	index.remove_file("foo/a.txt");
	assert_eq!(index.files.len(), 0);
	assert_eq!(index.dirs.len(), 0);
}

#[test]
fn test_remove_file_single_relative() {
	let mut index = new_test_index(&vec!["a.txt"]);
	index.remove_file("a.txt");
	assert_eq!(index.files.len(), 0);
	assert_eq!(index.dirs.len(), 1);
}

#[test]
fn test_remove_file_single_absolute() {
	let mut index = new_test_index(&vec!["/a.txt"]);
	index.remove_file("/a.txt");
	assert_eq!(index.files.len(), 0);
	assert_eq!(index.dirs.len(), 1);
}

#[test]
fn test_remove_file_single_nested() {
	let mut index = new_test_index(&vec!["foo/a.txt"]);
	index.remove_file("foo/a.txt");
	assert_eq!(index.files.len(), 0);
	assert_eq!(index.dirs.len(), 1);
}

#[test]
fn test_remove_file_multiple() {
	let mut index = new_test_index(&vec![
		"a.txt",
		"b.txt",
		"c.txt",
	]);
	index.remove_file("b.txt");
	assert_eq!(index.files.len(), 2);
	assert_eq!(index.dirs.len(), 1);
	assert_eq!(index.files[0].meta.path(), "a.txt");
	assert_eq!(index.files[1].meta.path(), "c.txt");
}

#[test]
fn test_remove_file_multiple_nested() {
	let mut index = new_test_index(&vec![
		"foo/a.txt",
		"foo/b.txt",
		"bar/c.txt",
	]);
	index.remove_file("foo/b.txt");
	assert_eq!(index.files.len(), 2);
	assert_eq!(index.dirs.len(), 2);
	assert_eq!(index.files[0].meta.path(), "bar/c.txt");
	assert_eq!(index.files[1].meta.path(), "foo/a.txt");
}

#[test]
fn test_remove_file_nonexistent() {
	let mut index = new_test_index(&vec![
		"a.txt",
		"foo/b.txt",
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
	let mut index = new_test_index(&vec![
		"a.txt",
		"b.txt",
		"c.txt",
	]);
	index.remove_file("a.txt");
	assert_eq!(index.files.len(), 2);
	assert_eq!(index.dirs.len(), 1);
	assert_eq!(index.files[0].meta.path(), "b.txt");
	assert_eq!(index.files[1].meta.path(), "c.txt");
}

#[test]
fn test_remove_file_last() {
	let mut index = new_test_index(&vec![
		"a.txt",
		"b.txt",
		"c.txt",
	]);
	index.remove_file("c.txt");
	assert_eq!(index.files.len(), 2);
	assert_eq!(index.dirs.len(), 1);
	assert_eq!(index.files[0].meta.path(), "a.txt");
	assert_eq!(index.files[1].meta.path(), "b.txt");
}

#[test]
fn test_remove_dir_empty() {
	let mut index = new_test_index(&vec![]);
	index.remove_dir("foo");
	assert_eq!(index.files.len(), 0);
	assert_eq!(index.dirs.len(), 0);
}

#[test]
fn test_remove_dir_single() {
	let mut index = new_test_index(&vec!["foo/a.txt"]);
	index.remove_dir("foo");
	assert_eq!(index.files.len(), 0);
	assert_eq!(index.dirs.len(), 0);
}

#[test]
fn test_remove_dir_multiple() {
	let mut index = new_test_index(&vec![
		"foo/a.txt",
		"foo/b.txt",
		"bar/c.txt",
	]);
	index.remove_dir("foo");
	assert_eq!(index.files.len(), 1);
	assert_eq!(index.dirs.len(), 1);
	assert_eq!(index.files[0].meta.path(), "bar/c.txt");
}

#[test]
fn test_remove_dir_nonexistent() {
	let mut index = new_test_index(&vec![
		"foo/a.txt",
		"bar/b.txt",
	]);
	index.remove_dir("baz");
	assert_eq!(index.files.len(), 2);
	assert_eq!(index.dirs.len(), 2);
}

#[test]
fn test_remove_dir_root() {
	let mut index = new_test_index(&vec![
		"a.txt",
		"b.txt",
		"foo/c.txt",
	]);
	index.remove_dir("");
	assert_eq!(index.files.len(), 0);
	assert_eq!(index.dirs.len(), 0);
}

#[test]
fn test_remove_dir_nested() {
	let mut index = new_test_index(&vec![
		"foo/bar/a.txt",
		"foo/b.txt",
		"c.txt",
	]);
	index.remove_dir("foo/bar");
	assert_eq!(index.files.len(), 2);
	assert_eq!(index.dirs.len(), 2);
	assert_eq!(index.files[0].meta.path(), "c.txt");
	assert_eq!(index.files[1].meta.path(), "foo/b.txt");
}

#[test]
fn test_remove_dir_nested_children() {
	let mut index = new_test_index(&vec![
		"foo/bar/baz/a.txt",
		"foo/bar/b.txt",
		"c.txt",
	]);
	index.remove_dir("foo");
	assert_eq!(index.files.len(), 1);
	assert_eq!(index.dirs.len(), 1);
	assert_eq!(index.files[0].meta.path(), "c.txt");
}
