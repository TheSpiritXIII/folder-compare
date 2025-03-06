#![warn(clippy::pedantic)]

mod index;
mod index2;
mod progress;

use std::env;
use std::path::PathBuf;
use std::sync::atomic;
use std::sync::Arc;
use std::thread;
use std::time::SystemTime;

use anyhow::Context;
use anyhow::Result;
use clap::Args;
use clap::Parser;
use clap::Subcommand;

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
	/// Indexes the given path.
	Index(Update),
	/// Show folder statistics.
	Stats(Stats),
	/// Find differences in two folders.
	Diff(Diff),
}

#[derive(Args, Debug)]
struct Update {
	/// Source path to index.
	src: PathBuf,

	/// Path to store the index.
	index_path: PathBuf,

	/// Whether to calculate the SHA-512 of the source files.
	#[clap(long)]
	sha_512: bool,
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

fn main() -> Result<()> {
	let cli = Cli::parse();
	let path = env::current_dir().context("Unable to retrieve the current directory")?;
	match cli.command {
		Command::Index(command) => update(&command),
		Command::Stats(command) => {
			let path = command.name.unwrap_or(path);
			stats(&path)
		}
		Command::Diff(command) => {
			let dst = command.dst.unwrap_or(path);
			diff(&command.src, &dst)
		}
	}
}

fn update(command: &Update) -> Result<()> {
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

	let mut index = if command.index_path.exists() {
		let mut index = index2::Index::open(&command.index_path).with_context(|| {
			let path = command.index_path.to_string_lossy();
			format!("Unable to open index: {path}")
		})?;
		index.add(std::path::absolute(&command.src)?, &task.counter)?;
		index
	} else {
		index2::Index::from_path(std::path::absolute(&command.src)?, &task.counter)?
	};
	task.set_done();
	print_thread.join().unwrap();
	if command.sha_512 {
		index.calculate_all()?;
	}
	index.save(&command.index_path)?;
	Ok(())
}

fn stats(path: &PathBuf) -> Result<()> {
	let mut index = index::Index::with(path).with_context(|| {
		let path = path.to_string_lossy();
		format!("Unable to index: {path}")
	})?;
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
	index.expand_all(&task.counter);
	task.set_done();
	print_thread.join().unwrap();

	let count = index.entry_count();
	println!("Found {count} total entries!");
	let file_count = index.file_count();
	println!("{file_count} files.");
	let dir_count = count - file_count;
	println!("{dir_count} directories.");
	Ok(())
}

/// Runs the given `run_fn` every second, as long as `is_done_fn` is false. It is possible that
/// `run_fn` never runs if `is_done_fn` is true.
fn interval(is_done_fn: impl Fn() -> bool, run_fn: impl Fn()) {
	loop {
		let now = SystemTime::now();
		loop {
			if is_done_fn() {
				return;
			}
			// thread::sleep(Duration::from_secs(1));
			if now.elapsed().unwrap().as_secs() >= 1 {
				break;
			}
			thread::yield_now();
		}
		run_fn();
	}
}

struct Task {
	counter: progress::AtomicProgressCounter,
	done: atomic::AtomicBool,
}

impl Task {
	fn new() -> Self {
		Self {
			counter: progress::AtomicProgressCounter::new(),
			done: atomic::AtomicBool::new(false),
		}
	}

	fn done(&self) -> bool {
		self.done.load(atomic::Ordering::Relaxed)
	}

	fn set_done(&self) {
		self.done.store(true, atomic::Ordering::SeqCst);
	}
}

fn diff(src: &PathBuf, dst: &PathBuf) -> Result<()> {
	let mut index_src = index::Index::with(src).with_context(|| {
		let path = src.to_string_lossy();
		format!("Unable to index: {path}")
	})?;
	let mut index_dst = index::Index::with(dst).with_context(|| {
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
