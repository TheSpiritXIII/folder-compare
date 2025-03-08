use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use anyhow::Result;

use crate::command::task::condition_delay;
use crate::command::task::Task;
use crate::index;
use crate::util::terminal::clear_line;

pub fn diff(src: &PathBuf, dst: &PathBuf) -> Result<()> {
	let mut index_src = index::Index::new();
	let mut index_dst = index::Index::new();

	let task_src = Task::new();
	let task_dst = Task::new();

	thread::scope(|s| -> io::Result<()> {
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
		let src_thread = s.spawn(|| -> io::Result<()> {
			index_src.add(src, &task_src.counter)?;
			task_src.set_done();
			Ok(())
		});

		index_dst.add(dst, &task_dst.counter)?;
		task_dst.set_done();
		src_thread.join().unwrap()?;
		Ok(())
	})?;

	let count_src = index_src.entry_count();
	let count_dst = index_dst.entry_count();
	let total = count_src + count_dst;
	println!("Found {total} total entries!");

	let task_diff = Arc::new(Task::new());
	let diff_list = thread::scope(|s| -> io::Result<_> {
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

		let diff_list = index_src.diff(&mut index_dst, |_, _| {
			task_diff.counter.inc();
		})?;
		if diff_list.is_empty() {
			println!("No changes");
		}
		task_diff.set_done();
		Ok(diff_list)
	})?;

	for diff in &diff_list {
		match diff {
			index::Diff::Added(name) => {
				println!("+ {name}");
			}
			index::Diff::Removed(name) => {
				println!("- {name}");
			}
			index::Diff::Changed(name) => {
				println!("Δ {name}");
			}
		}
	}
	Ok(())
}
