# Issue 510: Generate al-folio blog pagination pages (jekyll-paginate-v2)

## Problem

al-folio uses `jekyll-paginate-v2` to paginate the blog index. Jekyll generates 7 pages (`blog/index.html`, `blog/page/2/index.html` through `blog/page/7/index.html`). rustkyll generates only the first page (`blog/index.html`) and none of the subsequent pagination pages.

The al-folio `_config.yml` includes `jekyll-paginate-v2` in its plugins list.

## Scope

1. Ensure rustkyll's pagination support handles the `jekyll-paginate-v2` configuration used by al-folio.
2. Generate all pagination pages (page/2/ through page/N/).
3. Verify the paginated blog index pages are correct.

## Baseline

- al-folio missing pages: 6 pagination pages (blog/page/2/ through blog/page/7/)
- DTC DOM baseline: 790/790

## Acceptance Criteria

- [ ] Building al-folio generates `blog/page/2/index.html` through `blog/page/7/index.html` (or however many pages are needed based on post count and page size).
- [ ] Each pagination page contains links to blog posts appropriate for that page.
- [ ] Pagination navigation (next/previous links) works correctly.
- [ ] DTC DOM match count does not drop below 790/790.
- [ ] `cargo build` compiles without errors; `cargo clippy` clean; `cargo fmt` clean.

## Test Scenarios

### Integration: pagination output
- Build al-folio and verify `blog/page/2/index.html` exists.
- Verify the last pagination page (`blog/page/7/index.html`) exists.
- Verify the paginated pages contain different sets of posts (not duplicates).

## Dependencies

- Issue #235 (al-folio site is set up)
- Issue #505 (layouts needed for pagination pages to render with HTML structure)
