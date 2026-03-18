# Issue 208: Release v0.2.3

## Changes in this release

### Windows ARM64 support
- Issue #65: Windows ARM64 build support in release workflow
- Fix: Windows ARM64 Python wrapper platform mapping

### DOM compatibility improvements (57% → 62%)
- Issue #176: Inline code class at markdown event level
- Issue #177: XML syntax highlighting matches Rouge
- Issue #178: url_encode and cgi_escape use + for spaces
- Issue #180: Fix JSON string token splitting in syntax highlighting
- Issue #181: Fix JSON-LD date timezone offset
- Issue #182: Fix category/tag iteration order to match Jekyll
- Issue #183: Remove language-plaintext from wrapper div class
- Issue #184: Fix jekyll-seo-tag JSON-LD @type, url, name fields
- Issue #185: Fix JSON-LD whitespace corruption in markdownify
- Issue #186: Fix per-post related_posts to exclude current post
- Issue #187: Fix date formatting missing leading zeros
- Issue #188: Fix body class attribute for collection pages
- Issue #189: Fix permalink .html extension for pretty URLs
- Issue #190: Fix redirect pages to use absolute URLs
- Issue #191: Fix ampersand handling in heading IDs
- Issue #192: Fix title tag description/tagline suffix
- Issue #193: Fix YAML string token splitting for large-docs-site
- Issue #194: Fix muan-blog body class regression
- Issue #195: Fix SEO meta tag differences
- Issue #196: Fix layout not applied (shift filter, noop tags, nil indexing, lenient math)
- Issue #197: Fix remaining syntax highlighting (JS tokens)
- Issue #198: Fix content text/ordering (ZWSP emphasis, MediaWiki quotes)
- Issue #199: Fix markdown block structure (definition lists, text after HTML blocks)
- Issue #200: Fix markdown table rendering (kramdown pipe tables)
- Issue #201: Normalize bare void elements to XHTML-style
- Issue #202: Fix JSON-LD headline and soft break whitespace
- Issue #203: Fix missing HTML elements (text after block close tags)
- Issue #204: Fix extra HTML elements in list items
- Issue #205: Fix Cyrillic heading IDs in slugify
- Issue #206: Fix markdown inline formatting
- Issue #207: Fix content link href diffs (percent-encoding)

### CI improvements
- Fix integration CI: install uv for dom_compare.py
- Fix CI: handle missing _config.yml, relax perf test threshold

## Version

Patch (0.2.3): bug fixes, compatibility improvements

## Acceptance criteria

- [ ] CHANGELOG.md updated with release notes
- [ ] Version bumped in Cargo.toml, python/pyproject.toml, python/rustkyll/__init__.py
- [ ] All tests pass locally
- [ ] CI is green
- [ ] Tag v0.2.3 pushed
- [ ] Release workflow completes: 6 binaries built, GitHub Release created
- [ ] PyPI publish succeeds with 6 platform wheels
- [ ] `uvx rustkyll --help` works on Linux
- [ ] Release notes written (not just auto-generated)
