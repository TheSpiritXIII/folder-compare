use std::env;
use std::path::PathBuf;
use std::sync::atomic;
use std::sync::Arc;
use std::thread;
use std::time::SystemTime;

use clap::Args;
use clap::Parser;
use clap::Subcommand;

mod index;

/// Utility to compare folder contents.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Command,
}

/// Doc comment
#[derive(Subcommand, Debug)]
enum Command {
	/// Show folder statistics.
	Stats(Stats),
	/// Find differences in two folders.
	Diff(Diff),
}

#[derive(Args, Debug)]
struct Stats {
	/// Path to operate on, or the current path if not provided.
	name: Option<PathBuf>,
}

#[derive(Args, Debug)]
struct Diff {
	/// Source path to find differences from.
	src: PathBuf,
	/// Destination path to find differences to, or the current path if not provided.
	dst: Option<PathBuf>,
}

fn main() {
	let cli = Cli::parse();
	let path = env::current_dir().unwrap();
	match cli.command {
		Command::Stats(command_stats) => {
			let path = command_stats.name.unwrap_or(path);
			stats(path)
		}
		Command::Diff(command_diff) => {
			let dst = command_diff.dst.unwrap_or(path);
			diff(command_diff.src, dst)
		}
	}
}

fn stats(path: PathBuf) {
	let mut index = index::Index::with(&path);
	let task = Arc::new(Task::new());

	let task_thread = task.clone();
	let print_thread = thread::spawn(move || {
		loop {
			let now = SystemTime::now();
			loop {
				if task_thread.done() {
					return;
				}
				if now.elapsed().unwrap().as_secs() >= 1 {
					break;
				}
				thread::yield_now();
			}
			let found = task_thread.counter.value();
			println!("Discovered {found} entries...");
		}
	});
	index.expand_all(&task.counter);
	task.set_done();
	print_thread.join().unwrap();

	let count = index.entry_count();
	println!("Found {count} total entries!");
	let file_count = index.file_count();
	println!("{file_count} files.");
	let dir_count = count - file_count;
	println!("{dir_count} directories.");
}

struct Task {
	counter: index::AtomicProgressCounter,
	done: atomic::AtomicBool,
}

impl Task {
	fn new() -> Self {
		Self {
			counter: index::AtomicProgressCounter::new(),
			done: atomic::AtomicBool::new(false),
		}
	}

	fn done(&self) -> bool {
		self.done.load(atomic::Ordering::Relaxed)
	}

	fn set_done(&self) {
		self.done.store(true, atomic::Ordering::SeqCst)
	}
}

fn diff(src: PathBuf, dst: PathBuf) {
	let mut index_src = index::Index::with(&src);
	let mut index_dst = index::Index::with(&dst);

	let task_src = Arc::new(Task::new());
	let task_dst = Arc::new(Task::new());

	let task_thread_src = task_src.clone();
	let task_thread_dst = task_dst.clone();
	thread::scope(|s| {
		s.spawn(|| {
			loop {
				let now = SystemTime::now();
				loop {
					if task_thread_src.done() && task_thread_dst.done() {
						return;
					}
					if now.elapsed().unwrap().as_secs() >= 1 {
						break;
					}
					thread::yield_now();
				}
				let found_src = task_thread_src.counter.value();
				let found_dst = task_thread_dst.counter.value();
				let found = found_src + found_dst;
				println!("Discovered {found} entries...");
			}
		});
		s.spawn(|| {
			index_src.expand_all(&task_src.counter);
			task_src.set_done()
		});

		index_dst.expand_all(&task_dst.counter);
		task_dst.set_done()
	});

	let count_src = index_src.entry_count();
	let count_dst = index_dst.entry_count();
	println!("Found {count_src} vs {count_dst} total entries!");

	let diff_list = index_src.diff(&index_dst);
	if diff_list.is_empty() {
		println!("No changes");
	}

	for diff in &diff_list {
		match diff {
			index::Diff::Added(name) => {
				println!("+ {name}")
			}
			index::Diff::Removed(name) => {
				println!("- {name}")
			}
			index::Diff::Changed(name) => {
				println!("Δ {name}")
			}
		}
	}
}
