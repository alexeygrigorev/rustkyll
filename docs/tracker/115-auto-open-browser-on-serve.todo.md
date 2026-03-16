# Issue 115: Automatically open browser on serve

## Problem

When running `rustkyll serve`, the user has to manually open http://127.0.0.1:4000 in their browser. Jekyll does this automatically.

## Goal

After the site is built and the server starts, automatically open the default browser to the serve URL. Add `--no-browser` flag to disable this.

## Acceptance criteria

- `rustkyll serve` opens the default browser to http://127.0.0.1:{port}
- `--no-browser` flag disables auto-open
- Works on Linux (xdg-open), macOS (open), Windows (start)
- Browser opens AFTER the server is ready (not before)
- No error if browser can't be opened (e.g. headless server) — just skip silently
- All existing tests still pass
