# Issue 455: beautiful-jekyll — page.id evaluates as falsy in Liquid

## Problem

beautiful-jekyll (3/5, 60%) has 2 post pages with 66 diffs, all from
one root cause: `{% if page.id %}` evaluates as falsy for posts.

Jekyll sets page.id for posts (e.g., `/2020/02/26/flake-it-till-you-make-it`).
Rustkyll computes it correctly but the Liquid engine treats it as falsy.

This causes posts to render with `og:type: website` instead of `article`,
and missing `og:article:author` and `og:article:published_time` tags.

## Root Cause

Either:
1. page.id is not being set in the Liquid context for collection items
2. page.id is set but as nil/empty instead of the actual ID string

## Scope

Fix page.id to be truthy (non-empty string) for collection items in
the Liquid context. Check src/generator.rs or src/collection.rs where
page variables are set for Liquid rendering.

## Baseline

DTC 790/790. beautiful-jekyll 3/5.
Target: 5/5 (100%).

## Log

### [SWE] 2026-03-28
- Root cause: `item.id` was set in cross-reference Liquid objects (lines 677, 892 of generator.rs) but NOT injected into `page_fm` HashMap used for current-page rendering context. So `page.id` was nil/missing in templates.
- Wrote 3 tests in tests/test_issue_455.rs (TDD):
  - test_page_id_truthy_in_post_template: verifies `{% if page.id %}` takes the truthy branch
  - test_page_id_value_in_template: verifies `{{ page.id }}` renders the correct ID path
  - test_page_id_unicode_post: verifies page.id works with non-ASCII content
- Ran tests: all 3 FAIL as expected (page.id falsy, og:type=website instead of article)
- Fix: added `page_fm.insert("id", item.id)` in generate_collection_pages at line ~1587 of src/generator.rs
- Ran tests: all 3 PASS
- Full suite: 3400+ tests pass, 0 fail
- Clippy: clean (no warnings)
- Fmt: clean
- DOM verification:
  - DTC: 790/790 (no regression)
  - beautiful-jekyll: 4/5 (up from 3/5; remaining 1 diff is LaTeX/kramdown attribute issues unrelated to this fix)
- Files modified: src/generator.rs, tests/test_issue_455.rs (new)
