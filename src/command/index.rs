use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;

use super::task::Delayer;
use crate::index::Index;
use crate::util::terminal::clear_line;

pub fn index(src: &PathBuf, index_file: &PathBuf, sha_512: bool) -> Result<()> {
	let mut current = 0usize;
	let mut delayer = Delayer::new(Duration::from_secs(1));
	let mut last_path = String::new();
	let update_fn = |path: &str| {
		last_path = path.to_string();
		if delayer.run() {
			clear_line();
			print!("Discovered {current} entries...");
			io::stdout().flush().unwrap();
		}
		current += 1;
	};
	let mut index = if index_file.exists() {
		let mut index = Index::open(index_file)
			.with_context(|| format!("Unable to open index: {}", index_file.display()))?;
		index.add(std::path::absolute(src)?, update_fn)?;
		index
	} else {
		Index::from_path(std::path::absolute(src)?, update_fn)?
	};

	if sha_512 {
		index.calculate_all()?;
	}
	index.save(index_file)?;
	Ok(())
}
