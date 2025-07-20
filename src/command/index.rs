use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;

use crate::index::RootIndex;
use crate::util::terminal::clear_line;
use crate::util::timer::CountdownTimer;

pub fn index(src: &PathBuf, index_file: &PathBuf, sha_512: bool) -> Result<()> {
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

	let mut index = if index_file.exists() {
		println!("Opening index file...");
		let mut index = RootIndex::open(index_file)
			.with_context(|| format!("Unable to open index: {}", index_file.display()))?;
		println!("Updating index file...");
		index.add(std::path::absolute(src)?, update_fn)?;
		index
	} else {
		println!("Reading files...");
		RootIndex::from_path(std::path::absolute(src)?, update_fn)?
	};
	clear_line();
	println!("Discovered {current} total entries!");

	if sha_512 {
		println!("Updating checksums...");
		index.calculate_all()?;
	}
	println!("Saving index file...");
	index.save(index_file)?;
	Ok(())
}
