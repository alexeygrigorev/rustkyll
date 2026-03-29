# Issue 485: academicpages post metadata rendering

## Problem

On academicpages, post pages and archive listings render incorrect metadata HTML. Two distinct bugs:

### Bug 1: Missing read-time `page__meta` block on post pages and archive listings

The `archive-single.html` include and the `single.html` layout both check `post.read_time` / `page.read_time` to decide whether to render a read-time paragraph:

```html
{% if post.read_time %}
  <p class="page__meta"><i class="fa fa-clock" aria-hidden="true"></i> {% include read-time.html %}</p>
{% endif %}
```

In the academicpages `_config.yml`, posts have `read_time: true` via defaults. Jekyll evaluates `post.read_time` as truthy and renders the `page__meta` block. Rustkyll apparently does not -- `post.read_time` evaluates as falsy, the `page__meta` block is skipped entirely, and the DOM comparison shows `page__date` where `page__meta` is expected.

The DOM diff pattern on every affected page is:
- Expected: `class='page__meta'` with `<i>` icon child, followed by read-time text
- Actual: `class='page__date'` with `<strong>` child, followed by `<time>` element

This means the `{% if post.read_time %}` branch is being skipped and only the `{% elsif post.date %}` branch fires.

### Bug 2: Portfolio items render `page__date` block instead of `archive__item-excerpt`

Portfolio collection items (`_portfolio/`) have no `date` field in their frontmatter. In Jekyll, `post.read_time` is falsy and `post.date` is also falsy for portfolio items, so neither the `page__meta` nor the `page__date` block renders -- only the `archive__item-excerpt` paragraph renders.

Rustkyll appears to assign a date to portfolio items (possibly from the filesystem or a default), causing the `{% elsif post.date %}` branch to fire incorrectly, rendering a `page__date` block instead of skipping to the `archive__item-excerpt` block.

## Root Cause Investigation

The engineer should check:

1. **Default application for `read_time`**: Are `_config.yml` defaults (specifically `read_time: true` for posts) being applied to the template context for both `post.*` and `page.*` variables? Check `src/config.rs` defaults handling and how defaults are merged into the Liquid context.

2. **Spurious `date` on non-post collections**: Are collection items that lack a `date` in frontmatter being given a date anyway? Check whether rustkyll injects a date from the filename or filesystem for non-post collection types (like portfolio). Jekyll only extracts dates from filenames for posts, not for arbitrary collections.

3. **`words_per_minute` config**: The `read-time.html` include checks `site.words_per_minute`. Verify this is available in the Liquid context as a number (160 for academicpages).

## Affected Pages

From the DOM comparison (see `docs/comparison/dom-details/academicpages.txt`):

| Page | Diff Count | Bug |
|------|-----------|-----|
| `posts/2012/08/blog-post-1/index.html` | 24 | Bug 1: missing `page__meta` read-time |
| `posts/2013/08/blog-post-2/index.html` | 24 | Bug 1 |
| `posts/2014/08/blog-post-3/index.html` | 24 | Bug 1 |
| `tags/index.html` | 90 | Bug 1 (repeated per post in listing) |
| `year-archive/index.html` | 30 | Bug 1 (repeated per post in listing) |
| `portfolio/index.html` | 10 | Bug 2: spurious `page__date` on dateless items |

The single post pages (blog-post-1, -2, -3) each show the pattern twice: once in the header (from `single.html`) and once in the related-posts grid (from `archive-single.html`).

## Affected Sites

- academicpages

## Baseline

- DTC: 790/790. Must not regress.
- academicpages: 27/45. Must not regress (target: improvement).

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] On academicpages post pages (`posts/2012/08/blog-post-1/`, etc.), the header contains `<p class="page__meta"><i class="fa fa-clock" ...></i> less than 1 minute read</p>` matching Jekyll output
- [ ] On academicpages post pages, the `page__date` block with `<strong><i class="fa fa-fw fa-calendar" ...></i> Published:</strong> <time ...>` also renders (it should appear in addition to `page__meta`, not instead of it)
- [ ] On archive listing pages (`tags/index.html`, `year-archive/index.html`), each post entry contains `<p class="page__meta"><i class="fa fa-clock" ...></i>` read-time paragraph
- [ ] On `portfolio/index.html`, portfolio items render `<p class="archive__item-excerpt" itemprop="description">` without any `page__date` block (no spurious date)
- [ ] DTC DOM match count remains at 790/790 (no regression)
- [ ] academicpages DOM match count improves from 27/45 (target: fix all diffs attributable to this issue)

