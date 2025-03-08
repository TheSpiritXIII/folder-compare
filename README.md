# folder-compare

Command-line utility to compare folder contents. Hypothetically, it is cross-platform but only tested on Windows.

## Usage

### Basic

First, clone this repository:

```bash
git clone https://github.com/TheSpiritXIII/folder-compare.git
cd folder-compare
```

This tool uses a file index to perform operations. First, you must build an index from a path:

```bash
cargo run -- index "path/to/a" --index-file="index.ron"
```

You can view stats from your new index file. Note, you will not see changes:

```bash
cargo run -- stats --index-file="index.ron"
```

For a full list of sub-commands and arguments, use `--help`:

```bash
cargo run -- --help
```

### Advanced

If you make changes in the original folder, you can compare against it:

```bash
cargo run -- diff "path/to/a" --index-file="index.ron"
```

You can even find duplicate files:

```bash
cargo run -- duplicates --index-file="index.ron"
```

> [!WARNING]
> Calculating duplicates is expensive. This tool calculates checksums for each potential duplicate. Avoid using this frequently on large folders, as this might cause wear on an SSD.

If you're willing to except a few missing duplicates for faster comparison, you can match names and modified times:

```bash
cargo run -- duplicates --index-file="index.ron" --match-names --match-modified
```

## Contributions

Please create an issue if you have any feature requests!
