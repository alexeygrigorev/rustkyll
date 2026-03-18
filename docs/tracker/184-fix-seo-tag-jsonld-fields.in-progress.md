# Issue 184: Fix jekyll-seo-tag JSON-LD field accuracy

## Checklist Categories

This issue covers three related checklist categories:
- **JSON-LD missing fields (url)** -- 18 pages
- **JSON-LD extra fields (name)** -- 9 pages
- **JSON-LD other value differences** -- 212 pages (partially; the `@type` selection logic subset)

## Problem

rustkyll's SEO tag JSON-LD output differs from Jekyll's jekyll-seo-tag in several fields:
- `@type` is `WebPage` when Jekyll uses `WebSite` (or vice versa)
- Missing `url` field in JSON-LD
- Extra `name` field present when Jekyll omits it (or vice versa)

Sample diff (theme sites):
```
jsonld.@type: jsonld_value_differs - expected: 'WebSite', actual: 'WebPage'
jsonld.url: jsonld_missing_field - expected: '"/"', actual: '(none)'
jsonld.name: jsonld_extra_field - expected: '(none)', actual: '"Architect theme"'
```

## Goal

Match jekyll-seo-tag's JSON-LD output field-for-field for `@type`, `url`, and `name` fields.

## Affected Sites

- All 9 theme sites (architect, cayman, dinky, hacker, midnight, merlot, slate, time-machine, leap-day): 18 pages for url, 9 pages for name
- DTC (some pages for @type and url)

## Dependencies

None.

## Approach (TDD)

1. Write tests for @type selection logic, url field inclusion, name field logic
2. Verify tests fail
3. Fix SEO tag implementation in `src/template/seo_tag.rs` and/or `src/jsonld.rs`
4. Verify tests pass
5. Recount theme sites

## Acceptance Criteria

- [ ] `@type` matches jekyll-seo-tag logic: `WebSite` for the homepage (when `page.url == "/"`) and `WebPage` for all other pages. Verify against jekyll-seo-tag source for exact rules.
- [ ] `url` field is included in JSON-LD when `site.url` is configured, matching jekyll-seo-tag's behavior (page URL relative to site)
- [ ] `name` field is only included when jekyll-seo-tag includes it (typically only for `WebSite` type, not for `WebPage`)
- [ ] All 9 theme sites show improved DOM match counts
- [ ] `cargo test` passes

## Test Scenarios

### Unit: @type selection (write FIRST, must fail before fix)

- **Test `test_jsonld_type_homepage_is_website`**: Render JSON-LD for a page with `url: "/"`. Assert `@type` is `WebSite`.
- **Test `test_jsonld_type_subpage_is_webpage`**: Render JSON-LD for a page with `url: "/about.html"`. Assert `@type` is `WebPage`.

### Unit: url field inclusion

- **Test `test_jsonld_includes_url_field`**: Render JSON-LD for a page in a site with `url: "https://example.com"`. Assert the JSON-LD contains a `url` field with the page's absolute URL.
- **Test `test_jsonld_no_url_without_site_url`**: Site without `url` configured. Assert `url` field is absent (or follows jekyll-seo-tag behavior).

### Unit: name field logic

- **Test `test_jsonld_website_includes_name`**: Render JSON-LD for homepage (`WebSite` type). Assert `name` field is present with site title.
- **Test `test_jsonld_webpage_no_name`**: Render JSON-LD for a subpage (`WebPage` type). Assert `name` field is NOT present.

### Integration: Full site verification

- Build all 9 theme sites and verify JSON-LD output matches Jekyll for @type, url, and name fields.
- Compare against Jekyll output for architect-theme `another-page.html` specifically.

## Log

### [SWE] 2026-03-18
- Analyzed jekyll-seo-tag Ruby source (drop.rb, json_ld_drop.rb) to understand exact logic
- Root causes identified:
  1. `@type`: Was only BlogPosting (with date) or WebPage. Missing WebSite for homepage/about pages matching `^/(about/)?(index.html?)?$`
  2. `name`: Was always included when full_title existed. Jekyll only includes name for homepage/about pages
  3. `url`: Was only included when canonical_url was computed (requires site_url). Jekyll includes url from page_url even without site_url
- TDD approach: wrote 9 new tests first, verified 5 failed, then fixed implementation
- Added `is_homepage_or_about_url()` helper function (pure Rust, no regex crate needed)
- Fixed @type logic: WebSite for homepage/about, BlogPosting for dated pages, WebPage otherwise
- Fixed name logic: only emit for homepage/about pages, using site_title
- Fixed url logic: fall back to page_url when canonical_url is unavailable
- Updated existing `test_json_ld_contains_name` test to match new correct behavior
- Tests: 1480+ unit tests pass, 0 fail; clippy clean; fmt clean
- Files modified: src/template/seo_tag.rs
