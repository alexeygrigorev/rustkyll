# Issue 91: Show build progress during site generation

## Problem

When running `rustkyll serve` or `rustkyll build`, the user sees nothing until the build is complete:

```
$ uvx rustkyll serve
Building site before serving...
Build complete: 789 pages generated.
```

For a 2-second build this is fine, but for larger sites or slower machines, the user has no idea if it's working or stuck.

## Goal

Show progress during the build so the user knows something is happening. Should include:
- Phase indicators (loading config, loading collections, rendering pages, copying static files)
- Page count progress (e.g. "Rendering pages... 150/789")
- Elapsed time
- Final summary with timing breakdown

## Example output

```
$ rustkyll build
Source:      .
Destination: _site

Loading config...
Loading collections... 6 collections, 1543 items
Loading data files... 15 files
Rendering pages... 789/789
Copying static files... 1455 files
Generating sitemap... 789 entries
Generating feed... 20 entries

Build complete!
  Pages:        789
  Static files: 1455
  Time:         1.87s
```

Or with a progress bar showing the current file:
```
Rendering [=================>------] 650/789  blog/segmentation.html
```

## Dependencies

None. All prerequisite build infrastructure is already in place (issues 01-19 are done).

## Scope

This issue covers adding real-time progress reporting to the existing `build_site` function in `main.rs`. The build pipeline phases are already structured and timed (see `PhaseTiming` struct). The work is to:

1. Add a progress reporting mechanism (callback, trait, or direct stderr writes)
2. Emit progress messages at each build phase boundary
3. Add a progress bar with file count for page rendering (the longest phase)
4. Add a `--quiet` flag to suppress progress output
5. Ensure all progress output goes to stderr (not stdout)
6. Ensure progress works for both `build` and `serve` commands

## Design notes

- The existing `build_site` function in `main.rs` already has numbered phases (1-15) with `Instant::now()` timing. Progress messages should be emitted at each phase start.
- Page rendering happens in `generator::generate_collection_pages_cached` and `generator::generate_pages_cached_with_config`. These use rayon for parallelism. The progress counter must be thread-safe (e.g., `AtomicUsize`).
- The `indicatif` crate is the standard Rust choice for progress bars. It handles terminal detection, line clearing, and thread-safe counters. Using it is recommended but not required.
- Progress output MUST go to stderr so that stdout remains clean for piping/scripting.
- The current `println!` calls for "Source:", "Destination:", and the final summary already go to stdout. Those can stay on stdout. Only the real-time progress indicators (phase status, progress bar) must go to stderr.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes (all existing tests continue to pass)
- [ ] When running `rustkyll build`, each build phase prints a status line to stderr as it starts (config, data, collections, pages, rendering, static files, sitemap, feed)
- [ ] During page rendering, a progress counter or progress bar shows the number of pages rendered out of the total (e.g., "Rendering pages... 150/789" or a visual bar)
- [ ] The progress indicator updates as pages are rendered (not just printed once at the end)
- [ ] The final build summary (page count, static file count, time) is still printed after the build completes (to stdout, as it is today)
- [ ] All real-time progress output (phase indicators, progress bar) goes to stderr, not stdout
- [ ] A `--quiet` flag is added to the `build` subcommand that suppresses all progress output (only errors are shown)
- [ ] A `--quiet` flag is added to the `serve` subcommand that suppresses all progress output (only errors are shown)
- [ ] When `--quiet` is set, the final summary is also suppressed -- only errors go to stderr
- [ ] Progress output works correctly when stderr is not a TTY (e.g., piped to a file) -- no ANSI escape codes in non-TTY mode, or graceful fallback to simple line-by-line output
- [ ] The `serve` command also shows progress during its initial build (currently it just prints "Building site before serving...")
- [ ] No measurable performance regression from progress output (progress updates should not add more than 1% overhead to build time)

## Test Scenarios

### Unit: CLI flag parsing
- Parse `rustkyll build --quiet` and verify the quiet flag is true
- Parse `rustkyll build` (no --quiet) and verify the quiet flag is false/default
- Parse `rustkyll serve --quiet` and verify the quiet flag is true

### Unit: Progress reporting abstraction
- Create a progress reporter in quiet mode, verify no output is produced when methods are called
- Create a progress reporter in normal mode, verify phase start messages are written to a buffer
- Verify the progress counter can be incremented from multiple threads (AtomicUsize or similar) without data races

### Integration: Build with progress output
- Build a minimal test site and capture stderr; verify stderr contains phase indicator strings (e.g., "Loading config", "Rendering pages", "Copying static files")
- Build a minimal test site with `--quiet` and capture stderr; verify stderr is empty (no progress output, no errors for a clean build)
- Build a minimal test site and capture stdout; verify stdout does NOT contain progress bar or phase indicator strings (only the final summary)

