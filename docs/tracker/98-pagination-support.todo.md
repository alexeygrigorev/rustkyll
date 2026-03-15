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