## Test Scenarios

### Unit: Default application for read_time

- Build academicpages site and verify that `post.read_time` is truthy in the Liquid context for posts (defaults applied)
- Verify that portfolio collection items do NOT have `post.read_time` set to true (no defaults for portfolio read_time)

### Unit: Date handling for non-post collections

- Verify that portfolio items without a `date` in frontmatter have `post.date` evaluate as falsy/nil in Liquid context
- Verify that post items with dates in their filenames have `post.date` evaluate as truthy

### Integration: Post page rendering

- Build academicpages, read `posts/2012/08/blog-post-1/index.html`
- Assert the output contains `class="page__meta"` with `<i class="fa fa-clock"` child
- Assert the output contains `class="page__date"` with `<strong>` and `<time>` elements
- Assert both blocks are present (read-time AND date)

### Integration: Archive listing rendering

- Build academicpages, read `tags/index.html`
- Assert each post entry contains `<p class="page__meta">` with read-time content
- Assert each post entry contains `<p class="page__date">` with date content
- Assert `<p class="archive__item-excerpt"` blocks are present where expected

### Integration: Portfolio rendering

- Build academicpages, read `portfolio/index.html`
- Assert portfolio items contain `<p class="archive__item-excerpt" itemprop="description">`
- Assert portfolio items do NOT contain `class="page__date"`

### Regression: DOM baseline

- Run DOM comparison for DTC site, verify 790/790
- Run DOM comparison for academicpages, verify no regression below 27/45

## Dependencies

None (this is a rendering correctness fix).

## Log

### [SWE] 2026-03-29

#### Bug 1 Fix: Config defaults applied to collection items in site context

- Root cause: `collection_item_to_liquid_slim()` and `collection_item_to_liquid_ultra_slim()` in `src/generator.rs` did not apply `_config.yml` defaults when building the Liquid objects for `site.posts`, `site.categories`, etc. So `read_time: true` (set via defaults for posts in academicpages) was not available on `post` objects in site-level iteration.
- TDD: Wrote 3 failing tests first (`test_issue485_config_defaults_applied_to_site_posts`, `test_issue485_config_defaults_do_not_override_post_frontmatter`, `test_issue485_portfolio_items_have_backfilled_date`) -- confirmed they failed before implementation.
- Fix: Added `config: &SiteConfig` parameter to `collection_item_to_liquid_slim()` and `collection_item_to_liquid_ultra_slim()`. Applied config defaults as a base layer before copying front matter fields (front matter values override defaults). Updated all callers including `build_site_context`, `build_related_posts`, `build_categories_and_tags_from_liquid`, and related-posts construction in `generate_collection_pages`.
- After fix: All 5 issue-specific tests pass; full suite passes (3107+ tests, 0 failures).

#### Bug 2 Investigation: Portfolio dates

- Investigated whether portfolio items should have dates suppressed in the site-level Liquid context.
- Fresh Jekyll build confirms portfolio items DO have `page__date` blocks with the build timestamp (same as rustkyll). The issue description was based on stale cached Jekyll output.
- Verified by running `bundle exec jekyll build` on academicpages and checking `_site/portfolio/index.html`: it contains `page__date` blocks.
- Updated the Jekyll cache for academicpages with the fresh build.
- Conclusion: Bug 2 is NOT a bug -- rustkyll already matches Jekyll behavior for portfolio dates.

#### Results

- Build: `cargo build` passes
- Clippy: `cargo clippy -- -D warnings` passes
- Format: `cargo fmt --check` passes
- Tests: 3107 lib tests + 277 integration tests, 0 failures
- DTC DOM: 790/790 (no regression)
- academicpages DOM: 27/45 (no regression; per-page diff counts improved significantly for affected pages but they still have pre-existing tag ordering and excerpt rendering diffs)
- Post pages: `page__meta` with `fa-clock` read-time block now renders correctly
- Archive pages (tags, year-archive): read-time `page__meta` blocks now render correctly
- Files modified: `src/generator.rs`
