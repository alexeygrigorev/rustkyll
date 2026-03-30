# Issue 540: site.tags and site.categories iteration returns empty

## Problem

Tag and category archive pages that use `{% for tag in site.tags %}` with
`tag[1].size` render as nearly-empty pages in rustkyll. On the basically-basic
example site:

- `tags/index.html`: 4.8 KB (rustkyll) vs 363 KB (Jekyll)
- `categories/index.html`: similarly truncated

Jekyll provides `site.tags` as an array of pairs: `[["tagname", [post1, post2, ...]], ...]`.
Liquid templates iterate over these pairs using `tag[0]` for the name and `tag[1]`
for the posts array.

## Root Cause

`site.tags` and `site.categories` are either not populated or not structured as the
expected array-of-pairs format that Jekyll provides. When templates iterate over
`site.tags`, they get no data, resulting in empty archive pages.

## Acceptance Criteria

- [ ] `site.tags` returns array-of-pairs `[["tagname", [post1, post2, ...]], ...]`
- [ ] `site.categories` returns array-of-pairs `[["catname", [post1, post2, ...]], ...]`
- [ ] Tag archive pages render post lists for each tag
- [ ] Category archive pages render post lists for each category
- [ ] Basically-basic tags/index.html file size is within 10% of Jekyll output
- [ ] DTC DOM baseline must not regress

## Dependencies

- Discovered in #355 (basically-basic triage)

## Log

### [SWE] 2026-03-30

- Investigation: site.tags and site.categories iteration already works correctly
- The Object-with-`__key_order` structure used by `build_categories_and_tags_from_liquid()` in
  `src/generator.rs` produces proper `[key, value]` pairs when iterated in for-loops
- The template engine's `LenientValue` correctly handles `tag[0]` (name), `tag[1]` (posts array),
  `tag[1].size` (post count), and `tag.last` (posts array via .last)
- Verified basically-basic site output:
  - tags/index.html: 303 KB rustkyll vs 363 KB Jekyll (83% -- structural elements identical: 49 sections, 282 li elements, all tag names and counts match)
  - categories/index.html: 95 KB rustkyll vs 114 KB Jekyll (same section structure)
  - Size difference is due to content/excerpt rendering differences, not tags/categories iteration
- Added 4 regression tests in `src/template/engine.rs`:
  - `test_issue540_tag_pair_index1_size` -- verifies `tag[1].size` returns correct post count
  - `test_issue540_tag_pair_index1_size_comparison` -- verifies max-finding pattern from basically-basic
  - `test_issue540_tag_last_iterates_posts` -- verifies `tag.last` iterates post array
  - `test_issue540_category_pair_index1_size` -- verifies `category[1].size` pattern
- All 4 tests pass (functionality already working from previous issues)
- DTC DOM: 783/787 match (4 with differences) -- no regression from test additions
- Build: 3857 tests pass, 0 fail, clippy clean, fmt clean
- Files modified: `src/template/engine.rs` (test additions only)
