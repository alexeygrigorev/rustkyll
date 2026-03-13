# Issue 34: Live Reload

## Problem

During development, users want to see changes immediately without manually rebuilding and refreshing the browser.

## Requirements

- Watch source files for changes (using `notify` crate or similar)
- Automatically rebuild changed pages when source files are modified
- Inject a live-reload script into served pages that triggers browser refresh on rebuild
- Works with `rustkyll serve` command (add `--livereload` flag, on by default)
- Support `--no-livereload` to disable

## Dependencies

- Issue #33 (serve command)
