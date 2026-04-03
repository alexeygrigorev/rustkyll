# Issue 361: Fix where filter datetime-vs-string comparison (Yat theme)

## Problem

The Yat theme's archives page uses `site.posts | where: 'date', '2018'` to group posts by year. In Jekyll, the `where` filter coerces all values -- including dates -- to strings before comparison. In rustkyll, the `where` filter already coerces booleans and integers via `to_kstr()`, but datetime values stored as expanded date strings (e.g. `2018-01-15 00:00:00 +0000`) do not match a bare year string like `"2018"`.

## Root Cause

The `where` filter in `src/template/filters/where_filter.rs` uses exact string equality (`val.to_kstr() == target_value`). When comparing a date field like `2018-01-15 00:00:00 +0000` against the target `"2018"`, the comparison fails because the full datetime string does not equal `"2018"`.

Jekyll's `where` filter uses Ruby's `==` operator, which for dates does `Date#to_s == "2018"` -- but actually Jekyll converts dates to strings via `Utils.stringify_value` before comparison, which produces a full datetime string. The real mechanism in the Yat theme is that the archives page uses `group_by_exp` to extract years first, then uses `where` to filter by date *substring*. However, looking at the actual Yat archives template, it may use `group_by_exp: "post", "post.date | date: '%Y'"` followed by iterating groups.

The fix should ensure that when the `where` filter compares a datetime-containing string against a shorter target string, it applies substring/prefix matching to handle the common Jekyll pattern of filtering dates by year. Alternatively, the filter should attempt to apply the same coercion Jekyll uses -- converting the property value to a string representation and doing exact comparison. The key question is exactly which coercion path Jekyll uses.

The safest approach is: if exact string match fails AND the property value looks like a datetime string (matches `YYYY-MM-DD HH:MM:SS` pattern), try comparing just the year portion (`YYYY`) against the target when the target is a 4-digit year. This matches the observed Jekyll behavior for the Yat archives page.

## Discovered In

Issue #243 (Yat theme benchmark)

## Dependencies

- None (the `where` filter and Yat theme site already exist)

## Key Files

- `src/template/filters/where_filter.rs` -- the `where` filter implementation
- `websites/yat/_includes/views/archives.html` -- the Yat archives template (reference)
- `datatalksclub.github.io/` -- reference site (must not regress)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] The `where` filter correctly matches datetime string values against year strings (e.g. `"2018-01-15 00:00:00 +0000"` matches target `"2018"`)
- [ ] The Yat archives page (`archives.html`) renders posts under each year heading (not empty segments)
- [ ] Existing `where` filter behavior is preserved: exact string match, boolean coercion, integer coercion all still work
- [ ] The fix is generic (not Yat-specific hardcoding) -- works for any site using `where` with date fields and year targets
- [ ] DTC DOM baseline stays at 788/790 or above
- [ ] No regression on other test sites (run DOM comparison on at least chirpy, minimal-mistakes, and hydeout)
- [ ] `cargo test` passes with all new and existing tests
- [ ] `cargo fmt --check` and `cargo clippy` pass cleanly

## Test Scenarios

### Unit: where filter datetime-vs-string coercion
- Create array of objects with date fields containing expanded datetime strings (`"2018-06-15 00:00:00 +0000"`, `"2019-03-20 00:00:00 +0000"`)
- Filter with `where: 'date', '2018'` and verify only the 2018 post is returned
- Filter with `where: 'date', '2019'` and verify only the 2019 post is returned
- Filter with `where: 'date', '2020'` and verify empty result (no match)
- Verify exact string match still takes priority (if date field literally contains `"2018"`, it matches)

### Unit: where filter existing behavior preserved
- Verify boolean-to-string coercion still works (`pin: true` matches `'true'`)
- Verify integer-to-string coercion still works (`priority: 1` matches `'1'`)
- Verify plain string match still works
- Verify nil/absent fields do not match

### Unit: edge cases
- Date field with timezone offset (`"2018-12-31 23:00:00 -0500"`) should match year `"2018"`
- Date field as bare date string (`"2018-06-15"`) matching against `"2018"` -- should also work if the fix is prefix-based
- Non-date string that happens to start with `"2018"` (e.g. `"2018abc"`) -- define expected behavior
- Test with non-ASCII content in other fields to verify no encoding regression

### Integration: Yat archives page
- Build the Yat site with rustkyll
- Inspect the archives page HTML output
- Verify each year heading has post items listed beneath it (not empty)
- Compare against Jekyll cached output for the archives page

### Regression: DTC DOM
- Build DTC site and run DOM comparison
- Verify match count is at least 788/790

## Log

### [SWE] 2026-03-30
- TDD step 1: Wrote 6 new tests in where_filter.rs:
  - test_where_filter_datetime_matches_year (2018 and 2019 targets)
  - test_where_filter_datetime_with_negative_timezone (-0500 offset)
  - test_where_filter_bare_date_matches_year (YYYY-MM-DD format)
  - test_where_filter_exact_match_takes_priority
  - test_where_filter_non_date_string_no_spurious_match
  - test_where_filter_datetime_unicode_content_no_regression
- TDD step 2: Ran tests, 4 FAIL as expected (datetime tests), 2 pass (exact match, non-date guard)
- TDD step 3: Implemented fix in where_filter.rs:
  - Added `is_year_target()` helper: checks if target is exactly 4 digits
  - Added `is_datetime_string()` helper: checks YYYY-MM-DD pattern (at least 10 chars)
  - Added fallback in filter: when exact match fails, if target is a 4-digit year AND value is a datetime string AND value starts with the target year, match succeeds
- TDD step 4: All 20 where_filter tests pass, full suite 3460+ pass, 0 fail
- Clippy clean, fmt clean
- Files modified: src/template/filters/where_filter.rs
