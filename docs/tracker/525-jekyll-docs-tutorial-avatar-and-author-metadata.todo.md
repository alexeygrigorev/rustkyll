# Issue 525: jekyll-docs tutorial pages author avatar URL and metadata differences

## Problem

All 8 tutorial pages in jekyll-docs have author avatar/metadata differences:

1. **Avatar URL format**: `avatars1.githubusercontent.com` (v=3) vs
   `avatars.githubusercontent.com` (v=4). Jekyll's cached output uses the old
   GitHub avatar URL format. This is a cache staleness issue, not a rendering bug.
2. **`data-proofer-ignore` attribute**: Jekyll output has
   `data-proofer-ignore='true'` on avatar `<img>` tags; rustkyll does not.
3. **Missing `srcset` format**: The srcset URL patterns differ (avatars1 vs avatars).

### Affected pages (8 tutorials)

- tutorials/cache-api/index.html
- tutorials/convert-site-to-jekyll/index.html
- tutorials/csv-to-table/index.html
- tutorials/custom-404-page/index.html
- tutorials/navigation/index.html
- tutorials/orderofinterpretation/index.html
- tutorials/using-jekyll-with-bundler/index.html
- tutorials/video-walkthroughs/index.html

### Analysis

The avatar URL difference (`avatars1` vs `avatars`, `v=3` vs `v=4`) is caused by
GitHub changing their avatar URL format. The Jekyll cached output is old. This is
NOT a rustkyll bug -- it is a cache staleness issue. If Jekyll were re-run today,
it would produce the same `avatars.githubusercontent.com/v=4` URLs.

The `data-proofer-ignore` attribute IS a rustkyll issue. The jekyll-docs theme's
author include template adds this attribute to avatar images, and our include
rendering is not producing it.

## Scope

1. **Avatar URLs**: Mark as UNFIXABLE (cache staleness). These diffs should be
   excluded from the fixable count.
2. **data-proofer-ignore**: Investigate whether this comes from the theme's
   include template and fix if it's a template rendering issue.

## Dependencies

None.

## DTC DOM Baseline

- Current: 790/790
- Must not drop below: 790/790

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes clean
- [ ] `cargo fmt` produces no changes
- [ ] Avatar `<img>` tags include `data-proofer-ignore='true'` attribute when the template specifies it
- [ ] DTC DOM match count must not drop below 790/790
- [ ] Tutorial pages that had data-proofer-ignore diffs are resolved

## Test Scenarios

### Unit: Include template attribute passthrough

- Template with `<img data-proofer-ignore="true" ...>` renders the attribute
- Verify attribute appears in HTML output

### Integration: jekyll-docs site

- Build jekyll-docs, check tutorials/cache-api avatar img for data-proofer-ignore
- Run DOM comparison, verify tutorial page avatar diffs are only URL format (unfixable)
