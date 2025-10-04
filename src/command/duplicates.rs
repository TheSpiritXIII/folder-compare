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

	let total = index.file_count();
	let sub_index = &mut index.all_mut();
	let mut calculator = if dirs {
		println!("Comparing dirs...");
		ChecksumCalculator::with_dir_match(
			sub_index,
			allowlist,
			match_name,
			match_created,
			match_modified,
		)
	} else {
		println!("Comparing files...");
		ChecksumCalculator::with_file_match(
			sub_index,
			allowlist,
			match_name,
			match_created,
			match_modified,
		)
	};

	let mut current = 0;
	let mut render_countdown = CountdownTimer::new(Duration::from_secs(1));
	let mut snapshotting_countdown = CountdownTimer::new(Duration::from_secs(60));

	while let Some(file) = calculator.next() {
		let path = file?.meta.path();
		current += 1;
		if render_countdown.passed() {
			let percent = percentage(current, total);
			clear_line();
			print!("Processed {current} of {total} entries ({percent})...: {path}");
			io::stdout().flush().unwrap();
		}
		if snapshotting_countdown.passed() && calculator.index_mut().root_mut().dirty() {
			clear_line();
			println!("Snapshotting index...");
			calculator.index_mut().root_mut().save(index_file)?;
		}
	}

	clear_line();
	println!("Gathering duplicates...");

	let duplicates: Vec<_> = if dirs {
		index
			.duplicate_dirs(allowlist)
			.iter()
			.map(|dir_list| dir_list.iter().map(|dir| dir.meta.path()).collect::<Vec<_>>())
			.collect()
	} else {
		index
			.duplicates(allowlist)
			.iter()
			.map(|file_list| file_list.iter().map(|file| file.meta.path()).collect())
			.collect()
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
