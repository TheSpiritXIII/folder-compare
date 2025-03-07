#![warn(clippy::pedantic)]

mod command;

mod index;
mod legacy;
mod matches;
mod progress;

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
	Index(Update),
	/// Show folder statistics.
	Stats(Stats),
	/// Find differences in two folders.
	Diff(Diff),
	/// Finds duplicates in a folder.
	Duplicates(Duplicates),
}

#[derive(Args, Debug)]
struct Update {
	/// Source path to index.
	src: PathBuf,

	/// Path to store the index.
	index_path: PathBuf,

	/// Whether to calculate the SHA-512 of the source files.
	#[clap(long)]
	sha_512: bool,
}

#[derive(Args, Debug)]
struct Stats {
	/// Path to operate on, or the current path if not provided.
	name: Option<PathBuf>,
}

#[derive(Args, Debug)]
struct Diff {
	/// Source path to find differences from.
	src: PathBuf,
	/// Destination path to find differences to, or the current path if not provided.
	dst: Option<PathBuf>,
}

#[derive(Args, Debug)]
struct Duplicates {
	#[clap(long)]
	index_file: PathBuf,
}

fn main() -> Result<()> {
	let cli = Cli::parse();
	let path = env::current_dir().context("Unable to retrieve the current directory")?;
	match cli.command {
		Command::Index(command) => {
			command::update(&command.src, &command.index_path, command.sha_512)
		}
		Command::Stats(command) => {
			let path = command.name.unwrap_or(path);
			command::stats(&path)
		}
		Command::Diff(command) => {
			let dst = command.dst.unwrap_or(path);
			command::diff(&command.src, &dst)
		}
		Command::Duplicates(command) => command::duplicates(&command.index_file),
	}
}
