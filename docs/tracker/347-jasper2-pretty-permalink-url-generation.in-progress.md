# Issue 347: Jasper2 pretty permalink and canonical URL generation

## Problem

Jasper2 uses `permalink: /:title` in `_config.yml` (no `.html` extension). Jekyll treats this as a "pretty" permalink, generating output at `/<title>/index.html` so that the URL `/<title>/` works without a file extension. In the rustkyll build, many Jasper2 links and canonical URLs still point at `.html` paths, creating repeated DOM diffs across the homepage, posts, and metadata.

## Root Cause

The Jasper2 permalink pattern `/:title` does not end with `.html`. Jekyll interprets this as producing a directory with an `index.html` inside (i.e., `/<title>/index.html`), and the URL exposed to templates is `/<title>/` (with trailing slash). rustkyll's permalink expansion may be appending `.html` instead of creating the directory structure, or the URL exposed to Liquid templates may include `.html` when it should end with `/`.

## Scope

1. Verify how rustkyll handles permalink patterns without `.html` extension (e.g., `/:title`, `/:categories/:title`).
2. Fix the URL generation so that `permalink: /:title` produces:
   - Output file: `<output_dir>/<title>/index.html`
   - URL in Liquid context (`page.url`, `post.url`): `/<title>/`
3. Fix canonical URLs in `<link rel="canonical">` and `<meta property="og:url">` tags to use the pretty URL (no `.html`).
4. Verify links on the homepage, post pages, and navigation all use the pretty URL format.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with tests covering permalink patterns without `.html`
- [ ] For `permalink: /:title`, posts are output to `<output_dir>/<title>/index.html` (directory-based)
- [ ] `page.url` / `post.url` in Liquid templates resolves to `/<title>/` (trailing slash, no `.html`)
- [ ] Canonical URLs in SEO tags use the pretty URL format
- [ ] The Jasper2 homepage post links use `/<title>/` format, not `/<title>.html`
- [ ] Navigation links in Jasper2 templates use pretty URLs
- [ ] The fix is generic -- any site with `permalink: /:title` or `permalink: /:categories/:title` benefits, not just Jasper2
- [ ] The Jasper2 DOM comparison improves from the #240 baseline (fewer URL-related diffs)
- [ ] DTC DOM count remains at 788/790 or above

## Test Scenarios

### Unit: permalink expansion without .html
- `permalink: /:title` with title `"my-post"` produces URL `/my-post/` and output path `<dir>/my-post/index.html`
- `permalink: /:categories/:title` with category `"tech"` and title `"intro"` produces URL `/tech/intro/` and output path `<dir>/tech/intro/index.html`
- `permalink: /:title.html` continues to produce URL `/<title>.html` and output path `<dir>/<title>.html` (no regression)

### Unit: URL trailing slash
- Verify `page.url` ends with `/` for pretty permalinks
- Verify `page.url` ends with `.html` for explicit `.html` permalinks
- Verify `url_to_output_path` handles trailing-slash URLs correctly (already has a test)

### Integration: Jasper2 post URLs
- Build `websites/jasper2/` with rustkyll and verify posts are written to `<title>/index.html` directories
- Inspect the homepage HTML for post links -- all should be `href="/<baseurl>/<title>/"` format
- Inspect a post page's canonical URL -- should be `<production_url>/<title>/`
- Verify the Atom feed entry links use pretty URLs

### Regression: DTC site
- Build DTC site and verify its `permalink: /blog/:title.html` pattern still produces `.html` URLs and flat file output
- Verify DTC DOM count remains at 788/790 or above

## Dependencies

- Issue #240 (must be `.done.md` or `.in-progress.md`)

## Log

### [SWE] 2026-03-30

- **TDD Step 1: Wrote 6 failing tests** in `src/collection.rs`:
  - `test_permalink_title_no_ext_produces_pretty_url`: `/:title` -> `/my-post/`
  - `test_permalink_categories_title_no_ext_produces_pretty_url`: `/:categories/:title` -> `/tech/intro/`
  - `test_permalink_title_html_no_trailing_slash`: `/:title.html` -> `/my-post.html` (no regression)
  - `test_permalink_blog_title_html_no_trailing_slash`: `/blog/:title.html` -> `/blog/my-post.html` (DTC pattern)
  - `test_permalink_pretty_named_style_trailing_slash`: `pretty` -> `/2024/01/15/my-post/`
  - `test_permalink_year_month_title_no_ext_produces_pretty_url`: `/:year/:month/:title` -> `/2024/01/my-post/`
- **Ran tests: 4 of 6 FAILED as expected** (the `.html` tests already passed since no change needed)
  - `test_permalink_title_no_ext_produces_pretty_url`: got `/my-post`, expected `/my-post/`
- **Implemented fix** in `src/collection.rs`:
  - Added `url_has_extension()` function to detect file extensions in URL paths
  - Modified `generate_url_with_context()` to append trailing `/` when URL has no file extension (pretty URL rule)
  - Fixed `process_collection_file()` id computation to strip trailing `/` before computing dirname (matching Ruby's `File.dirname` behavior)
- **Updated `resolve_link_post_url()`** in `src/template/engine.rs` to also apply the pretty URL rule for `{% link %}` tags pointing to posts
- **Updated 4 existing tests** with correct expected values:
  - `test_default_collection_permalink_no_html`: `/pages/banners` -> `/pages/banners/`
  - `test_generate_url_collection_path_pattern`: `/notes/2018-06-04-aa` -> `/notes/2018-06-04-aa/`
  - `test_generate_url_collection_path_unicode`: `/pages/uber-uns` -> `/pages/uber-uns/`
  - `test_link_tag_posts_uses_permalink_pattern`: added trailing `/` to pretty URL assertions
- **Ran all tests: 3503 lib tests pass, 0 fail; full suite all pass**
- **Clippy clean, fmt clean**
- Files modified: `src/collection.rs`, `src/template/engine.rs`
