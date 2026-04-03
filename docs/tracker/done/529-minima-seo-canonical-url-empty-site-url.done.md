# Issue 529: SEO tag emits canonical/og:url when site.url is absent from _config.yml

## Problem

When `_config.yml` does not contain a `url:` key, Jekyll's `jekyll-seo-tag` treats
`site.url` as nil and does NOT emit `<link rel="canonical">` or `<meta property="og:url">`.

Rustkyll's config parser defaults `url` to `""` (empty string) when the key is absent.
The empty string is then inserted into the Liquid context as `site.url = ""`. The
`get_nested_str_allow_empty_non_nil` function in `seo_tag.rs` returns `Some("")`
for this value (since it is not nil), causing the canonical URL computation at line 607
to produce `canonical_url = Some("/page/path")` -- a relative canonical that Jekyll
would never emit.

Issue 425e fixed the case where `site.url` is truly nil, but `config.url` always defaults
to `""`, so `site.url` is never nil in the Liquid context.

### Affected sites

All sites without an explicit `url:` key in `_config.yml`, including:
- **minima** (all 9 pages): extra `<link rel="canonical">` and `<meta property="og:url">`
- Potentially many other theme benchmark sites

### Example

Jekyll (correct -- no canonical when site.url absent):
```html
<meta property="og:type" content="article" />
<meta property="article:published_time" content="2016-05-20T00:00:00+02:00" />
```

Rustkyll (wrong -- emits canonical with relative path):
```html
<link rel="canonical" href="/2016/05/20/my-example-post.html" />
<meta property="og:url" content="/2016/05/20/my-example-post.html" />
<meta property="og:type" content="article" />
<meta property="article:published_time" content="2016-05-20T00:00:00+02:00" />
```

## Root Cause

Two locations:

1. `src/config.rs:169` -- defaults `url: String::new()` (empty string)
2. `src/generator.rs:285` -- inserts `site.url = ""` into Liquid context unconditionally

The SEO tag's `get_nested_str_allow_empty_non_nil` correctly handles nil, but the
value is never nil because config always provides an empty string default.

## Proposed Fix

Option A (preferred): Track whether `url` was explicitly set in `_config.yml`. If not,
do not insert it into the Liquid context (or insert Nil). This matches Jekyll behavior
where missing keys are nil, not empty string.

Option B: In `seo_tag.rs`, treat `Some("")` the same as `None` for the site_url check.
This would be simpler but may break sites that explicitly set `url: ""`.

## Dependencies

None. Issue 425e is already done.

## Scope

- Fix the site.url nil vs empty-string distinction
- Verify minima canonical/og:url suppressed
- Verify sites with explicit `url: ""` still work (they should emit canonical)
- Verify sites with explicit `url: "https://example.com"` still work

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` produces no changes
- [ ] `cargo test` passes
- [ ] DTC DOM baseline: 790/790 (must not regress)
- [ ] Minima: pages do NOT contain `<link rel="canonical">` when `url:` is absent from config
- [ ] Minima: pages do NOT contain `<meta property="og:url">` when `url:` is absent from config
- [ ] Sites with explicit `url: "https://..."` still emit canonical/og:url correctly
- [ ] At least 3 new unit tests covering absent vs empty vs explicit url handling

## Test Scenarios

### Unit: site.url nil detection
- Config with no `url:` key -> `site.url` is nil in Liquid context
- Config with `url: ""` -> `site.url` is `""` (Some("")) in Liquid context
- Config with `url: "https://example.com"` -> normal behavior preserved

### Unit: SEO tag canonical suppression
- SEO tag with nil site.url -> no canonical link, no og:url
- SEO tag with empty string site.url -> canonical emitted with relative path (Jekyll compat)
- SEO tag with full site.url -> canonical emitted with absolute URL

### Integration: minima build
- Build minima site, verify no `rel="canonical"` in any HTML output
- Build minima site, verify no `property="og:url"` in any HTML output

## Baselines

- DTC: 790/790
- Minima: 0/9 (this fix should eliminate 2 diffs per page = ~18 diffs)

## Log

### [SWE] 2026-03-30

- TDD: Wrote 7 failing tests first (4 in config.rs, 3 in generator.rs)
  - config tests: url_explicitly_set true/false for present/absent/empty/null url
  - generator tests: site.url Nil when absent, empty string when explicit "", full URL when set
- Verified tests fail: compilation error (url_explicitly_set field does not exist)
- Implemented fix:
  - Added `url_explicitly_set: bool` field to `SiteConfig` with `#[serde(skip)]`
  - Modified `from_yaml_str` to parse YAML as Value first, check if `url` key present in mapping
  - Modified `build_site_context` in generator.rs to insert `Nil` when url not explicitly set
- All 7 new tests pass
- Full test suite: 3437 passed, 0 failed, 2 ignored
- clippy clean, fmt clean
- DTC DOM: 790/790 (no regression)
- Chirpy DOM: 12/17 (same as baseline, no regression from url: "" handling)
- Files modified: src/config.rs, src/generator.rs
