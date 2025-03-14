use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use regex::Regex;

use crate::index;
use crate::util::display::percentage;
use crate::util::terminal::clear_line;
use crate::util::timer::CountdownTimer;

pub fn duplicates(
	index_file: &PathBuf,
	filter: Option<&Regex>,
	match_name: bool,
	match_created: bool,
	match_modified: bool,
) -> Result<()> {
	println!("Opening index file...");
	let mut index = index::Index::open(index_file)
		.with_context(|| format!("Unable to open index: {}", index_file.display()))?;

	println!("Comparing files...");
	let total = index.file_count();
	let mut current = 0usize;
	let mut countdown = CountdownTimer::new(Duration::from_secs(1));
	let mut last_path = String::new();
	index
		.calculate_matches(
			|path| {
				last_path = path.to_string();
				current += 1;
				if countdown.passed() {
					clear_line();
					let percent = percentage(current, total);
					print!("Processed {current} of {total} entries ({percent}%)...: {path}");
					io::stdout().flush().unwrap();
				}
			},
			filter,
			match_name,
			match_created,
			match_modified,
		)
		.with_context(|| format!("Comparison failed during file: {last_path}"))?;

	println!("Gathering duplicates...");
	let duplicates = index.duplicates(filter);
	if duplicates.is_empty() {
		println!("No duplicates found");
	} else {
		for file_list in duplicates {
			println!("Duplicate: {file_list:?}");
		}
	}

	if index.dirty() {
		println!("Updating index with checksums...");
		index.save(index_file)?;
	}

	Ok(())
}
