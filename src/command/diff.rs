use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use anyhow::Context;
use anyhow::Result;

use crate::command::task::condition_delay;
use crate::command::task::Task;
use crate::legacy;
use crate::util::terminal::clear_line;

pub fn diff(src: &PathBuf, dst: &PathBuf) -> Result<()> {
	let mut index_src = legacy::index::Index::with(src)
		.with_context(|| format!("Unable to index: {}", src.display()))?;
	let mut index_dst = legacy::index::Index::with(dst)
		.with_context(|| format!("Unable to index: {}", dst.display()))?;

	let task_src = Task::new();
	let task_dst = Task::new();

	thread::scope(|s| {
		s.spawn(|| {
			loop {
				if condition_delay(|| task_src.done() && task_dst.done()) {
					return;
				}
				let found_src = task_src.counter.value();
				let found_dst = task_dst.counter.value();
				let found = found_src + found_dst;
				clear_line();
				print!("Discovered {found} entries...");
				io::stdout().flush().unwrap();
			}
		});
		s.spawn(|| {
			index_src.expand_all(&task_src.counter);
			task_src.set_done();
		});

		index_dst.expand_all(&task_dst.counter);
		task_dst.set_done();
	});

	let count_src = index_src.entry_count();
	let count_dst = index_dst.entry_count();
	let total = count_src + count_dst;
	println!("Found {total} total entries!");

	let task_diff = Arc::new(Task::new());
	let diff_list = thread::scope(|s| {
		s.spawn(|| {
			loop {
				if condition_delay(|| task_diff.done()) {
					return;
				}
				let found = task_diff.counter.value();
				#[allow(clippy::cast_precision_loss)]
				let percent = found as f64 / total as f64 * 100_f64;
				clear_line();
				print!("Compared {found} ({percent:04.1}%) entries...");
				io::stdout().flush().unwrap();
			}
		});

		let diff_list = index_src.diff(&index_dst, &task_diff.counter);
		if diff_list.is_empty() {
			println!("No changes");
		}
		task_diff.set_done();
		diff_list
	});

	for diff in &diff_list {
		match diff {
			legacy::index::Diff::Added(name) => {
				println!("+ {name}");
			}
			legacy::index::Diff::Removed(name) => {
				println!("- {name}");
			}
			legacy::index::Diff::Changed(name) => {
				println!("Δ {name}");
			}
		}
	}
	Ok(())
}
