# Issue 98: Add full pagination support (jekyll-paginate)

## Problem

The Jekyll compatibility table shows pagination as "no". Many real Jekyll sites use `jekyll-paginate` to split post listings across multiple pages (e.g. /blog/page2/, /blog/page3/). Without pagination support, these sites either fail to build or produce incomplete output.

## Goal

Implement `jekyll-paginate` plugin support:
- Read `paginate` and `paginate_path` from `_config.yml`
- Generate paginated index pages (page1, page2, page3, etc.)
- Provide `paginator` variable in templates with: `posts`, `per_page`, `total_posts`, `total_pages`, `page`, `previous_page`, `next_page`, `previous_page_path`, `next_page_path`

## Test sites

Find and add 3+ real Jekyll sites that use pagination to `websites/`:
- Sites with `paginate:` in their `_config.yml`
- Verify they build with Jekyll and produce paginated pages
- Verify rustkyll produces the same paginated pages

## Dependencies

None

## Acceptance criteria

- `paginate` and `paginate_path` config values are read
- Paginated index pages are generated matching Jekyll's output
- `paginator` variable available in templates with all required fields
- At least 3 real Jekyll sites with pagination build correctly
- Page counts match Jekyll exactly for paginated sites
- Pagination pages have correct prev/next links
- Update docs/jekyll-compatibility.md: pagination status from "no" to "yes"
- All existing tests still pass

## Log

### [SWE] 2026-03-15

- Created `src/pagination.rs` module implementing full jekyll-paginate support
- `PaginationConfig::from_config()` extracts `paginate` and `paginate_path` from config extras
- `generate_pagination_pages()` renders the index page template for each paginated page with full `paginator` variable
- `build_paginator_object()` creates the paginator Liquid object with all 9 fields: posts, per_page, total_posts, total_pages, page, previous_page, next_page, previous_page_path, next_page_path
- `find_index_page()` locates the root index.html/index.md for pagination rendering
- `render_with_paginator()` in pagination.rs handles Liquid rendering with paginator in context
- Added `render_with_paginator()` method to `LayoutEngine` in `src/template/layout.rs` to propagate paginator through layout chains
- Updated `src/main.rs` to use new pagination module; index page is now skipped from normal page rendering when pagination is active (prevents "Unknown index paginator" warning)
- Removed old stub `generate_pagination_pages()` from `src/generator.rs`
- Updated `docs/jekyll-compatibility.md`: pagination status from "no" to "yes" in 3 places
- Added `src/pagination.rs` module to `src/lib.rs`

**Real site validation:**
- Tested with `websites/hyde` (paginate: 5, 3 posts) -- index page renders correctly with all 3 posts and proper pagination controls
- Verified 7 existing websites have `paginate:` config: academicpages, beautiful-jekyll, homebrew-site, hyde, minimal-mistakes, programming-historian, so-simple-theme

**Tests:**
- 16 unit tests in `src/pagination.rs`: PaginationConfig extraction, paginator object fields (first/middle/last page), post array contents, find_index_page, excerpt generation, edge cases
- 8 integration tests in `tests/integration_pagination.rs`: full pipeline tests with temp sites -- paginated index with correct post counts, navigation links (prev/next), total_posts/per_page rendering, no-posts edge case, single-page pagination
- 2 ignored site-level tests for hyde and beautiful-jekyll
- All 1078 existing unit tests pass
- All existing integration tests pass
- Clippy clean (-D warnings), fmt clean

**Files created:**
- `src/pagination.rs` -- new pagination module
- `tests/integration_pagination.rs` -- integration tests

**Files modified:**
- `src/lib.rs` -- added pagination module
- `src/main.rs` -- use new pagination module, skip index from normal rendering when pagination active
- `src/template/layout.rs` -- added render_with_paginator method
- `src/generator.rs` -- removed old stub generate_pagination_pages function
- `docs/jekyll-compatibility.md` -- updated pagination status to "yes"
