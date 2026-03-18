# Issue 194: Fix muan-blog body class regression

## Problem

Issue 188 added `page.collection` variable support, but for muan-blog it now produces `col-pages` where Jekyll produces `col-` (empty collection name). The fix was correct for most sites but wrong for muan-blog's specific case.

Jekyll apparently sets `page.collection` to empty string for pages in certain contexts (e.g., when pages are standalone, not part of a named collection). We need to match this behavior.

## Goal

Fix body class to produce `col-` when the page has no real collection, matching Jekyll.

## Approach (TDD)

1. Investigate when Jekyll sets page.collection to empty vs the collection name
2. Write a failing test
3. Fix the logic
4. Verify muan-blog body class matches

## Log

### [SWE] 2026-03-18

- Root cause: Issue 188 added `page.collection = "pages"` injection for standalone pages in `generate_pages_cached_with_config_and_progress()` (line 1076-1079). In Jekyll, standalone pages have `page.collection = nil`, so `col-{{ page.collection }}` produces `col-` (empty). Our code incorrectly produced `col-pages`.
- Fix: Removed the `collection = "pages"` injection for standalone pages. Only actual collection items (going through `generate_collection_pages`) get their collection name injected.
- TDD followed: wrote failing test first (`test_standalone_page_collection_is_empty`), verified it failed with `col-pages`, then applied fix, verified test passes.
- Existing issue 188 tests (collection items: pages, posts, custom) all still pass -- those correctly go through `generate_collection_pages` which injects from `item.collection_name`.
- Build: all tests pass, clippy clean, fmt clean
- Files modified: `src/generator.rs`