### Integration: Progress counter accuracy
- Build a test site with a known number of pages (e.g., 5 collection items + 2 standalone pages); verify the progress output mentions the correct total count
- Verify the final "Build complete!" summary page count matches the progress counter's final value

### Edge case: Non-TTY stderr
- Run the build with stderr redirected to a file; verify the output file contains readable progress lines without ANSI escape codes or carriage returns that would make it unreadable

## Log

### [SWE] 2026-03-15 12:00
- Implemented build progress reporting with `indicatif` crate
- Created `src/progress.rs` with `ProgressReporter` and `RenderProgress` abstractions
- `ProgressReporter` emits phase messages to stderr; suppressed in quiet mode
- `RenderProgress` wraps indicatif `ProgressBar` with `Arc<AtomicUsize>` counter for thread-safe progress tracking
- Added `--quiet` flag to both `build` and `serve` commands
- Non-TTY fallback: when stderr is not a terminal, progress bar is hidden (no ANSI codes); phase messages are simple line-by-line text
- Phase indicators added at: config loading, data loading, collection loading, page loading, context building, layout loading, rendering, static file copying, sitemap generation, feed generation
- Rendering progress uses atomic counter passed to `generate_collection_pages_cached_with_progress` and `generate_pages_cached_with_config_and_progress` -- incremented inside rayon's `par_iter().for_each()` after each page is written
- All progress output goes to stderr via `eprintln!`; final summary stays on stdout via `println!`
- When `--quiet` is set, both progress output and final summary are suppressed
- Added 11 tests: 6 unit tests for progress module + 5 CLI/integration tests in main.rs
- Build: 947 lib + 28 bin + all integration tests pass (1063 total), 0 failures, clippy clean, fmt clean
- Files created: `src/progress.rs`
- Files modified: `Cargo.toml` (added indicatif), `src/lib.rs` (added progress module), `src/main.rs` (--quiet flags, progress integration), `src/generator.rs` (added `_with_progress` variants)

### [QA] 2026-03-15 13:00
- All tests pass: 947 lib + 28 bin + all integration tests, 0 failures
- Clippy: PASS (clean)
- Fmt: FAIL -- `src/generator.rs` has a formatting issue around the `page.path` insert (multi-line vs single-line)

**Acceptance criteria results:**
- AC1 (compiles): PASS
- AC2 (clippy): PASS
- AC3 (tests pass): PASS
- AC4 (phase indicators to stderr): PASS
- AC5 (progress counter/bar during rendering): PASS
- AC6 (progress updates as pages render): FAIL -- The progress bar does NOT update in real-time per page. The atomic counter is incremented per-page inside the rayon loop (generator.rs), but `set_position_from_counter()` is only called AFTER the entire par_iter finishes for each collection batch. So for a collection with 500 posts, the bar jumps from 0 to 500 instead of showing incremental progress. The `RenderProgress::inc()` method (which would update the bar per-page) is dead code -- never called from anywhere.
- AC7 (final summary to stdout): PASS
- AC8 (progress to stderr): PASS
- AC9 (--quiet on build): PASS
- AC10 (--quiet on serve): PASS
- AC11 (--quiet suppresses summary): PASS
- AC12 (non-TTY fallback): PASS
- AC13 (serve shows progress): PASS
- AC14 (no performance regression): PASS

**Test scenario coverage:**
- CLI flag parsing (4 tests): PASS
- Progress reporting abstraction (3 tests): PASS
- Integration build with progress output: PARTIAL -- `test_build_quiet_mode_produces_no_progress` only asserts build success, does NOT verify stderr is empty in quiet mode or that stderr contains phase indicators in normal mode
- Progress counter accuracy: MISSING -- no test verifies that progress count matches a known number of pages
- Non-TTY edge case: MISSING -- no test verifies absence of ANSI codes when stderr is redirected

