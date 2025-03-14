use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;

use crate::command::task::Delayer;
use crate::index::Diff;
use crate::index::Index;
use crate::util::display::percentage;
use crate::util::terminal::clear_line;

pub fn diff(src: &PathBuf, index_file: &PathBuf) -> Result<()> {
	let mut index_src = Index::new();
	let mut index_dst = Index::new();

	thread::scope(|s| -> io::Result<()> {
		let src_thread = s.spawn(|| -> io::Result<()> {
			let mut current = 0usize;
			let mut delayer = Delayer::new(Duration::from_secs(1));
			let mut last_path = String::new();
			index_src.add(std::path::absolute(src)?, |path| {
				last_path = path.to_string();
				if delayer.run() {
					clear_line();
					print!("Discovered {current} entries: {path}");
					io::stdout().flush().unwrap();
				}
				current += 1;
			})?;
			clear_line();
			print!("Loading index file...");
			io::stdout().flush().unwrap();
			Ok(())
		});

		index_dst = Index::open(index_file)?;
		src_thread.join().unwrap()?;

		clear_line();
		io::stdout().flush().unwrap();
		Ok(())
	})?;

	let count_src = index_src.entry_count();
	let count_dst = index_dst.entry_count();
	let total = count_src + count_dst;
	println!("Found {total} total entries!");

	let mut current = 1usize;
	let mut delayer = Delayer::new(Duration::from_secs(1));
	let mut last_rhs = String::new();
	let mut last_lhs = String::new();
	let diff_list = index_src
		.diff(&mut index_dst, |lhs, rhs| {
			current += 1;
			last_rhs = rhs.to_string();
			last_lhs = lhs.to_string();
			if delayer.run() {
				clear_line();
				let percent = percentage(current, total);
				print!("Comparing {rhs} vs {lhs} ({percent}))...");
				io::stdout().flush().unwrap();
			}
		})
		.with_context(|| format!("Comparison failed during {last_rhs} and {last_lhs}"))?;
	if diff_list.is_empty() {
		println!("No changes");
	}

	for diff in &diff_list {
		match diff {
			Diff::Added(name) => {
				println!("+ {name}");
			}
			Diff::Removed(name) => {
				println!("- {name}");
			}
			Diff::Changed(name) => {
				println!("Δ {name}");
			}
		}
	}
	Ok(())
}
