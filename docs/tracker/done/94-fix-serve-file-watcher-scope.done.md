# Issue 94: Fix serve file watcher -- only watch the site directory

## Problem

When running `cargo run --release -- serve --source datatalksclub.github.io` from the rustkyl project directory, the file watcher detects changes to files in the current directory (Cargo.toml, src/*.rs, etc.) and triggers unnecessary site rebuilds. It should only watch the `--source` directory, not the directory where the binary is invoked from.

Additionally, there is no way to disable the file watcher while keeping the HTTP server running (e.g., for production-like local previews or CI). The `--no-livereload` flag disables both the WebSocket server AND the watcher, but sometimes you want to serve with no watcher and no live reload overhead.

## Goal

The file watcher in serve mode must only watch files inside the source directory (the Jekyll site), not the directory where the binary is invoked from. A `--no-watch` flag must allow disabling file watching entirely while still serving the site.

## Dependencies

- None. The serve command and livereload module already exist.

## Current State of the Code

- `src/livereload.rs` contains `start_file_watcher()`, `should_watch_file()`, and `is_in_destination()`.
- `start_file_watcher()` already receives the `source` path and calls `debouncer.watcher().watch(&source, ...)`, so in theory it watches only the source dir.
- However, `should_watch_file()` does NOT verify that the event path is actually inside `source` -- it only checks file extension and ignores `.git/`, `node_modules/`, swap files, etc.
- The `is_in_destination()` check filters out destination dir events, but there is no equivalent "is in source" check.
- The Serve CLI struct in `src/main.rs` has `--livereload` / `--no-livereload` but no `--no-watch` flag.

## Scope

1. Add a `--no-watch` CLI flag to the `Serve` command that disables file watching (build once and serve, no rebuilds, no WebSocket server needed).
2. In `start_file_watcher()`, add an explicit check that each event path is actually inside the source directory (canonicalized comparison), rejecting any events for paths outside source.
3. Ensure `.git/`, `.github/`, and other dotfile directories inside the source are still ignored.
4. Ensure the `_site/` (destination) directory is still ignored even if it is inside the source directory.

## Acceptance Criteria

- [ ] `rustkyll serve --no-watch --source <dir>` builds the site once and serves it without starting a file watcher or WebSocket server
- [ ] `rustkyll serve --no-watch` is parseable by clap (unit test on CLI parsing)
- [ ] `start_file_watcher()` only processes events whose paths are within the source directory (canonicalized)
- [ ] `should_watch_file()` or a new helper function `is_in_source(path, source)` returns false for paths outside source
- [ ] Changes to files outside `--source` do not trigger rebuilds (e.g., `Cargo.toml`, `src/main.rs` when source is `datatalksclub.github.io/`)
- [ ] Changes to files inside `--source` (`.md`, `.html`, `.yml`, `.css`, `.js`, etc.) still trigger rebuilds
- [ ] Changes to `_site/` output directory do not trigger rebuilds (infinite loop prevention) -- this already works, but must not regress
- [ ] Changes to `.git/` and `.github/` directories inside source do not trigger rebuilds -- this already works, but must not regress
- [ ] `--no-watch` and `--no-livereload` are independent flags: `--no-watch` disables watching but could still allow livereload injection if desired (though in practice, without a watcher there is nothing to trigger reloads, so the WebSocket server can be skipped too)
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes -- all existing tests continue to pass
- [ ] `cargo clippy -- -D warnings` passes

## Test Scenarios

### Unit: CLI parsing
- Parse `rustkyll serve --no-watch` and verify the `no_watch` flag is true
- Parse `rustkyll serve` and verify the `no_watch` flag defaults to false
- Parse `rustkyll serve --no-watch --no-livereload` and verify both flags are true
- Parse `rustkyll serve --no-watch --source /tmp/site` and verify both fields are correct

### Unit: Source directory filtering
- `is_in_source(Path::new("/project/site/post.md"), Path::new("/project/site"))` returns true
- `is_in_source(Path::new("/project/Cargo.toml"), Path::new("/project/site"))` returns false
- `is_in_source(Path::new("/other/dir/file.md"), Path::new("/project/site"))` returns false
- Paths that are inside source but also inside destination return false (existing `is_in_destination` check)
- Paths inside source's `.git/` directory return false (existing `should_watch_file` check)

### Unit: Event filtering in start_file_watcher
- An event for a file inside source that passes `should_watch_file` is treated as relevant
- An event for a file outside source is filtered out even if it passes `should_watch_file`
- An event for a file inside destination (which is inside source) is filtered out

### Integration: --no-watch flag behavior
- When `--no-watch` is set, the serve command does NOT spawn a file watcher thread (verify by code path, not by timing)
- When `--no-watch` is set, the serve command does NOT start a WebSocket server

## Implementation Notes

- The `is_in_source()` helper should use `canonicalize()` on both paths (with a fallback to `starts_with` if canonicalization fails), similar to the existing `is_in_destination()` function.
- In `start_file_watcher()`, add the source-containment check to the filter on line 217-219 of `livereload.rs`.
- In `main.rs`, add a `no_watch: bool` field to the `Serve` variant and use it to decide whether to spawn the watcher/WebSocket threads.

## Log

### [SWE] 2026-03-15
- Implemented both fixes from the issue spec
- Fix 1: Added `--no-watch` CLI flag to Serve command (default false). When set, `livereload_enabled` is false, so no watcher thread or WebSocket server is started -- just build once and serve.
- Fix 2: Added `is_in_source(path, source)` function in livereload.rs, modeled after `is_in_destination()`. Uses `canonicalize()` with fallback to `starts_with`. Added the check to the event filter in `start_file_watcher()` so only events for paths inside the source directory trigger rebuilds.
- Tests added: 6 new unit tests for `is_in_source()` in livereload.rs, 4 new CLI parsing tests for `--no-watch` in main.rs
- Build: all 1021+ tests pass, 0 failures, clippy clean, fmt clean
- Files modified: src/livereload.rs, src/main.rs
- Files renamed: docs/tracker/94-fix-serve-file-watcher-scope.groomed.md -> .in-progress.md

### [PM] 2026-03-15
- ACCEPTED. All 12 acceptance criteria verified.
- `is_in_source()` correctly implemented with canonicalize + fallback, mirrors existing `is_in_destination()` pattern.
- `--no-watch` flag properly disables both watcher and WebSocket server via `livereload_enabled` logic.
- 10 new tests (6 unit for is_in_source, 4 CLI parsing) -- all meaningful, use real filesystem with tempdir.
- No regressions: existing destination/dotfile filtering preserved, all 1021+ tests pass, clippy clean.
- No descoped items.
