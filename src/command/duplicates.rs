use std::path::PathBuf;
use std::thread;

use anyhow::Context;
use anyhow::Result;

use crate::command::task::condition_delay;
use crate::command::task::Task;
use crate::index;
use crate::util::terminal::clear_line;

pub fn duplicates(index_file: &PathBuf) -> Result<()> {
	let mut index = index::Index::open(index_file)
		.with_context(|| format!("Unable to open index: {}", index_file.display()))?;

	println!("Comparing files...");
	let task = Task::new();
	thread::scope(|s| -> Result<_> {
		s.spawn(|| {
			loop {
				if condition_delay(|| task.done()) {
					return;
				}
				let found = task.counter.value();
				clear_line();
				print!("Discovered {found} entries...");
			}
		});

		index.calculate_matches(&task.counter)?;
		task.set_done();
		Ok(())
	})?;

	let duplicates = index.duplicates();
	if duplicates.is_empty() {
		println!("No duplicates found");
		return Ok(());
	}

	for file_list in duplicates {
		println!("Duplicate: {file_list:?}");
	}
	Ok(())
}
