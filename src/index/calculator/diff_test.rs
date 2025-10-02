use std::time::Duration;
use std::time::SystemTime;

use super::diff;
use crate::index::model::Checksum;
use crate::index::model::File;
use crate::index::model::Metadata;
use crate::index::Diff;

// TODO: A builder API would look nicer here.
fn create_file(name: &str, size: u64, created: u64, modified: u64) -> File {
	File {
		meta: Metadata {
			path: name.to_string(),
			created_time: SystemTime::UNIX_EPOCH + Duration::from_secs(created),
			modified_time: SystemTime::UNIX_EPOCH + Duration::from_secs(modified),
			hidden: false,
		},
		size,
		checksum: Checksum::new(),
	}
}

#[test]
fn diff_same() {
	let mut self_files = vec![create_file("a", 1, 1, 1)];
	self_files[0].checksum = Checksum {
		sha512: "dummy_checksum".to_string(),
	};
	let mut other_files = vec![create_file("a", 1, 1, 1)];
	other_files[0].checksum = Checksum {
		sha512: "dummy_checksum".to_string(),
	};
	let mut self_dirty = false;
	let mut other_dirty = false;
	let diffs = diff(
		&mut self_files,
		&mut self_dirty,
		&mut other_files,
		&mut other_dirty,
		|_, _| {},
		false,
		false,
		false,
	)
	.unwrap();
	assert!(diffs.is_empty());
}

#[test]
fn diff_added() {
	let mut self_files = vec![create_file("a", 1, 1, 1)];
	let mut other_files = vec![];
	let mut self_dirty = false;
	let mut other_dirty = false;
	let diffs = diff(
		&mut self_files,
		&mut self_dirty,
		&mut other_files,
		&mut other_dirty,
		|_, _| {},
		false,
		false,
		false,
	)
	.unwrap();
	assert_eq!(diffs, vec![Diff::Added("a".to_string())]);
}

#[test]
fn diff_removed() {
	let mut self_files = vec![];
	let mut other_files = vec![create_file("a", 1, 1, 1)];
	let mut self_dirty = false;
	let mut other_dirty = false;
	let diffs = diff(
		&mut self_files,
		&mut self_dirty,
		&mut other_files,
		&mut other_dirty,
		|_, _| {},
		false,
		false,
		false,
	)
	.unwrap();
	assert_eq!(diffs.len(), 1);
	assert_eq!(diffs, vec![Diff::Removed("a".to_string())]);
}

#[test]
fn diff_changed() {
	let mut self_files = vec![create_file("a", 2, 1, 1)];
	let mut other_files = vec![create_file("a", 1, 1, 1)];
	let mut self_dirty = false;
	let mut other_dirty = false;
	let diffs = diff(
		&mut self_files,
		&mut self_dirty,
		&mut other_files,
		&mut other_dirty,
		|_, _| {},
		false,
		false,
		false,
	)
	.unwrap();
	assert_eq!(diffs, vec![Diff::Changed("a".to_string())]);
}

#[test]
fn diff_moved() {
	let mut self_files = vec![create_file("a", 1, 1, 1)];
	self_files[0].checksum = Checksum {
		sha512: "dummy_checksum".to_string(),
	};
	let mut other_files = vec![create_file("b", 1, 1, 1)];
	other_files[0].checksum = Checksum {
		sha512: "dummy_checksum".to_string(),
	};
	let mut self_dirty = false;
	let mut other_dirty = false;
	let diffs = diff(
		&mut self_files,
		&mut self_dirty,
		&mut other_files,
		&mut other_dirty,
		|_, _| {},
		false,
		false,
		false,
	)
	.unwrap();
	// TODO: Fix this test.
	// assert_eq!(diffs, vec![Diff::Moved("a".to_string(), "a".to_string())]);
}
