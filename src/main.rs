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
}

#[derive(Args, Debug)]
struct Stats {
	/// Path to operate on, or the current path if not provided.
	name: Option<PathBuf>,
}

fn main() {
	let cli = Cli::parse();
	let path = env::current_dir().unwrap();
	match cli.command {
		Command::Stats(command_stats) => {
			let path = command_stats.name.unwrap_or(path);
			stats(path)
		}
	}
}

fn stats(path: PathBuf) {
	let mut index = index::Index::with(&path);
	let task = Arc::new(Task {
		counter: index::AtomicProgressCounter::new(),
		done: atomic::AtomicBool::new(false),
	});

	let task_thread = task.clone();
	let print_thread = thread::spawn(move || {
		loop {
			let now = SystemTime::now();
			loop {
				if task_thread.done.load(atomic::Ordering::Relaxed) {
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
	task.done.store(true, atomic::Ordering::SeqCst);
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
