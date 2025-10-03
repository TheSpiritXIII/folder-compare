use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;

use crate::index::Allowlist;
use crate::index::ChecksumCalculator;
use crate::index::Index;
use crate::index::RootIndex;
use crate::util::display::percentage;
use crate::util::terminal::clear_line;
use crate::util::timer::CountdownTimer;

#[allow(clippy::fn_params_excessive_bools)]
pub fn duplicates(
	index_file: &PathBuf,
	dirs: bool,
	allowlist: &Allowlist,
	match_name: bool,
	match_created: bool,
	match_modified: bool,
) -> Result<()> {
	println!("Opening index file...");
	let mut index = RootIndex::open(index_file)
		.with_context(|| format!("Unable to open index: {}", index_file.display()))?;

	let duplicates = if dirs {
		println!("Comparing dirs...");
		let total = index.file_count();
		let mut current = 0usize;
		let mut countdown = CountdownTimer::new(Duration::from_secs(1));

		let sub_index = &mut index.all_mut();
		let mut calculator = ChecksumCalculator::with_dir_match(
			sub_index,
			allowlist,
			match_name,
			match_created,
			match_modified,
		);
		while let Some(file) = calculator.next() {
			let path = file?.meta.path();
			current += 1;
			if countdown.passed() {
				let percent = percentage(current, total);
				clear_line();
				print!("Processed {current} of {total} entries ({percent})...: {path}");
				io::stdout().flush().unwrap();
			}
		}

		clear_line();
		println!("Gathering duplicates...");
		index.duplicate_dirs(allowlist)
	} else {
		println!("Comparing files...");
		let total = index.file_count();
		let mut current = 0usize;
		let mut countdown = CountdownTimer::new(Duration::from_secs(1));

		let sub_index = &mut index.all_mut();
		let mut calculator = ChecksumCalculator::with_file_match(
			sub_index,
			allowlist,
			match_name,
			match_created,
			match_modified,
		);
		while let Some(file) = calculator.next() {
			let path = file?.meta.path();

			// TODO: Add check-pointing for long-running operations.
			current += 1;
			if countdown.passed() {
				let percent = percentage(current, total);
				clear_line();
				print!("Processed {current} of {total} entries ({percent})...: {path}");
				io::stdout().flush().unwrap();
			}
		}

		clear_line();
		println!("Gathering duplicates...");
		index.duplicates(allowlist)
	};

	if duplicates.is_empty() {
		println!("No duplicates found");
	} else {
		for (i, file_list) in duplicates.iter().enumerate() {
			println!("Duplicate group {i}:");
			for file in file_list {
				println!("- {file}");
			}
		}
	}

	if index.dirty() {
		println!("Updating index with checksums...");
		index.save(index_file)?;
	}

	Ok(())
}
