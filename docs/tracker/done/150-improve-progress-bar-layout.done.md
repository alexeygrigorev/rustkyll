# Issue 150: Improve progress bar layout — two lines

## Problem

The current progress bar shows the progress + current file on the same line, which causes the bar to jump/shift as filenames change length. Confusing to watch.

## Goal

Two-line layout during rendering:
- Line 1: Progress bar (fixed width, stays in place)
- Line 2: Current file being rendered (updates independently)

```
Rendering pages [████████████░░░░░░░░] 650/789 (1.2s)
  → podcast/ai-for-ecology-biodiversity-and-conservation.html
```

The elapsed time shows how long the build has been running so far.

The progress bar stays stable while the filename updates below it.

## Acceptance criteria

- Progress bar on line 1 with elapsed time
- Current file on line 2
- Progress bar doesn't shift/jump when filename changes
- Works on TTY (ANSI escape codes for cursor positioning)
- Non-TTY: just print filename per line (no cursor tricks)
- --quiet suppresses both lines
- No performance regression
- All existing tests still pass

## Log

### [SWE] 2026-03-16 14:00
- Refactored `src/progress.rs` to use `indicatif::MultiProgress` for two-line TTY layout
- Line 1: progress bar with elapsed time via `{elapsed}` template variable
- Line 2: current file indicator using a spinner-style ProgressBar with `{wide_msg}` template
- On TTY: `MultiProgress` manages both bars, keeping line 1 stable while line 2 updates
- On non-TTY: unchanged behavior (hidden bar, no ANSI tricks)
- On quiet: no-op, both `bar` and `file_line` are `None`
- Added `file_line: Option<ProgressBar>` and `_multi: Option<MultiProgress>` fields to `RenderProgress`
- `inc()` now updates bar and file_line independently
- `finish()` clears both lines (file_line first, then bar)
- Added 5 new tests: `test_tty_render_progress_has_two_lines`, `test_non_tty_render_progress_has_no_file_line`, `test_tty_render_progress_thread_safe`, `test_render_progress_finish_clears_both_lines`, `test_quiet_render_progress_finish_is_noop`
- Updated existing test `test_render_progress_quiet_returns_noop` to check new fields
- Build: all tests pass (1265 unit + integration tests), 0 failures, clippy clean, fmt clean
- Files modified: `src/progress.rs`
- No new dependencies added (already using `indicatif` 0.17 which includes `MultiProgress`)
