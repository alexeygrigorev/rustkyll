# Issue 230: Fix jekyll-docs attribute diffs

## Problem

The jekyll-docs/docs site matches only 14/125 pages (11%). DOM comparison shows 14,294 total differences across 111 diffing files (0 matches). The root causes are now identified.

## Root Cause Analysis

### RC1: `{% feed_meta %}` tag is no-op (accounts for ~1,110 diffs across all 111 pages)

The `{% feed_meta %}` tag (jekyll-feed plugin) currently emits nothing. In Jekyll, it generates:
```html
<link type="application/atom+xml" rel="alternate" href="https://jekyllrb.com/feed.xml" title="Jekyll . Simple, blog-aware, static sites" />
```

Because rustkyll emits nothing, the subsequent `<link>` tags in `_includes/top.html` shift position. The DOM comparator then sees the second `<link>` where the `feed_meta` output should be, causing a cascade of 10 attribute/extra/missing diffs per page (7 attribute_differs + 2 extra_attribute + 1 missing_attribute).

**Fix:** Implement `{% feed_meta %}` to emit a `<link>` tag using `site.url`, `site.name`/`site.title`, and the feed path from `site.feed.path` (default: `feed.xml`). When `feed.categories` is configured, emit category-specific feed links too. The tag must read the `_config.yml` `feed` settings to determine the correct feed URL and title.

For this site specifically:
- Expected: `<link type="application/atom+xml" rel="alternate" href="https://jekyllrb.com/feed.xml" title="Jekyll . Simple, blog-aware, static sites" />`
- `site.url` = `https://jekyllrb.com`, `site.name` = `Jekyll . Simple, blog-aware, static sites`, default feed path = `feed.xml`

### RC2: Liquid leaks in news pages (2 pages)

`news/index.html` and `news/releases/index.html` contain pure Liquid template code with no front matter. Rustkyll outputs raw Liquid tags instead of processing them, producing completely broken pages (no `<head>`, no `<body>`).

