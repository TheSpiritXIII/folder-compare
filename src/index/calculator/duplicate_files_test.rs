use std::time::Duration;
use std::time::SystemTime;

use super::potential_file_matches;
use crate::index::calculator::Allowlist;
use crate::index::model::Checksum;
use crate::index::model::File;
use crate::index::model::Metadata;

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
fn potentially_matching_name_same_size() {
	let files = vec![
		create_file("a", 1, 1, 1),
		create_file("a", 1, 2, 2),
		create_file("b", 1, 1, 1),
	];
	let allowlist = Allowlist::allow_all();
	let matches: Vec<_> = potential_file_matches(&files, &allowlist, true, false, false).collect();
	assert_eq!(
		matches,
		vec![
			0,
			1
		]
	);
}

#[test]
fn potentially_matching_name_different_size() {
	let files = vec![
		create_file("a", 1, 1, 1),
		create_file("a", 2, 2, 2),
		create_file("b", 1, 1, 1),
	];
	let allowlist = Allowlist::allow_all();
	let matches: Vec<_> = potential_file_matches(&files, &allowlist, true, false, false).collect();
	assert!(matches.is_empty());
}

#[test]
fn potentially_matching_created_same_size() {
	let files = vec![
		create_file("a", 1, 1, 1),
		create_file("b", 1, 1, 2),
		create_file("c", 1, 2, 1),
	];
	let allowlist = Allowlist::allow_all();
	let matches: Vec<_> = potential_file_matches(&files, &allowlist, false, true, false).collect();
	assert_eq!(
		matches,
		vec![
			0,
			1
		]
	);
}

#[test]
fn potentially_matching_created_different_size() {
	let files = vec![
		create_file("a", 1, 1, 1),
		create_file("b", 2, 1, 2),
		create_file("c", 1, 2, 1),
	];
	let allowlist = Allowlist::allow_all();
	let matches: Vec<_> = potential_file_matches(&files, &allowlist, false, true, false).collect();
	assert!(matches.is_empty());
}

#[test]
fn potentially_matching_modified_same_size() {
	let files = vec![
		create_file("a", 1, 1, 1),
		create_file("b", 1, 2, 1),
		create_file("c", 1, 1, 2),
	];
	let allowlist = Allowlist::allow_all();
	let matches: Vec<_> = potential_file_matches(&files, &allowlist, false, false, true).collect();
	assert_eq!(
		matches,
		vec![
			0,
			1
		]
	);
}

#[test]
fn potentially_matching_modified_different_size() {
	let files = vec![
		create_file("a", 1, 1, 1),
		create_file("b", 2, 2, 1),
		create_file("c", 1, 1, 2),
	];
	let allowlist = Allowlist::allow_all();
	let matches: Vec<_> = potential_file_matches(&files, &allowlist, false, false, true).collect();
	assert!(matches.is_empty());
}
