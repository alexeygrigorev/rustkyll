# Issue 92: Integration tests for build progress output (stderr/stdout verification)

## Problem

Issue 91 implemented build progress output but several test scenarios from the spec were descoped because they require subprocess-based integration testing (running the binary and capturing stderr/stdout), which is non-trivial and not the pattern currently used in this codebase.

The core functionality is tested via unit tests on the `ProgressReporter`/`RenderProgress` abstractions, but end-to-end verification of actual output streams is missing.

## Goal

Add integration tests that run the `rustkyll` binary as a subprocess and verify:
1. stderr contains phase indicator strings in normal mode
2. stderr is empty in quiet mode (no progress output for a clean build)
3. stdout does NOT contain progress bar or phase indicator strings (only the final summary)
4. Progress count in output matches a known number of pages
5. When stderr is redirected to a file, output contains no ANSI escape codes or carriage returns

## Dependencies

- Issue 91 (build progress output) -- done

## Acceptance Criteria

- [ ] `cargo test` passes with new integration tests
- [ ] Test that runs `rustkyll build` on a minimal site and captures stderr; asserts stderr contains "Loading config", "Rendering", "Copying static files"
- [ ] Test that runs `rustkyll build --quiet` on a minimal site and captures stderr; asserts stderr is empty
- [ ] Test that runs `rustkyll build` and captures stdout; asserts stdout does NOT contain phase indicator strings
- [ ] Test that builds a site with a known number of pages (e.g., 3 posts + 2 standalone); verifies the progress output mentions the correct total
- [ ] Test that runs the build with stderr piped (non-TTY); verifies no ANSI escape codes (\x1b[) in stderr output

## Test Scenarios

All tests should use `std::process::Command` to run the built binary, set up a temporary site directory, and capture stdout/stderr separately.

### Integration: stderr contains progress in normal mode
- Create minimal site with _config.yml, _layouts/page.html, index.md
- Run `rustkyll build --source <dir> --destination <dir>/_site`
- Assert stderr contains "Loading config"
- Assert stderr contains "Rendering"

### Integration: stderr empty in quiet mode
- Same minimal site
- Run `rustkyll build --quiet --source <dir> --destination <dir>/_site`
- Assert stderr is empty (length 0)

### Integration: stdout clean of progress
- Same minimal site, normal mode
- Assert stdout does NOT contain "Loading config" or progress bar characters

### Integration: progress count accuracy
- Create site with _posts collection (3 posts) + 2 standalone pages
- Run build, capture stderr
- Assert stderr mentions the total count (5 or similar)

### Integration: non-TTY no ANSI codes
- Run build with stderr captured (which is inherently non-TTY)
- Assert stderr does not contain \x1b[ sequences

## Log

### [SWE] 2026-03-15
- Created `tests/integration_progress_output.rs` with 5 subprocess-based integration tests
- Tests use `std::process::Command` with `env!("CARGO_BIN_EXE_rustkyll")` and temp directories
- Key finding: in non-TTY mode, only `phase_done()` messages appear (not `phase()`), so tests check for "Loading data files", "Loading collections", etc. instead of "Loading config" (which only has a `phase()` call)
- Tests:
  1. `progress_stderr_contains_phase_messages_in_normal_mode` -- verifies phase_done messages on stderr
  2. `progress_stderr_empty_in_quiet_mode` -- verifies --quiet suppresses all stderr
  3. `progress_stdout_does_not_contain_phase_indicators` -- verifies stdout has only summary
  4. `progress_count_matches_known_page_count` -- creates 3 posts + 2 pages, verifies "Total pages: 5"
  5. `progress_non_tty_no_ansi_escape_codes` -- verifies no \x1b[ or \r in piped stderr
- Build: 5 tests pass, 0 fail, clippy clean, fmt clean
- Files created: tests/integration_progress_output.rs
