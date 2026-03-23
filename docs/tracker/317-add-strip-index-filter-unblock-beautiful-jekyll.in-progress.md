# Issue 317: Add `strip_index` Liquid filter, unblock beautiful-jekyll site

## Problem

beautiful-jekyll (5 pages) fails to render entirely because of a missing
`strip_index` Liquid filter. Every page produces empty output with the error:

```
template render error: liquid: Unknown filter
  requested filter=strip_index
```

The filter is used in `_includes/head.html` for canonical URLs and Open Graph
meta tags:

```liquid
<meta property="og:url" content="{{ page.url | absolute_url | strip_index }}">
<link rel="canonical" href="{{ page.url | absolute_url | strip_index }}">
```

`strip_index` is a built-in Jekyll URL filter defined in
`lib/jekyll/filters/url_filters.rb`. It removes a trailing `/index.html` or
`/index.htm` from a URL, turning `https://example.com/about/index.html` into
`https://example.com/about/`.

Ruby implementation (from Jekyll source):

```ruby
def strip_index(input)
  return if input.nil? || input.to_s.empty?
  input.sub(%r!/index\.html?$!, "/")
end
```

This filter is also used by jekyll-docs and potentially other sites. It is a
standard Jekyll filter that rustkyll should support.

Note: Issue 300 already implemented similar logic for the canonical URL in
`seo_tag.rs` (stripping `index.html` from homepage canonical URLs). The
`strip_index` filter is the general-purpose Liquid filter equivalent.

## Scope

### In scope

1. **Implement `strip_index` Liquid filter** -- register it alongside the other
   URL filters (absolute_url, relative_url). The filter removes trailing
   `/index.html` or `/index.htm` from the input string, replacing it with `/`.
   If the input is nil/empty, return empty string.

2. **Verify beautiful-jekyll renders correctly** -- after adding the filter, all
   5 pages should render with layouts applied (not empty). Run DOM comparison
   against Jekyll cached output.

### Out of scope

- Any other missing filters for other sites (each would be a separate issue)
- Fixing DOM diffs in beautiful-jekyll beyond what the filter unblocks

## Dependencies

- None. This is an independent filter implementation.

## Key Files to Modify

- `src/template/filters/mod.rs` or `src/template/filters/url.rs` -- add the
  `strip_index` filter implementation and registration
- `src/template/engine.rs` or wherever filters are registered with the Liquid
  engine -- register `strip_index`

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new tests below
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes
- [ ] `strip_index` filter is registered and available in Liquid templates
- [ ] `"https://example.com/about/index.html" | strip_index` produces
      `"https://example.com/about/"`
- [ ] `"https://example.com/about/index.htm" | strip_index` produces
      `"https://example.com/about/"`
- [ ] `"https://example.com/about/" | strip_index` produces
      `"https://example.com/about/"` (no change when no index.html)
- [ ] `"" | strip_index` produces `""` (empty input returns empty)
- [ ] `"https://example.com/index.html/extra" | strip_index` produces
      `"https://example.com/index.html/extra"` (only strips at end of string)
- [ ] beautiful-jekyll builds without warnings about `strip_index`
- [ ] beautiful-jekyll pages render with full HTML (not empty files) -- all 5
      common pages produce `<html><head>...</head><body>...</body></html>`
- [ ] beautiful-jekyll DOM comparison matches 3+/5 pages (allowing for other
      minor diffs unrelated to strip_index)
- [ ] No regressions on DTC (must remain 745+/790)
- [ ] No regressions on muan-blog (must remain 2174+/2218)
- [ ] No regressions on any of the 13+ sites currently at 100%
- [ ] Tests include non-ASCII content (URL with Unicode path segments)

## Test Scenarios

### Unit: strip_index filter basic behavior

- Input `"https://example.com/about/index.html"` -> output `"https://example.com/about/"`
- Input `"https://example.com/about/index.htm"` -> output `"https://example.com/about/"`
- Input `"https://example.com/"` -> output `"https://example.com/"` (unchanged)
- Input `""` -> output `""`
- Input `"/index.html"` -> output `"/"`
- Input `"https://example.com/index.html"` -> output `"https://example.com/"`

### Unit: strip_index edge cases

- Input `"https://example.com/index.html/page"` -> output unchanged (index.html
  not at end)
- Input `"https://example.com/my-index.html"` -> output unchanged (not exactly
  `/index.html`)
- Input `"https://example.com/INDEX.HTML"` -> output unchanged (case-sensitive,
  matching Jekyll behavior)
- Input with Unicode path: `"https://example.com/cafe/index.html"` -> output
  `"https://example.com/cafe/"`

### Integration: beautiful-jekyll site build

- Build beautiful-jekyll with rustkyll
- Verify no "Unknown filter: strip_index" warnings in build output
- Verify `index.html` is not empty (has `<html>` tag)
- Verify `aboutme/index.html` is not empty
- Run DOM comparison against Jekyll cached output
- Verify match count is 3+/5

### Integration: Regression check

- Run `cargo test` full suite
- Build DTC and verify no regression
- Verify all sites currently at 100% remain at 100%

## Output Verification

```bash
./scripts/cargo-safe build --release
./target/release/rustkyll build \
  --source websites/beautiful-jekyll/ \
  --destination /tmp/beautiful_jekyll_317

# Verify no strip_index warnings
./target/release/rustkyll build \
  --source websites/beautiful-jekyll/ \
  --destination /tmp/beautiful_jekyll_317 2>&1 | grep -c "strip_index"
# Must be 0

# Verify pages are not empty
wc -c /tmp/beautiful_jekyll_317/index.html
# Must be > 1000 (not 0)

# DOM comparison
python3 scripts/dom_compare.py \
  --jekyll-dir websites/beautiful-jekyll/_site_jekyll_cached \
  --rustkyll-dir /tmp/beautiful_jekyll_317
```

Spot-checks:
- `grep '<html' /tmp/beautiful_jekyll_317/index.html` -- must show `<html` tag
- `grep 'canonical' /tmp/beautiful_jekyll_317/index.html` -- must show canonical
  link without `/index.html` suffix
- `grep 'og:url' /tmp/beautiful_jekyll_317/aboutme/index.html` -- must show OG
  URL without `/index.html` suffix

## Log

### [SWE] 2026-03-23
- Created `src/template/filters/strip_index.rs` with `StripIndex` filter
- Wrote 11 unit tests first (TDD): basic behavior, edge cases, Unicode (cafe, cyrillic)
- Ran tests: all 11 PASS
- Registered filter in `src/template/filters/mod.rs` and `src/template/engine.rs`
- Clippy: clean (fixed `manual_strip` lint by using `strip_suffix`)
- Format: clean after `cargo fmt`
- Full test suite: 2843 passed, 0 failed
- Files created: `src/template/filters/strip_index.rs`
- Files modified: `src/template/filters/mod.rs`, `src/template/engine.rs`
