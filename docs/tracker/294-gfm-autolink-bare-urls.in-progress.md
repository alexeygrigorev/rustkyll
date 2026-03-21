# Issue 294: Enable GFM autolink for bare URLs in CommonMarkGhPages sites

## Problem

Sites using `markdown: CommonMarkGhPages` with the `autolink` extension (in `commonmark.extensions`) expect bare URLs like `https://example.com` to be automatically converted to clickable `<a>` links. Currently, rustkyll does not enable this GFM feature, so bare URLs appear as plain text in the output.

This affects 18 pages on muan-blog and potentially other CommonMarkGhPages sites.

## Root Cause

In `src/frontmatter.rs`, the `markdown_to_html_with_options` function sets up pulldown-cmark options but does not enable `Options::ENABLE_GFM`, which includes GFM autolinks (bare URL auto-linking). The site config declares `extensions: ["autolink"]` but this is never read or applied to the pulldown-cmark parser options.

pulldown-cmark 0.13 provides `Options::ENABLE_GFM` which enables "Misc GitHub Flavored Markdown features not supported in CommonMark" including GFM autolinks.

## Affected Pages

All 18 are on **muan-blog** (which uses `markdown: CommonMarkGhPages` with `extensions: ["strikethrough", "autolink", "table"]`):

- `notes.html` -- 3 bare URLs not linked (Wikipedia URLs)
- `notes/2019-12-06-zz.html` -- YouTube URL not linked
- `notes/2020-02-14-rr.html` -- YouTube URL not linked
- `notes/2020-09-07-zz.html` -- herokuapp URL not linked
- `notes/2022-01-23-rr.html` -- blog.mollywhite.net URL not linked
- `notes/2024-06-25-uu.html` -- openstories.fyi URL not linked
- `notes/2024-10-15-uu.html` -- en.zakka.reviews URL not linked
- `notes/2024-11-21-uu.html` -- pagespeed.web.dev URL not linked
- `notes/2025-08-29-aa.html` -- bare URL not linked
- And 9 more notes/posts with similar patterns

Each affected page shows:
- `text_differs`: The paragraph text includes the raw URL where Jekyll hid it inside an `<a>` tag
- `missing_element`: The `<a>` element is absent in rustkyll output

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes all existing tests plus new tests
- [ ] When site config has `markdown: CommonMarkGhPages` with `extensions: ["autolink"]`, bare URLs in markdown content are auto-linked to `<a>` tags
- [ ] Bare URL `https://example.com` in a paragraph renders as `<a href="https://example.com">https://example.com</a>`
- [ ] Bare URL `http://example.com` (without https) also auto-links
- [ ] URLs inside existing markdown links `[text](url)` are NOT double-linked
- [ ] URLs inside code spans and code blocks are NOT auto-linked
- [ ] URLs already wrapped in `<url>` angle brackets continue to work as before
- [ ] The feature is ONLY enabled for CommonMarkGhPages sites with the autolink extension, NOT for kramdown sites
- [ ] DOM comparison recount shows improvement:
  - muan-blog: at least 14 of the 18 autolink-affected pages now match
- [ ] No regressions on kramdown sites (DTC, mlbookcamp, kids-horror-stories-ru, etc.)
- [ ] No regressions on other CommonMarkGhPages sites
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` is clean

## Test Scenarios

### Unit: Bare URL auto-linking

- Markdown `"Visit https://example.com for details."` with GFM autolinks enabled -> output contains `<a href="https://example.com">https://example.com</a>`
- Markdown `"Check http://old.example.com too."` -> output contains `<a href="http://old.example.com">http://old.example.com</a>`
- Markdown with URL at end of line: `"See https://example.com"` -> auto-linked
- Markdown with URL followed by punctuation: `"Go to https://example.com."` -> URL linked (period NOT included in URL)
- Markdown with URL followed by closing paren: `"(see https://example.com)"` -> URL linked (paren NOT included)

### Unit: No double-linking

- Markdown `"[Example](https://example.com)"` -> renders as single `<a>`, URL not double-linked
- Markdown `` `https://example.com` `` (code span) -> URL NOT auto-linked, stays as code

### Unit: Kramdown sites unaffected

- Same bare URL content processed with kramdown options -> URL stays as plain text (no auto-linking)

### Unit: Config parsing

- Config with `commonmark.extensions: ["autolink"]` -> `has_commonmark_autolink()` returns true (or equivalent)
- Config without autolink extension -> feature disabled
- Config with `markdown: kramdown` (default) -> feature disabled regardless of extensions

### Integration: muan-blog page rendering

- Build a minimal CommonMarkGhPages site with bare URLs in content
- Verify HTML output contains `<a>` tags for bare URLs
- Verify the surrounding text is correctly split (text before URL, URL as link, text after URL)

### Site-level: muan-blog DOM comparison

- Build muan-blog (or use cached output)
- Run DOM comparison on `notes/2025-08-29-aa.html` (pure autolink issue)
- Verify the page now matches

## Dependencies

None -- this is independent of other issues.

## Implementation Notes

- The fix is likely a small change in `src/frontmatter.rs` in the `markdown_to_html_with_options` function (and possibly `markdown_to_html`).
- Add a parameter or check site config for CommonMarkGhPages autolink extension.
- When enabled, add `options.insert(Options::ENABLE_GFM)` to the pulldown-cmark parser options.
- **Caution**: `ENABLE_GFM` may enable other GFM features beyond autolinks. Check what else it enables and ensure it doesn't break anything. If it enables too much, you may need to handle autolinks via a preprocessing step instead.
- The config already has `commonmark.extensions` parsing in `SiteConfig`. Check if `has_commonmark_autolink()` or similar method exists; if not, add one.
- The `markdown_to_html_with_options` function needs to know whether autolink is enabled. This may require adding a parameter or passing the site config through.

## Files Likely Affected

- `src/config.rs` -- Add method to check for autolink extension
- `src/frontmatter.rs` -- Enable `ENABLE_GFM` option when autolink extension is configured
- `src/template/layout.rs` -- Pass autolink config through to markdown rendering
- `src/collection.rs` -- May need to pass config through

## Log

### [SWE] 2026-03-21
- Wrote 13 failing tests in frontmatter.rs for bare URL autolinking (TDD step 1)
- Wrote 6 tests in config.rs for has_commonmark_autolink() config parsing
- Tests initially failed to compile (function signature mismatch) -- expected
- Implemented autolink_bare_urls() preprocessing in src/frontmatter.rs
  - Approach: wrap bare http/https URLs in angle brackets before pulldown-cmark parsing
  - pulldown-cmark 0.13's ENABLE_GFM only adds blockquote tags, NOT autolinks
  - So preprocessing approach is necessary (no regex dependency needed)
  - Handles code spans, code blocks, angle-bracket autolinks, markdown links, HTML <a> tags
- Added enable_autolink parameter to markdown_to_html_with_options()
- Added has_commonmark_autolink() to SiteConfig in config.rs
- Added enable_autolink field + set_autolink() to LayoutEngine in layout.rs
- Threaded enable_autolink through collection.rs and main.rs
- All 19 issue-294 tests pass
- Build: 2388 pass, 13 fail (all pre-existing issue-293 failures), clippy clean, fmt clean
- Files modified: src/config.rs, src/frontmatter.rs, src/template/layout.rs, src/collection.rs, src/main.rs
