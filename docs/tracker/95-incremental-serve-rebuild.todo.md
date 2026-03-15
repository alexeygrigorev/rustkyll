# Issue 95: Incremental rebuild on file change in serve mode

## Problem

When serve mode detects a file change, it rebuilds the entire site from scratch. For a site with 789 pages, this takes ~2 seconds every time you save a file. It should only rebuild the page that changed.

## Goal

When a file changes during serve mode:
- If a content file changed (post, page, collection item): only rebuild that one page
- If a layout changed: rebuild all pages using that layout
- If an include changed: rebuild all pages that use that include
- If config changed: full rebuild
- If a data file changed: rebuild pages that reference that data

This is the `--incremental` behavior but applied automatically during serve.

## Current state

rustkyll already has `--incremental` flag for build mode. Check if this works with serve mode and if it handles the dependency tracking correctly.

## Acceptance criteria

- Single file change only rebuilds affected pages (not entire site)
- Layout changes rebuild all pages using that layout
- Include changes rebuild all pages using that include
- Config changes trigger full rebuild
- Rebuild time for a single page change is under 100ms (not 2 seconds)
- No stale pages (changed content always reflected)
- Works with live reload (browser updates after incremental rebuild)
- All existing tests still pass
