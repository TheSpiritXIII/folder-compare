use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use anyhow::bail;
use anyhow::Context;
use anyhow::Result;

use crate::command::task::condition_delay;
use crate::command::task::Task;
use crate::index::Index;

pub fn stats(src: Option<&PathBuf>, index_file: Option<&PathBuf>) -> Result<()> {
	let task = Arc::new(Task::new());
	let task_thread = task.clone();

	let print_thread = thread::spawn(move || {
		loop {
			if condition_delay(|| task_thread.done()) {
				return;
			}
			let found = task_thread.counter.value();
			println!("Discovered {found} entries...");
		}
	});

	let index = if let Some(path) = index_file {
		println!("Opening index file...");
		let mut index = Index::open(path)
			.with_context(|| format!("Unable to open index: {}", path.display()))?;
		if let Some(path) = src {
			index.add(std::path::absolute(path)?, &task.counter)?;
		}
		index
	} else if let Some(path) = src {
		Index::from_path(std::path::absolute(path)?, &task.counter)?
	} else {
		bail!("Expected source or index-file");
	};

	task.set_done();
	print_thread.join().unwrap();

	let count = index.entry_count();
	println!("Found {count} total entries!");
	let file_count = index.file_count();
	println!("{file_count} files.");
	let dir_count = index.dirs_count();
	println!("{dir_count} directories.");

	if let Some(path) = index_file {
		index.save(path)?;
	}
	Ok(())
}
