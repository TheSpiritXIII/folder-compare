use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;

use crate::command::task::Delayer;
use crate::index;
use crate::util::terminal::clear_line;

pub fn duplicates(index_file: &PathBuf) -> Result<()> {
	let mut index = index::Index::open(index_file)
		.with_context(|| format!("Unable to open index: {}", index_file.display()))?;

	println!("Comparing files...");
	let total = index.file_count();
	let mut current = 0;
	let mut delayer = Delayer::new();
	index.calculate_matches(|x| {
		if delayer.run() {
			current += 1;
			clear_line();
			print!("Processed {current} of {total} entries...: {x}");
		}
	})?;

	let duplicates = index.duplicates();
	if duplicates.is_empty() {
		println!("No duplicates found");
		return Ok(());
	}

	for file_list in duplicates {
		println!("Duplicate: {file_list:?}");
	}
	Ok(())
}
