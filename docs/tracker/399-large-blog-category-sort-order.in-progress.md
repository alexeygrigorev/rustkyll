# Issue 399: large-blog-3000 -- category sort order differs from Jekyll

## Problem

large-blog-3000 is at 3000/3001 DOM match. The remaining page (`index.html`)
has 54 diffs showing wrong category order and wrong post-to-category assignments.

When iterating `site.categories` in a `{% for category in site.categories %}`
loop, Jekyll produces categories in **first-encounter order** (the order each
category is first seen when processing posts in date-ascending order). Rustkyll
produces categories in **non-deterministic HashMap order** because `Object` is
backed by `HashMap`, which does not preserve insertion order.

Jekyll order: Technology, Science, Travel, Food, Sports, Business, Health,
Education, Arts, Politics.

Rustkyll order (varies per run): e.g., Arts, Business, Education, Food, Health,
Politics, Science, Sports, Technology, Travel (or any other random permutation).

## Root Cause

In `src/generator.rs`, function `build_categories_and_tags_from_liquid`:

1. Categories are collected into an `IndexMap` (preserving first-encounter order).
2. The IndexMap is then iterated and each key-value pair is inserted into a
   `liquid::Object`, which is backed by `BTreeMap`.
3. BTreeMap iterates in alphabetical order, so the first-encounter ordering
   from the IndexMap is lost.

## Fix

Added `__key_order` metadata array to categories and tags Objects. The existing
`for_block.rs` in liquid-lib already checks for `__key_order` and iterates in
that order instead of BTreeMap alphabetical order. This preserves first-encounter
order during iteration while keeping key-based lookups working.

## Acceptance Criteria

- [x] `cargo build` compiles without errors
- [x] `cargo clippy -- -D warnings` passes clean (on files I modified)
- [x] `cargo fmt` reports no changes (on files I modified)
- [x] `site.categories` iterates in first-encounter order
- [x] `site.tags` iterates in first-encounter order
- [x] `site.categories["category_name"]` key-based lookups still work correctly
- [x] large-blog-3000 DOM comparison reaches 3001/3001
- [x] DTC DOM match count does not drop below 787/790 (actual: 788/790)
- [x] Existing tests continue to pass
- [x] Posts within each category are still in reverse chronological order

## Log

### [SWE] 2026-03-28
- TDD step 1: wrote 5 new tests first:
  - `test_categories_key_order_metadata` -- checks __key_order array exists with first-encounter order
  - `test_tags_key_order_metadata` -- same for tags
  - `test_categories_key_order_with_duplicates` -- duplicate categories listed once at first encounter
  - `test_categories_reverse_chronological_within_category` -- posts within category are newest-first
  - (existing tests also verified key-based lookups still work)
- TDD step 2: ran tests, all 5 FAILED as expected: "categories should have __key_order metadata"
- TDD step 3: implemented fix in `build_categories_and_tags_from_liquid()`:
  - Build `__key_order` Vec from IndexMap iteration order (first-encounter)
  - Insert as metadata into categories/tags Objects
  - Only insert when non-empty (avoid breaking empty-categories case)
  - Also applied same fix to dead-code `build_categories_and_tags()` for consistency
- TDD step 4: all 5 new tests PASS
- Updated 4 pre-existing tests that checked `cats.keys()` to filter out `__key_order` and verify encounter order
- Tests: 2930 lib tests pass, 0 fail; all integration tests pass
- Build: clean compile, clippy clean, fmt clean
- DOM: large-blog-3000 3001/3001, DTC 788/790
- Files modified: src/generator.rs (only file changed)
