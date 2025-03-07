use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use anyhow::Context;
use anyhow::Result;

use crate::command::task::interval;
use crate::command::task::Task;
use crate::legacy;

pub fn diff(src: &PathBuf, dst: &PathBuf) -> Result<()> {
	let mut index_src = legacy::index::Index::with(src).with_context(|| {
		let path = src.to_string_lossy();
		format!("Unable to index: {path}")
	})?;
	let mut index_dst = legacy::index::Index::with(dst).with_context(|| {
		let path = dst.to_string_lossy();
		format!("Unable to index: {path}")
	})?;

	let task_src = Arc::new(Task::new());
	let task_dst = Arc::new(Task::new());

	let task_thread_src = task_src.clone();
	let task_thread_dst = task_dst.clone();
	thread::scope(|s| {
		s.spawn(|| {
			interval(
				|| task_thread_src.done() && task_thread_dst.done(),
				|| {
					let found_src = task_thread_src.counter.value();
					let found_dst = task_thread_dst.counter.value();
					let found = found_src + found_dst;
					println!("Discovered {found} entries...");
				},
			);
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
	let task_diff_copy = task_diff.clone();
	let print_thread = thread::spawn(move || {
		interval(
			|| task_diff_copy.done(),
			|| {
				let found = task_diff_copy.counter.value();
				#[allow(clippy::cast_precision_loss)]
				let percent = found as f64 / total as f64 * 100_f64;
				println!("Compared {found} ({percent:04.1}%) entries...");
			},
		);
	});

	let diff_list = index_src.diff(&index_dst, &task_diff.counter);
	if diff_list.is_empty() {
		println!("No changes");
	}
	task_diff.set_done();
	print_thread.join().unwrap();

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
