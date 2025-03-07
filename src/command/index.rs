use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use anyhow::Context;
use anyhow::Result;

use crate::command::task::condition_delay;
use crate::command::task::Task;
use crate::index::Index;

pub fn index(src: &PathBuf, index_path: &PathBuf, sha_512: bool) -> Result<()> {
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

	let mut index = if index_path.exists() {
		let mut index = Index::open(index_path).with_context(|| {
			let path = index_path.to_string_lossy();
			format!("Unable to open index: {path}")
		})?;
		index.add(std::path::absolute(src)?, &task.counter)?;
		index
	} else {
		Index::from_path(std::path::absolute(src)?, &task.counter)?
	};
	task.set_done();
	print_thread.join().unwrap();
	if sha_512 {
		index.calculate_all()?;
	}
	index.save(index_path)?;
	Ok(())
}
