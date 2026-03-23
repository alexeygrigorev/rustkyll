# Issue 321: `relative_url` filter must pass through absolute URLs unchanged

## Problem

Jekyll's `relative_url` filter checks whether the input already starts with a
protocol scheme (`http://` or `https://`). If it does, the filter returns the
input unchanged. This matches the behavior of `absolute_url`, which already
handles this correctly in rustkyll.

Rustkyll's `relative_url` implementation does NOT check for absolute URLs. When
the input is an external URL like `https://beautifuljekyll.com`, the filter
either prepends `baseurl` or prepends `/`, producing broken URLs like
`/https://beautifuljekyll.com`.

### Impact

This directly breaks **beautiful-jekyll** (5 pages). The nav template in
`_includes/navbar-links.html` uses `{{ link[1] | relative_url }}` where
`link[1]` comes from `site.navbar-links` in `_config.yml`. Three nav entries
are external URLs:

```yaml
navbar-links:
  Beautiful Jekyll: "https://beautifuljekyll.com"
  Learn markdown: "https://www.markdowntutorial.com/"
  Author's home: "https://deanattali.com"
```

All 5 beautiful-jekyll pages have 3 broken nav hrefs each (15 diffs total from
this bug alone). The `absolute_url` filter already handles this correctly (see
`src/template/filters/absolute_url.rs` lines 29-31), so the fix for
`relative_url` should follow the same pattern.

This is also a correctness issue for any Jekyll site that passes external URLs
through `relative_url` in templates or includes.

### Root cause

In `src/template/filters/relative_url.rs`, the `evaluate` method on
`RelativeUrlFilter` has two branches:

1. When `baseurl` is set: prepends baseurl + `/` to the path
2. When `baseurl` is not set: prepends `/` if path doesn't start with `/`

Neither branch checks whether the input is already an absolute URL. The fix is
to add an early return (like `absolute_url` does) before either branch.

## Scope

### In scope

1. **Add absolute URL detection to `relative_url` filter** -- before any path
   manipulation, check if the input starts with `http://` or `https://`. If so,
   return it unchanged (matching Jekyll and matching the existing `absolute_url`
   behavior).
2. **Add tests** for the passthrough behavior.
3. **Verify beautiful-jekyll nav links render correctly** after the fix.

### Out of scope

- Other beautiful-jekyll diffs (missing nav `<div>`, content diffs on post
  pages, meta tag ordering). These are separate issues.
- Fixing `absolute_url` (it already works correctly).
- Protocol-relative URLs (`//example.com`) -- Jekyll does not special-case
  these and neither should we.

## Dependencies

- None. Independent fix.

## Key Files to Modify

- `src/template/filters/relative_url.rs` -- add early return for absolute URLs
  in the `evaluate` method (approximately 3 lines of code)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes
- [ ] `"https://example.com/path" | relative_url` returns
      `"https://example.com/path"` unchanged (no baseurl)
- [ ] `"https://example.com/path" | relative_url` returns
      `"https://example.com/path"` unchanged (WITH baseurl set)
- [ ] `"http://example.com/" | relative_url` returns `"http://example.com/"`
      unchanged
- [ ] Relative paths continue to work: `"/about.html" | relative_url` still
      returns `"/about.html"` (no baseurl) or `"/blog/about.html"` (with
      baseurl `/blog`)
- [ ] beautiful-jekyll nav links render with correct external URLs (no `/`
      prefix on `https://` URLs)
- [ ] beautiful-jekyll DOM comparison: 3 pages with only the nav `<div>` diff
      remaining (404.html, aboutme/index.html should drop from 4 diffs to 1)
- [ ] No regressions: DTC remains 746+/790, muan-blog remains 2172+/2218
- [ ] All sites currently at 100% remain at 100%
- [ ] Tests include non-ASCII content (external URL with Unicode path)

## Test Scenarios

### Unit: absolute URL passthrough

- Input: `"https://example.com"` with no baseurl -> output: `"https://example.com"`
- Input: `"https://example.com/path"` with no baseurl -> output: `"https://example.com/path"`
- Input: `"http://example.com/"` with no baseurl -> output: `"http://example.com/"`
- Input: `"https://example.com"` WITH baseurl `/blog` -> output:
  `"https://example.com"` (baseurl must NOT be prepended)
- Input: `"https://example.com/cafe/%C3%A9"` (URL with encoded Unicode) -> output unchanged

### Unit: relative paths still work (regression)

- Input: `"/about.html"` with no baseurl -> output: `"/about.html"`
- Input: `"images/photo.jpg"` with no baseurl -> output: `"/images/photo.jpg"`
- Input: `"/about.html"` with baseurl `/blog` -> output: `"/blog/about.html"`
- Input: `""` with no baseurl -> output: `""`

### Integration: beautiful-jekyll nav

- Build beautiful-jekyll with rustkyll
- Verify `index.html` contains `href="https://beautifuljekyll.com"` (not
  `href="/https://beautifuljekyll.com"`)
- Verify `index.html` contains `href="https://www.markdowntutorial.com/"` (not
  `href="/https://www.markdowntutorial.com/"`)
- Verify `index.html` contains `href="https://deanattali.com"` (not
  `href="/https://deanattali.com"`)
- Run DOM comparison, verify improvement from 0/5

### Integration: Regression check

- Build DTC, verify no regression (746+/790)
- Build muan-blog, verify no regression (2172+/2218)
- Run `cargo test` full suite
- Verify all 100% sites remain at 100%

## Output Verification

```bash
./scripts/cargo-safe build --release

# beautiful-jekyll
./target/release/rustkyll build \
  --source websites/beautiful-jekyll/ \
  --destination /tmp/bj_321

# Verify no /https: in nav links
grep 'href="/https:' /tmp/bj_321/index.html
# Must find 0 matches

# Verify correct external URLs
grep 'href="https://beautifuljekyll.com"' /tmp/bj_321/index.html
# Must find a match

grep 'href="https://deanattali.com"' /tmp/bj_321/index.html
# Must find a match

# DOM comparison
uv run scripts/dom_compare.py \
  --jekyll-dir websites/beautiful-jekyll/_site_jekyll_cached \
  --rustkyll-dir /tmp/bj_321
# 404.html, aboutme: should drop from 4 diffs to 1

# Regression
./target/release/rustkyll build \
  --source websites/DataTalksClub/datatalksclub.github.io \
  --destination /tmp/dtc_321
uv run scripts/dom_compare.py \
  --jekyll-dir websites/DataTalksClub/datatalksclub.github.io/_site_jekyll_cached \
  --rustkyll-dir /tmp/dtc_321
# Must remain 746+/790
```
