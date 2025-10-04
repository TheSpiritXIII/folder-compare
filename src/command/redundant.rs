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

pub fn redundant(
	index_file: &PathBuf,
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
	println!("Comparing files...");
	let mut calculator = ChecksumCalculator::with_file_match(
		sub_index,
		allowlist,
		match_name,
		match_created,
		match_modified,
	);

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
	let duplicates = index.duplicates(allowlist);

	if duplicates.is_empty() {
		println!("No duplicates found");
	} else {
		let files = duplicates.clone().into_iter().flatten().cloned().collect();
		// TODO: Might need to insert empty directories too.
		let duplicate_index = RootIndex::with_files(files);
		println!("Redundant files: ");
		for dir in &duplicate_index.dirs {
			let Some(sub_index_original) = index.sub_index(dir.meta.path()) else {
				continue;
			};
			let Some(sub_index_duplicate) = duplicate_index.sub_index(dir.meta.path()) else {
				continue;
			};
			if sub_index_original.matches(&sub_index_duplicate) {
				println!("- {}", dir.meta.path());
				for file in sub_index_duplicate.files {
					'duplicate_finder: for duplicate_list in &duplicates {
						for duplicate in duplicate_list {
							if file.meta.path() == duplicate.meta.path() {
								println!(
									"  - duplicate {}: {:?}",
									file.meta.path(),
									duplicate_list
										.iter()
										.map(|file| file.meta.path())
										.filter(|path| path != &file.meta.path())
										.collect::<Vec<_>>()
								);
								break 'duplicate_finder;
							}
						}
					}
				}
				for dir in sub_index_duplicate.dirs {
					println!("  - dir {}", dir.meta.path());
				}
			}
		}
	}

	if index.dirty() {
		println!("Updating index with checksums...");
		index.save(index_file)?;
	}

	Ok(())
}
