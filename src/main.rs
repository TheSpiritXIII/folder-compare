use std::sync::atomic;
use std::sync::Arc;
use std::thread;
use std::time::SystemTime;

use clap::Parser;

mod index;

/// Utility to compare folder contents.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
	/// Optional path to operate on, or the current path.
	name: Option<String>,
}

fn main() {
	let cli = Cli::parse();
	let path = cli.name.unwrap_or("./".to_owned());

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
