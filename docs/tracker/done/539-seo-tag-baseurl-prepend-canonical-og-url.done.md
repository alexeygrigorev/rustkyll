# Issue 539: SEO tag baseurl prepend for canonical/og:url/JSON-LD URLs

## Problem

When a site sets `baseurl` in `_config.yml` (e.g. `baseurl: "/example"`), Jekyll's
`jekyll-seo-tag` automatically prepends `site.baseurl` to canonical URLs, `og:url`,
and JSON-LD `url` fields. Rustkyll's `src/template/seo_tag.rs` does not apply
`site.baseurl` when constructing these values.

This causes ~180 DOM differences on the basically-basic example site alone, and
will affect any site that uses a non-empty `baseurl`.

### Expected (Jekyll)

```html
<link rel="canonical" href="/example/404.html" />
<meta property="og:url" content="/example/404.html" />
```

### Actual (rustkyll)

```html
<link rel="canonical" href="/404.html" />
<meta property="og:url" content="/404.html" />
```

## Root Cause

`src/template/seo_tag.rs` constructs canonical/og:url values from `page.url` without
prepending `site.baseurl`. The JSON-LD `url`, `mainEntityOfPage.@id`, and
`publisher.logo.url` fields also miss the prefix.

## Acceptance Criteria

- [ ] Canonical URL includes `site.baseurl` prefix when set
- [ ] `og:url` includes `site.baseurl` prefix when set
- [ ] JSON-LD `url` and `mainEntityOfPage.@id` include `site.baseurl` prefix
- [ ] JSON-LD `publisher.logo.url` includes `site.baseurl` prefix
- [ ] Sites with empty/unset baseurl are unaffected
- [ ] DTC DOM baseline must not regress

## Dependencies

- Discovered in #355 (basically-basic triage)

## Log

### [SWE] 2026-03-30
- Wrote 8 failing tests (test_issue539_*) covering canonical URL, og:url, JSON-LD url, mainEntityOfPage @id, publisher logo URL, empty baseurl, no baseurl, and index.html stripping with baseurl
- Ran tests: 6 FAIL as expected (2 pass for no-baseurl/empty-baseurl cases)
- Root cause: `site.baseurl` was never read in `seo_tag.rs`; canonical_url and absolute_image_url did not include it
- Fix: Read `site.baseurl` from runtime context, prepend to canonical_url construction, and pass to `absolute_image_url` (which now takes a `baseurl` parameter)
- Ran tests: all 8 issue-539 tests PASS
- Full suite: 3472 lib tests + all integration tests pass, 0 failures
- Clippy: clean (no warnings from our code)
- Fmt: clean
- Files modified: `src/template/seo_tag.rs`
