# Changelog

## v0.2.3 (2026-03-18)

### Windows ARM64 support

- Fixed Python wrapper platform mapping for Windows ARM64 (`uvx rustkyll` now works on win32 ARM64)

### DOM compatibility improvements (57% -> 62%)

Major push to eliminate DOM differences between rustkyll and Jekyll output across 35+ benchmark sites.

**Syntax highlighting:**
- Fixed JSON string token splitting for large-docs-site (301/801 -> 801/801, 100%)
- Fixed YAML string token: only opening quote gets `s2` class
- Fixed JS `var`/`function` as `kd`, identifiers as `nx` (fixes 10 theme sites)
- Fixed XML syntax highlighting to match Rouge

**SEO and meta tags:**
- Fixed jekyll-seo-tag JSON-LD `@type`, `url`, `name` fields
- Fixed JSON-LD date timezone offset (chrono-tz)
- Fixed JSON-LD whitespace corruption in markdownify
- Fixed JSON-LD headline and soft break whitespace
- Fixed title tag description/tagline suffix
- Fixed SEO description fallback from page content, og:locale from page.lang
- Fixed `article:published_time` meta tag

**Markdown/kramdown compatibility:**
- Normalized bare void elements to XHTML-style (`<br />`, `<hr />`)
- Fixed kramdown pipe table conversion
- Fixed kramdown definition list rendering
- Fixed text after HTML block close tags parsed as markdown
- Fixed extra HTML elements in list items
- Fixed markdown inline formatting (ZWSP emphasis, word-boundary emphasis)
- Fixed MediaWiki consecutive single quote protection
- Fixed content link href percent-encoding

**Layout and template engine:**
- Fixed layout not applied: registered shift filter, added feed_meta/github_edit_link no-op tags
- Fixed nil array indexing in Liquid (returns nil instead of error)
- Added lenient math filters (times/plus/minus coerce strings to 0)
- Fixed category/tag iteration order (BTreeMap for sorted output)

**URLs and permalinks:**
- Fixed permalink .html extension for pretty URLs
- Fixed redirect pages to use absolute URLs
- Fixed Cyrillic heading IDs in slugify
- Fixed ampersand handling in heading IDs
- Fixed url_encode and cgi_escape to use + for spaces
- Fixed date formatting missing leading zeros

**Other fixes:**
- Fixed body class attribute for collection pages
- Fixed per-post related_posts to exclude current post
- Removed language-plaintext from wrapper div class
- Fixed muan-blog body class regression

### CI improvements

- Fixed integration CI: install uv for dom_compare.py
- Fixed CI: handle missing _config.yml, relax perf test threshold

## v0.2.2 (2026-03-12)

- Cross-platform release workflow (6 binaries: linux/macOS/Windows x amd64/arm64)
- Installable via `uvx rustkyll` / `uv tool install rustkyll`
- DOM comparison testing infrastructure

## v0.2.1 (2026-03-10)

- Performance optimizations
- Extended Jekyll compatibility

## v0.2.0 (2026-03-08)

- Initial public release
- Full Jekyll site generation for DataTalks.Club and 35+ benchmark sites
