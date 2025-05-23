#![warn(clippy::pedantic)]

mod command;
mod store;
mod util;

use std::env;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use clap::Args;
use clap::Parser;
use clap::Subcommand;
use regex::Regex;
use store::Allowlist;

/// Utility to compare folder contents.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Command,
}

/// Doc comment
#[derive(Subcommand, Debug)]
enum Command {
	/// Indexes the given path.
	Index(IndexSubcommand),
	/// Show folder statistics.
	Stats(StatsSubcommand),
	/// Find differences in two folders.
	Diff(DiffSubcommand),
	/// Finds duplicates in a folder.
	Duplicates(Duplicates),
}

#[derive(Args, Debug)]
struct IndexSubcommand {
	/// Source path to index.
	src: PathBuf,

	/// Path to store the index.
	#[clap(long)]
	index_file: PathBuf,

	/// Whether to calculate the SHA-512 of the source files.
	#[clap(long)]
	sha_512: bool,
}

#[derive(Args, Debug)]
struct StatsSubcommand {
	/// Path to operate on, or the current path if this and `--index_file` are not provided.
	src: Option<PathBuf>,

	/// Path to the index file to check the stats for.
	#[clap(long)]
	index_file: Option<PathBuf>,

	/// Source dir to find stats for, or all indexed directories if not provided.
	#[clap(long)]
	dir: Option<PathBuf>,
}

#[derive(Args, Debug)]
struct DiffSubcommand {
	/// Source path to find differences from.
	src: PathBuf,

	/// Path to the index file to compare to.
	#[clap(long)]
	index_file: PathBuf,

	#[command(flatten)]
	matches: Matches,
}

#[derive(Args, Debug)]
struct Duplicates {
	/// Path to the index file.
	#[clap(long)]
	index_file: PathBuf,

	#[command(flatten)]
	filter: Filter,

	/// Finds duplicate dirs. If unset, finds duplicate files instead.
	#[clap(long)]
	dirs: bool,

	#[command(flatten)]
	matches: Matches,
}

#[derive(Args, Debug)]
struct Matches {
	/// If set, matches names, causing potential false negatives but a faster evaluation.
	#[clap(long = "match-name")]
	name: bool,

	/// If set, matches created times, causing potential false negatives but a faster evaluation.
	/// Note: On Windows, created times are updated when duplicating files.
	#[clap(long = "match-created")]
	created: bool,

	/// If set, matches modified times, causing potential false negatives but a faster evaluation.
	#[clap(long = "match-modified")]
	modified: bool,
}

#[derive(Args, Debug)]
struct Filter {
	/// Regular expression for expressing the paths to keep. Multiple allowlists are considered an
	/// "OR".
	#[clap(long)]
	allow: Vec<Regex>,

	/// Regular expression for expressing the paths to ignore. Multiple denylists are considered an
	/// "OR".
	#[clap(long)]
	deny: Vec<Regex>,
}

fn main() -> Result<()> {
	let cli = Cli::parse();
	let path = env::current_dir().context("Unable to retrieve the current directory")?;
	match cli.command {
		Command::Index(subcommand) => {
			command::index(&subcommand.src, &subcommand.index_file, subcommand.sha_512)
		}
		Command::Stats(subcommand) => {
			let path = if subcommand.index_file.is_some() {
				subcommand.src.as_ref()
			} else {
				subcommand.src.as_ref().or(Some(&path))
			};
			command::stats(path, subcommand.index_file.as_ref(), subcommand.dir.as_ref())
		}
		Command::Diff(subcommand) => {
			command::diff(
				&subcommand.src,
				&subcommand.index_file,
				subcommand.matches.name,
				subcommand.matches.created,
				subcommand.matches.modified,
			)
		}
		Command::Duplicates(subcommand) => {
			let allowlist = Allowlist {
				allow: subcommand.filter.allow,
				deny: subcommand.filter.deny,
			};
			command::duplicates(
				&subcommand.index_file,
				subcommand.dirs,
				&allowlist,
				subcommand.matches.name,
				subcommand.matches.created,
				subcommand.matches.modified,
			)
		}
	}
}
