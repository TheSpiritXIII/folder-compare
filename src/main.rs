#![warn(clippy::pedantic)]

mod command;
mod index;
mod legacy;
mod progress;
mod util;

use std::env;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use clap::Args;
use clap::Parser;
use clap::Subcommand;

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
}

#[derive(Args, Debug)]
struct DiffSubcommand {
	/// Source path to find differences from.
	src: PathBuf,
	/// Destination path to find differences to, or the current path if not provided.
	dst: Option<PathBuf>,
}

#[derive(Args, Debug)]
struct Duplicates {
	/// Path to the index file.
	#[clap(long)]
	index_file: PathBuf,

	/// If set, only matches duplicates whose names match causing potential false negatives but a
	/// faster evaluation.
	#[clap(long)]
	match_name: bool,

	/// If set, only matches duplicates whose metadata match causing potential false negatives but
	/// a faster evaluation.
	#[clap(long)]
	match_meta: bool,
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
			command::stats(path, subcommand.index_file.as_ref())
		}
		Command::Diff(subcommand) => {
			let dst = subcommand.dst.unwrap_or(path);
			command::diff(&subcommand.src, &dst)
		}
		Command::Duplicates(subcommand) => {
			command::duplicates(
				&subcommand.index_file,
				subcommand.match_name,
				subcommand.match_meta,
			)
		}
	}
}