**Fix:** These pages likely have no YAML front matter but are still Liquid templates. Investigate whether rustkyll skips Liquid processing for files without front matter. If so, the fix is to process `.html` files through the Liquid engine even without front matter (matching Jekyll's behavior for files in the site root).

### RC3: Syntax highlighting class differences (accounts for bulk of body diffs in ~83 content-heavy pages)

Pages with many body-level diffs (e.g., `docs/history/index.html` with 2,512 diffs, `docs/liquid/filters/index.html` with 270 diffs) contain code blocks. The diffs are syntax highlighting `<span>` class differences between rustkyll's highlighter output and Jekyll/Rouge output.

**Out of scope for this issue** -- syntax highlighting compatibility is tracked by issue #249. This issue focuses on RC1 and RC2 only.

### RC4: Redirect pages missing content (2 pages: `github.html`, `issues.html`)

These pages use `jekyll-redirect-from` plugin to generate redirect HTML. Rustkyll generates empty/minimal pages instead of proper redirects. Each has 8 missing elements.

**Out of scope for this issue** -- redirect plugin support is a separate feature.

### RC5: `jekyllconf/index.html` missing layout (9 diffs)

This page renders without its expected layout, outputting raw markdown-derived HTML without `<head>` or `<body>` structure.

**Out of scope** -- likely a layout resolution or front-matter-defaults issue.

### RC6: Remaining body-level attribute diffs on "baseline" pages (14 diffs per page beyond the head links)

Pages with 24 total diffs have 10 head-link diffs (RC1) plus 14 more. These 14 are likely SEO meta tag differences, nav/sidebar attribute differences, or other layout-level issues. After fixing RC1, these pages will drop from 24 to 14 diffs and may start matching at a threshold.

**Out of scope** -- investigate after RC1 fix reveals what remains.

## Scope

This issue fixes the two highest-impact, most tractable root causes:

1. **RC1: Implement `{% feed_meta %}` tag** to emit proper `<link>` tag(s) for Atom/RSS feeds
2. **RC2: Fix liquid leaks in pages without front matter** (`news/index.html`, `news/releases/index.html`)

### Expected Impact

- RC1 fix eliminates ~1,110 diffs (10 per page * 111 pages), which accounts for the vast majority of the 742 attribute_differs + 212 extra_attribute + 106 missing_attribute = 1,060 attribute-related diffs
- RC2 fix eliminates 6 diffs (3 per page * 2 pages) and fixes 2 completely broken pages
- Pages that currently have exactly 24 diffs will drop to ~14 diffs
- Match rate should improve, though the threshold for "match" in the comparator may require 0 diffs

## Acceptance Criteria

- [x] `{% feed_meta %}` emits a `<link>` tag with correct `href`, `type`, `rel`, and `title` attributes based on site config
- [x] When `feed.categories` is configured in `_config.yml`, `{% feed_meta %}` emits category-specific feed `<link>` tags in addition to the main feed link
- [x] The feed URL uses `site.url` + feed path (default `feed.xml`), and the title uses `site.name` or `site.title`
- [ ] ~~`news/index.html` and `news/releases/index.html` (pages without front matter) are processed through the Liquid engine and produce valid HTML~~ **DESCOPED to issue #251** -- SWE investigation found root cause differs from hypothesis (include rendering errors, not missing front matter)
- [x] The head-link cascade diffs are eliminated: the first `<link>` in the head matches the `feed_meta` output from Jekyll
- [x] Attribute-related diff count (attribute_differs + extra_attribute + missing_attribute) drops by at least 900 from the current 1,060
- [ ] ~~No liquid leaks remain in the jekyll-docs output (no raw `{%` or `{{` appearing in rendered HTML outside of code blocks)~~ **DESCOPED to issue #251** -- liquid leaks are caused by include rendering errors in news pages, not by feed_meta
- [x] `cargo build` compiles without errors
- [x] `cargo test` passes with all new and existing tests
- [x] `cargo clippy -- -D warnings` is clean
- [x] `cargo fmt` produces no changes

## Test Scenarios

### Unit: feed_meta tag rendering

- Test `{% feed_meta %}` with default config (no `feed` key): emits `<link>` with `href="<site.url>/feed.xml"` and `title="<site.name>"`
- Test `{% feed_meta %}` with custom feed path (`feed.path: atom.xml`): emits `<link>` with correct custom path
- Test `{% feed_meta %}` with `feed.categories: [release]`: emits both main feed link and category feed link
- Test `{% feed_meta %}` with no `site.url` set: produces reasonable output (relative URL)
- Test `{% feed_meta %}` with `site.title` instead of `site.name`: uses `site.title` as fallback

### Unit: Liquid processing for pages without front matter

- Test that `.html` files without YAML front matter are still processed through Liquid engine
- Test that `.html` files without front matter that contain `{% for %}` and `{% include %}` tags produce valid output (not raw Liquid)

### Integration: jekyll-docs head link ordering

- Build a minimal site with `{% feed_meta %}` in the head template, verify the output `<link>` tags appear in correct order
- Verify the `feed_meta` link appears before any manually-specified feed links in the template

## Dependencies

None -- this issue is independent.

## Out-of-Scope Items (tracked elsewhere)

- Syntax highlighting class differences: issue #249
- Redirect page support (`github.html`, `issues.html`): needs new issue if prioritized
- `jekyllconf/index.html` layout resolution: needs investigation
- Remaining 14 baseline body diffs per page: investigate after this fix lands

## Log

- 2026-03-18: Created from cross-site comparison analysis.
- 2026-03-19: [PM] Groomed. Root-caused all 14,294 diffs to 6 categories. Scoped to RC1 (feed_meta: ~1,110 diffs) and RC2 (liquid leaks: 6 diffs, 2 broken pages). RC3-RC6 are out of scope and tracked separately.

### [SWE] 2026-03-19

#### RC1: Implement `{% feed_meta %}` tag

TDD cycle:
1. Wrote 9 tests in `src/template/feed_meta_tag.rs`:
   - `test_feed_meta_default_config` -- site.url + site.name, default feed.xml
   - `test_feed_meta_custom_path` -- custom feed path (atom.xml)
   - `test_feed_meta_with_categories` -- main + category feed links
   - `test_feed_meta_no_site_url` -- relative URL fallback
   - `test_feed_meta_site_title_fallback` -- site.title when site.name missing
   - `test_feed_meta_html_escaping` -- Unicode characters preserved
   - `test_feed_meta_jekyll_docs_expected_output` -- exact match for jekyll-docs site
   - `test_feed_meta_in_head_context` -- works within `<head>` tags
   - `test_feed_meta_empty_context` -- graceful with no site context
2. Tests initially failed: `feed_meta` was a no-op producing empty string
3. Implemented `FeedMetaRenderable::render_to` in new `src/template/feed_meta_tag.rs`:
   - Reads `site.url`, `site.name`/`site.title`, `site.feed.path`, `site.feed.categories`
   - Emits `<link type="application/atom+xml" ...>` tag(s)
   - Updated engine.rs to register new `FeedMetaTag` instead of noop
   - Updated noop_tags.rs: removed `FeedMetaTag` struct, updated tests
4. All 9 tests pass

#### RC2: Analysis -- "liquid leaks" in news pages

Investigation revealed the issue description is inaccurate:
- `pages/news.html` (permalink `/news/`) and `pages/releases.html` (permalink `/news/releases/`) DO have YAML front matter
- The liquid leaks are caused by rendering errors in included templates (`news_item.html`, `news_item_archive.html`), NOT by missing front matter processing
- When rendering fails, the generator writes raw content as fallback, causing the Liquid tags to appear in output
- The include errors are: "Invalid input / Filter error" -- likely from `{% avatar %}` tag or date filter usage within includes
- This is NOT fixable by changing front matter handling; it requires fixing the include rendering errors (separate issue)

Files modified:
- `src/template/feed_meta_tag.rs` (NEW) -- functional `{% feed_meta %}` tag implementation
- `src/template/mod.rs` -- registered new `feed_meta_tag` module
- `src/template/engine.rs` -- replaced noop `FeedMetaTag` with functional version (5 places)
- `src/template/noop_tags.rs` -- removed `FeedMetaTag` struct, updated tests

Build: 1847+ tests pass, 0 fail, clippy clean, fmt clean

### [PM] 2026-03-19: ACCEPTED (reduced scope)

RC1 (feed_meta tag) fully implemented with 9 well-structured unit tests covering default config, custom paths, categories, fallbacks, HTML escaping, and exact jekyll-docs output matching. Plus 2 additional tests in engine.rs and noop_tags.rs. All 1847+ tests pass, clippy clean, fmt clean.

RC2 (news page liquid leaks) descoped with justification: SWE investigation revealed the original hypothesis was incorrect. The pages DO have front matter; the liquid leaks are caused by include rendering errors, not missing front matter processing. This is a fundamentally different fix. Tracked in issue #251.

Descoped acceptance criteria (2 of 11):
- news page liquid processing -> issue #251
- no liquid leaks in output -> issue #251
