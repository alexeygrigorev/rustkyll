# Issue 94: Fix serve file watcher — only watch the site directory

## Problem

When running `cargo run --release -- serve --source datatalksclub.github.io` from the rustkyl project directory, the file watcher detects changes to files in the current directory (Cargo.toml, src/*.rs, etc.) and triggers unnecessary site rebuilds. It should only watch the `--source` directory, not the working directory.

## Goal

The file watcher in serve mode must only watch files inside the source directory (the Jekyll site), not the directory where the binary is invoked from.

## Acceptance criteria

- File watcher only watches the `--source` directory
- Changes to files outside `--source` do not trigger rebuilds
- Changes to files inside `--source` (markdown, layouts, includes, data, config) still trigger rebuilds
- Changes to `_site/` output directory do not trigger rebuilds (would cause infinite loop)
- Changes to dotfiles (`.git/`, `.github/`) do not trigger rebuilds
- All existing tests still pass