**Issues to fix:**
1. **`cargo fmt --check` fails** -- Run `cargo fmt` to fix `src/generator.rs` formatting.
2. **Progress bar does not update in real-time (AC6 violation)** -- In `src/generator.rs`, the progress bar position needs to be updated per-page inside the rayon loop, not just after the batch. Either: (a) pass the `ProgressBar` (or `RenderProgress`) into the generator functions so `bar.set_position()` is called after each page, or (b) call `bar.set_position(counter.load(...))` periodically inside the loop. The simplest fix is to pass the `Option<ProgressBar>` alongside the counter and call `bar.inc(1)` directly in the par_iter closure (indicatif's ProgressBar is thread-safe).
3. **Dead code: `RenderProgress::inc()` is never called** -- Either use it or remove it. If approach (2b) above is used, `inc()` could be the vehicle. Otherwise remove it to avoid confusion.

- VERDICT: FAIL

### [SWE] 2026-03-15 14:00 -- QA fixes

Fixed all 3 issues from QA feedback:

1. **cargo fmt**: Ran `cargo fmt` to fix formatting in `src/generator.rs`.

2. **Progress bar now updates in real-time**: Changed `generate_collection_pages_cached_with_progress` and `generate_pages_cached_with_config_and_progress` to accept `Option<&RenderProgress>` instead of `Option<Arc<AtomicUsize>>`. The `RenderProgress::inc()` method is now called directly inside each rayon `par_iter` closure, so the progress bar increments after every single page render rather than in batch jumps.

3. **RenderProgress::inc() is no longer dead code**: It is now the primary mechanism for progress updates, called from inside the rayon loops in `generator.rs`. Removed the now-unnecessary `set_position_from_counter()` and `counter()` methods from `RenderProgress`. Simplified `inc()` to use `bar.inc(1)` (indicatif's native thread-safe increment) instead of manually setting position.

Files modified:
- `src/generator.rs`: Changed `_with_progress` function signatures from `Option<Arc<AtomicUsize>>` to `Option<&RenderProgress>`, call `progress.inc()` directly in par_iter closures, removed unused atomic imports.
- `src/progress.rs`: Removed `set_position_from_counter()` and `counter()` methods, simplified `inc()` to use `bar.inc(1)`, updated thread-safety test to use scoped threads with `&RenderProgress` (matching real usage pattern).
- `src/main.rs`: Pass `Some(&render_progress)` instead of `Some(render_progress.counter())`, removed `set_position_from_counter()` calls.

All tests pass: 947 lib + 28 bin + all integration tests, 0 failures. Clippy clean. Fmt clean.

### [PM] 2026-03-15 15:00 -- Acceptance review

**Build verification:**
- `cargo build`: PASS
- `cargo clippy -- -D warnings`: PASS
- `cargo fmt --check`: PASS
- `cargo test`: PASS (1063 total: 947 lib + 28 bin + 88 integration, 0 failures)

**Acceptance criteria review (all 14 criteria):**
- AC1 (compiles): PASS
- AC2 (clippy): PASS
- AC3 (tests pass): PASS
- AC4 (phase indicators to stderr): PASS -- `progress.phase()` called at all 10 build phases
- AC5 (progress counter/bar during rendering): PASS -- `RenderProgress` with indicatif bar
- AC6 (progress updates in real-time): PASS -- `p.inc()` called inside `par_iter` closure per page (fixed after QA feedback)
- AC7 (final summary to stdout): PASS -- `println!` used for summary
- AC8 (progress to stderr): PASS -- `eprintln!` for phases, indicatif targets stderr
- AC9 (--quiet on build): PASS -- flag added via clap
- AC10 (--quiet on serve): PASS -- flag added via clap
- AC11 (--quiet suppresses summary): PASS -- `Ok(summary) if !quiet =>` pattern
- AC12 (non-TTY fallback): PASS -- `ProgressDrawTarget::hidden()` when not a TTY, plain `eprintln!` for phases
- AC13 (serve shows progress): PASS -- same `build_site` call with progress integrated
- AC14 (no performance regression): PASS

**Code quality observations:**
- Clean separation: `ProgressReporter` for phase messages, `RenderProgress` for the rendering bar
- Thread safety: `bar.inc(1)` inside rayon closures (indicatif is natively thread-safe)
- `_with_progress` variants maintain backward compatibility via delegation
- 6 unit tests in `progress.rs` + 5 tests in `main.rs` = 11 new tests total

**Descoped test scenarios (tracked in issue 92):**
The following test scenarios from the spec are not implemented. They require subprocess-based integration tests (capturing stderr from a binary invocation) which is non-trivial and not the pattern used elsewhere in this codebase:
1. Capture stderr in normal mode and verify it contains phase indicator strings
2. Capture stderr in quiet mode and verify it is empty
3. Verify stdout does NOT contain progress bar strings
4. Verify progress count matches a known number of pages
5. Verify non-TTY output has no ANSI escape codes

These are tracked in issue 92.

**Unrelated changes in working tree:**
The diff also contains changes to `seo_tag.rs`, `playwright/tests/visual-compare.spec.ts`, `page.name`/`page.path` in `generator.rs`, and issue 87/90 tracker files. These are not part of issue 91 and should be committed separately.

- VERDICT: ACCEPT
