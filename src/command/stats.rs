use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::bail;
use anyhow::Context;
use anyhow::Result;

use crate::index::Index;
use crate::util::terminal::clear_line;
use crate::util::timer::CountdownTimer;

pub fn stats(
	src: Option<&PathBuf>,
	index_file: Option<&PathBuf>,
	dir: Option<&PathBuf>,
) -> Result<()> {
	let mut current = 0usize;
	let mut countdown = CountdownTimer::new(Duration::from_secs(1));
	let mut last_path = String::new();
	let update_fn = |path: &str| {
		last_path = path.to_string();
		if countdown.passed() {
			clear_line();
			print!("Discovered {current} entries: {path}");
			io::stdout().flush().unwrap();
		}
		current += 1;
	};

	let index = if let Some(path) = index_file {
		let mut index = Index::open(path)
			.with_context(|| format!("Unable to open index: {}", path.display()))?;
		if let Some(path) = src {
			index.add(std::path::absolute(path)?, update_fn)?;
		}
		index
	} else if let Some(path) = src {
		Index::from_path(std::path::absolute(path)?, update_fn)?
	} else {
		bail!("Expected source or index-file");
	};

	clear_line();
	let sub_index = if let Some(dir) = dir {
		let Some(sub_index) = index.sub_index(dir) else {
			bail!("Invalid directory");
		};
		sub_index
	} else {
		index.all()
	};
	let count = sub_index.entry_count();
	println!("Found {count} total entries!");
	let file_count = sub_index.file_count();
	println!("{file_count} files.");
	let dir_count = sub_index.dir_count();
	println!("{dir_count} directories.");
	let dir_count = sub_index.file_size();
	println!("{dir_count} bytes.");

	if let Some(path) = index_file {
		if index.dirty() {
			println!("Saving index...");
			index.save(path)?;
		}
	}
	Ok(())
}
