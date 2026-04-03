# Issue 248: Mediumish category/tag Liquid filters

## Problem

The Mediumish theme uses Liquid filters that rustkyll currently does not recognize:
- `url_escape`
- `camelcase`

These appear in the category and tag navigation templates (`_layouts/default.html` lines 157, 161) and cause warnings plus broken category/tag output.

## Root Cause

The theme relies on Jekyll-specific Liquid filters for category anchors and display names. Jekyll itself does not define `camelcase` or `url_escape` as built-in filters, but themes reference them anyway; they should pass through without error.

## Current State

Both filters are already implemented as passthrough filters in `src/template/filters/url_escape.rs` and `src/template/filters/camelcase.rs`, and both are registered in `src/template/filters/mod.rs`. This issue may already be resolved.

## Scope

1. Verify that `url_escape` and `camelcase` filters are registered and produce the expected passthrough behavior.
2. Verify the Mediumish category/tag sidebar renders correctly (no raw Liquid markup, no unknown-filter warnings).
3. Confirm the Mediumish DOM comparison does not regress from the parent issue (#239) baseline.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes, including the existing `url_escape` and `camelcase` filter tests
- [ ] Building `websites/mediumish/` with rustkyll produces no unknown-filter warnings for `url_escape` or `camelcase`
- [ ] The generated Mediumish `default.html` output contains rendered category sidebar links (not raw Liquid `{{ ... | url_escape }}` or `{{ ... | camelcase }}` markup)
- [ ] Mediumish DOM comparison does not regress from the #239 baseline
- [ ] DTC DOM count remains at 788/790 or above

## Test Scenarios

### Unit: url_escape filter
- Verify `url_escape` filter passes through input unchanged (already covered in `url_escape.rs` tests)
- Verify filter handles empty string, strings with spaces, and plain strings

### Unit: camelcase filter
- Verify `camelcase` filter passes through input unchanged (already covered in `camelcase.rs` tests)
- Verify filter handles snake_case, capitalized, empty, and space-containing inputs

### Integration: Mediumish category sidebar
- Build `websites/mediumish/` with rustkyll and inspect the homepage output
- Verify the category navigation sidebar contains `<a>` tags with `href` attributes referencing category anchors
- Verify no raw Liquid `{{ ... | url_escape }}` or `{{ ... | camelcase }}` markup appears in the HTML output

## Dependencies

- Issue #239 (must be `.done.md` or `.in-progress.md`)

## Log

### [SWE] 2026-03-30
- Verified existing passthrough filter implementations in `src/template/filters/url_escape.rs` and `src/template/filters/camelcase.rs`
- Verified both filters are registered in `src/template/filters/mod.rs`
- Existing unit tests: 3 for url_escape, 5 for camelcase, 2 engine-level camelcase tests -- all pass
- Wrote 4 integration tests in `tests/test_issue_248_mediumish_filters.rs`:
  - `test_mediumish_no_unknown_filter_warnings`: builds mediumish, checks stderr for url_escape/camelcase warnings
  - `test_mediumish_category_sidebar_rendered`: verifies no raw Liquid markup in index.html
  - `test_mediumish_category_sidebar_has_links`: verifies category anchor links exist in fortags section
  - `test_mediumish_no_raw_liquid_in_any_page`: walks all HTML files checking for raw filter markup
- All 4 integration tests PASS; build completes in ~0.5s
- Existing unit tests (10 total for url_escape + camelcase): all PASS
- Full suite: 3502 passed, 1 failed (pre-existing `test_link_tag_collection_unicode_with_trailing_slash` from other in-progress issues), 2 ignored
- Clippy clean, fmt clean
- Files created: `tests/test_issue_248_mediumish_filters.rs`
