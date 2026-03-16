# Issue 150: Improve progress bar layout — two lines

## Problem

The current progress bar shows the progress + current file on the same line, which causes the bar to jump/shift as filenames change length. Confusing to watch.

## Goal

Two-line layout during rendering:
- Line 1: Progress bar (fixed width, stays in place)
- Line 2: Current file being rendered (updates independently)

```
Rendering pages [████████████░░░░░░░░] 650/789
  → podcast/ai-for-ecology-biodiversity-and-conservation.html
```

The progress bar stays stable while the filename updates below it.

## Acceptance criteria

- Progress bar on line 1, current file on line 2
- Progress bar doesn't shift/jump when filename changes
- Works on TTY (ANSI escape codes for cursor positioning)
- Non-TTY: just print filename per line (no cursor tricks)
- --quiet suppresses both lines
- No performance regression
- All existing tests still pass
