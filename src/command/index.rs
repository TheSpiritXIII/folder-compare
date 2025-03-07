use std::path::PathBuf;
use std::thread;

use anyhow::Context;
use anyhow::Result;

use crate::command::task::condition_delay;
use crate::command::task::Task;
use crate::index::Index;

pub fn index(src: &PathBuf, index_file: &PathBuf, sha_512: bool) -> Result<()> {
	let task = Task::new();
	let mut index = thread::scope(|s| -> Result<Index> {
		s.spawn(|| {
			loop {
				if condition_delay(|| task.done()) {
					return;
				}
				let found = task.counter.value();
				println!("Discovered {found} entries...");
			}
		});

		let index = if index_file.exists() {
			let mut index = Index::open(index_file)
				.with_context(|| format!("Unable to open index: {}", index_file.display()))?;
			index.add(std::path::absolute(src)?, &task.counter)?;
			index
		} else {
			Index::from_path(std::path::absolute(src)?, &task.counter)?
		};
		task.set_done();
		Ok(index)
	})?;

	if sha_512 {
		index.calculate_all()?;
	}
	index.save(index_file)?;
	Ok(())
}
