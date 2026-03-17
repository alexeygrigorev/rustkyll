# Issue 192: Fix title tag missing site description suffix

## Checklist Category

**Title tag missing site description suffix** -- 19 pages.

## Problem

The `<title>` element shows only the page title without the `| site.description` or `| site.tagline` suffix that Jekyll's jekyll-seo-tag appends. Jekyll outputs `Theme Name | Description` but rustkyll outputs just `Theme Name`.

Sample diff:
```
head > title: text_differs
  expected: 'How Do Data Professionals Use Data Engineering Tools and Practices? - DataTalks.Club'
  actual:   'How Do Professionals Use Data Engineering Tools and Practices? - DataTalks.Club'
```

Note: The DTC sample above may also involve a text truncation bug (missing "Data" word), which could be a separate issue. This issue focuses specifically on the `| site.description` suffix logic.

For theme sites:
```
head > title: text_differs
  expected: 'Architect theme | A GitHub pages theme'
  actual:   'Architect theme'
```

## Goal

Match jekyll-seo-tag's title generation logic, including appending `site.description` or `site.tagline` as a suffix where appropriate.

## Affected Sites

- DataTalksClub/datatalksclub.github.io: 1 page
- architect-theme: 2 pages
- cayman-theme: 1 page
- dinky-theme: 2 pages
- hacker-theme: 2 pages
- leap-day-theme: 2 pages
- merlot-theme: 2 pages
- midnight-theme: 2 pages
- opensource-guide: 1 page
- slate-theme: 2 pages
- time-machine-theme: 2 pages

## Dependencies

None.

## Approach (TDD)

1. Write a test that creates a site with `title: "My Site"` and `description: "A cool site"` and a page with `title: "About"`. Assert the `<title>` tag is `About | A cool site`.
2. Verify the test fails
3. Fix the SEO tag title generation in `src/template/seo_tag.rs`
4. Verify the test passes

## Acceptance Criteria

- [ ] Title tag for non-homepage pages includes `| site.description` suffix (or `| site.tagline` if set), matching jekyll-seo-tag's logic
- [ ] Title tag for the homepage uses the appropriate format (site.title alone, or site.title | site.description -- match jekyll-seo-tag exactly)
- [ ] When `site.title` is set and page has no title, use `site.title | site.description`
- [ ] The separator between title and description matches Jekyll's (` | ` by default, or custom `seo.title_separator` if configured)
- [ ] All 9 theme sites show improved title tags
- [ ] `cargo test` passes

## Test Scenarios

### Unit: Title tag with description suffix (write FIRST, must fail before fix)

- **Test `test_title_tag_with_description_suffix`**: Site with `title: "My Site"`, `description: "A cool site"`. Page with `title: "About"`. Assert `<title>` is `About | A cool site`.
- **Test `test_title_tag_homepage_format`**: Homepage (no page title). Assert `<title>` uses site title format per jekyll-seo-tag rules.
- **Test `test_title_tag_with_tagline`**: Site with `tagline: "My Tagline"` instead of `description`. Assert tagline is used in the suffix.
- **Test `test_title_tag_custom_separator`**: Site with SEO config `title_separator: " - "`. Assert the separator is ` - ` not ` | `.
- **Test `test_title_tag_no_description`**: Site with no `description` or `tagline`. Assert title tag has no suffix.

### Regression: Existing title behavior preserved

- **Test `test_title_tag_page_with_explicit_seo_title`**: Page with `seo.title` override in front matter. Assert that overrides the computed title.

### Integration: Output verification

- Build architect-theme and inspect `<title>` tags to verify they include the description suffix.
- Build DTC site and verify the title tag format matches Jekyll.

## Log

### [SWE] 2026-03-17
- Root cause: In `src/template/seo_tag.rs`, the title generation match arm `(None, Some(st))` (no page title, only site title) returned just `st.clone()` without appending site tagline/description. Jekyll-seo-tag appends `| site.tagline` or `| site.description` in this case.
- Also missing: support for custom `site.title_separator` config (defaulting to ` | `).
- Fixed the `(None, Some(st))` arm to append tagline/description when available.
- Added `site.title_separator` support: reads from runtime context, wraps with spaces, falls back to ` | `.
- Changed all title formatting to use the dynamic separator instead of the constant.
- Tests added: 6 new tests (test_title_tag_with_description_suffix, test_title_tag_homepage_format, test_title_tag_with_tagline, test_title_tag_custom_separator, test_title_tag_no_description, test_title_tag_page_with_different_titles)
- Build: 51 seo_tag tests pass (45 existing + 6 new), full suite passes, clippy clean, fmt clean
- Files modified: src/template/seo_tag.rs
