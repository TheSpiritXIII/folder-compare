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
	index.expand_all();
	let count = index.entry_count();
	println!("Found {count} total entries!");
	let file_count = index.file_count();
	println!("{file_count} files.");
	let dir_count = count - file_count;
	println!("{dir_count} directories.");
}
