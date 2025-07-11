# folder-compare

Command-line utility to compare folder contents. Hypothetically, it is cross-platform but only tested on Windows.

This tool creates file-indices, allowing you to compare changes to the index.

## Usage

### Installation

This tool is not published anywhere yet. To use it, clone this repository:

```bash
git clone https://github.com/TheSpiritXIII/folder-compare.git
cd folder-compare
```

For a full list of sub-commands and arguments, use `--help`:

```bash
cargo run -- --help
```

### Basic Demo

All examples will use this demo folder:

```bash
mkdir -p path/to/a
echo "bar" > path/to/a/foo.txt
```

First, you must build an index from a path:

```bash
cargo run -- index "path/to/a" --index-file="index.ron"
```

You can view stats from your new index file:

```bash
cargo run -- stats --index-file="index.ron"
```

You will see:

```txt
Found 2 total entries!
1 files.
1 directories.
```

Let's say we create a new file:

```bash
echo "qux" > path/to/a/baz.txt
```

If you rerun the stats command, you will not see any changes. You can view the diff:

```bash
cargo run -- diff "path/to/a" --index-file="index.ron"
```

You will see something like:

```txt
Δ path/to/a/baz.txt
```

## Advanced Demo:

Let's clone a file in the demo directory and regenerate the index:

```bash
cp -p path/to/a/foo.txt path/to/a/bar.txt
cargo run -- index "path/to/a" --index-file="index.ron"
```

You can find duplicates:

```bash
cargo run -- duplicates --index-file="index.ron"
```

This will output:

```txt
Duplicate group 1:
- path/to/a/foo.txt
- path/to/a/bar.txt
```

> [!WARNING]
> Calculating duplicates is expensive. This tool calculates checksums for each potential duplicate. Avoid using this frequently on large folders, as this might cause wear on an SSD.

If you're willing to except a few missing duplicates for faster comparison, you can match names and modified times:

```bash
cargo run -- duplicates --index-file="index.ron" --match-name --match-modified
```

This only works because we used `cp -p` which keeps modification times! If we were to create a new file:

```bash
echo "qux" > path/to/a/qux.txt
```

If you rerun the command, you will not see `qux.txt` marked as a duplicate, even though it matches `bar.txt` because the modification times do not match.

## Contributions

Please create an issue if you have any feature requests!
