use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use anyhow::Context;
use anyhow::Result;

use crate::command::task::interval;
use crate::command::task::Task;
use crate::index;

pub fn stats(path: &PathBuf) -> Result<()> {
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

	let index = index::Index::from_path(path, &task.counter).with_context(|| {
		let path = path.to_string_lossy();
		format!("Unable to index: {path}")
	})?;

	task.set_done();
	print_thread.join().unwrap();

	let count = index.entry_count();
	println!("Found {count} total entries!");
	let file_count = index.file_count();
	println!("{file_count} files.");
	let dir_count = index.dirs_count();
	println!("{dir_count} directories.");
	Ok(())
}
