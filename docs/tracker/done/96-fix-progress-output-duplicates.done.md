# Issue 96: Fix duplicate progress output lines

## Problem

Build output shows duplicate lines for each phase -- first the phase name, then the same name with counts:

```
Loading data files...
Loading data files... 6 files
Loading collections...
Loading collections... 7 collections, 777 items
Copying static files...
Copying static files... 1453 files
```

Should be a single line per phase, updated in place:

```
Loading data files... 6 files
Loading collections... 7 collections, 777 items
Copying static files... 1453 files
```

## Root Cause

In `src/progress.rs`, both `phase()` and `phase_done()` call `eprintln!()`, which always prints a new line. In `src/main.rs`, the build flow calls `phase("Loading data files...")` then later `phase_done("Loading data files... 6 files")`, producing two lines per phase.

## Goal

Each phase shows ONE line with the final count. No duplicate "starting" line. On TTY, update the line in place (print the "starting" message without a newline, then overwrite with the completed message). On non-TTY (piped output), print only once after the phase completes.

## Implementation Approach

Modify `ProgressReporter` in `src/progress.rs`:

1. **TTY mode:** `phase()` prints the message *without* a trailing newline (use `eprint!` + flush), then `phase_done()` uses `\r` (carriage return) to overwrite and prints the completed message with `eprintln!`. This gives the user a "Loading data files..." message that visually transforms into "Loading data files... 6 files" on the same line.

2. **Non-TTY mode:** `phase()` becomes a no-op (or stores the message internally). `phase_done()` prints the final line with `eprintln!`. This ensures piped/captured output contains exactly one line per phase.

3. **Quiet mode:** Both remain no-ops (already handled).

No changes needed to `src/main.rs` call sites -- the API stays the same, only the behavior of `phase()` and `phase_done()` changes.

## Dependencies

- None. Issue 91 (--quiet flag) is already done.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes (all existing tests, plus new ones below)
- [ ] `cargo clippy -- -D warnings` passes
- [ ] Each build phase appears as exactly ONE line in stderr output (not two)
- [ ] The final line for each phase includes the phase name AND the count (e.g., "Loading data files... 6 files")
- [ ] **TTY behavior:** `phase()` prints the starting message without a trailing newline (so the cursor stays on the same line), and `phase_done()` overwrites it using `\r` followed by the completed message with a newline
- [ ] **Non-TTY behavior:** `phase()` produces no output; `phase_done()` prints the final line once
- [ ] **Quiet mode:** Neither `phase()` nor `phase_done()` produces any output (unchanged from current behavior)
- [ ] The rendering phase progress bar (`render_progress`) still works correctly and is unaffected
- [ ] The `phase()` / `phase_done()` public API signatures are unchanged (callers in `main.rs` do not need modification)
- [ ] No performance regression -- the fix is trivial I/O change, no new allocations or syscalls per page

## Test Scenarios

### Unit: TTY mode phase output (src/progress.rs)

- Create a `ProgressReporter` with `new_with_tty(false, true)` (TTY mode). Call `phase("Loading...")` then `phase_done("Loading... 5 items")`. Verify the reporter does not panic and the API works. (We cannot easily capture stderr in unit tests, but we verify the code path executes.)

### Unit: Non-TTY mode phase output (src/progress.rs)

- Create a `ProgressReporter` with `new_with_tty(false, false)` (non-TTY mode). Call `phase("Loading...")` then `phase_done("Loading... 5 items")`. Verify no panic and the API works.
- Verify that in non-TTY mode, `phase()` is effectively a no-op (does not write output). This can be tested by adding a flag or counter that tracks writes, or by documenting the behavior with a comment-based test.

### Unit: Quiet mode unchanged

- Existing test `test_quiet_mode_suppresses_output` should continue to pass unchanged.

### Unit: phase_done without prior phase call

- Call `phase_done()` without calling `phase()` first. Verify it does not panic and prints the message correctly. (Defensive behavior.)

### Integration: Build output line count

- Run a real build (or a minimal site build) with `--quiet` off, capturing stderr.
- Count lines matching the phase pattern (e.g., "Loading data files", "Loading collections", "Copying static files", "Generating sitemap", "Generating feed").
- Verify each phase name appears exactly ONCE in the output (not twice).
- This is the key regression test: if the fix breaks, this test catches the duplicate.

### Integration: Quiet mode produces no phase output

- Run a build with `--quiet`, capturing stderr.
- Verify stderr contains no phase messages at all.

## Notes

- The `RenderProgress` (progress bar for rendering) is a separate mechanism using `indicatif` and should not be changed.
- Issue 92 (progress output integration tests) is a separate issue that may overlap; this issue should focus on the fix itself and add tests specific to the duplicate-line bug.

## Log

### [SWE] 2026-03-15

- Root cause: `phase()` and `phase_done()` both used `eprintln!()`, producing two lines per phase
- Fix in `src/progress.rs`:
  - TTY mode: `phase()` uses `eprint!("\r\x1b[2K{}")` + flush (no newline, overwrites previous); `phase_done()` uses `eprintln!("\r\x1b[2K{}")` (overwrites and finalizes with newline)
  - Non-TTY mode: `phase()` is a no-op; `phase_done()` prints final line once with `eprintln!`
  - Quiet mode: unchanged (both are no-ops)
- No changes to `src/main.rs` -- API signatures unchanged, only behavior of `phase()`/`phase_done()` changed
- Added 6 new unit tests in `src/progress.rs`: TTY mode, non-TTY mode, phase_done without prior phase, quiet mode both TTY variants, multiple phases, phase-without-phase_done
- Added 2 integration tests in `tests/integration_build.rs`: `test_cli_build_no_duplicate_phase_lines` (verifies each phase appears exactly once), `test_cli_build_quiet_no_phase_output` (verifies quiet mode suppresses all phases)
- Build: 1235 tests pass, 0 failed, 29 ignored
- Clippy: clean (no warnings)
- Fmt: clean
- Files modified: `src/progress.rs`, `tests/integration_build.rs`
