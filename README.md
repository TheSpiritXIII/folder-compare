# folder-compare

Cross-platform command-line utility to compare folder contents.

## Usage

First, clone this repository:

```bash
git clone https://github.com/TheSpiritXIII/folder-compare.git
cd folder-compare
```

To view stats about a path use the `stats` subcommand:

```bash
cargo run -- stats "some/path/here"
```

To compare two directories use the `diff` subcommand:

```bash
cargo run -- diff "path/to/a" "path/to/b"
```

For a full list of sub-commands and arguments, use `--help`:

```bash
cargo run -- --help
```

## Contributions

Please create an issue if you have any feature requests!
