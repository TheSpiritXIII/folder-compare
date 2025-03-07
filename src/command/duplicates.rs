use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use anyhow::Context;
use anyhow::Result;

use crate::command::task::interval;
use crate::command::task::Task;
use crate::index;
use crate::matches;

pub fn duplicates(index_file: &PathBuf) -> Result<()> {
	let index = index::Index::open(index_file).with_context(|| {
		let path = index_file.to_string_lossy();
		format!("Unable to open index: {path}")
	})?;

	println!("Comparing files...");
	let task = Arc::new(Task::new());
	let task_thread = task.clone();

	let print_thread = thread::spawn(move || {
		interval(
			|| task_thread.done(),
			|| {
				let found = task_thread.counter.value();
				println!("Discovered {found} entries...");
			},
		);
	});

	let duplicates = index.calculate_duplicates(&task.counter);
	task.set_done();
	print_thread.join().unwrap();

	if duplicates.is_empty() {
		println!("No duplicates found");
		return Ok(());
	}

	for (match_kind, file_list) in duplicates {
		match match_kind {
			matches::MatchKind::Size => {
				println!("Potential duplicates: {file_list:?}");
			}
			matches::MatchKind::Metadata => {
				println!("Metadata duplicates: {file_list:?}");
			}
			matches::MatchKind::Checksums => {
				println!("Checksum duplicates: {file_list:?}");
			}
		}
	}
	Ok(())
}
