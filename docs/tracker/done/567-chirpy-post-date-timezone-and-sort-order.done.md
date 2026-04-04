# Issue 549: Chirpy post date timezone handling and sort order

## Problem

Two related issues affecting chirpy's post ordering and date display:

### 1. Post date timezone conversion

Chirpy's `customize-the-favicon` post has date `2019-08-11 00:34:00 +0800`. Jekyll displays this as "Aug 10, 2019" (converting to UTC: Aug 10 16:34 UTC). Rustkyll displays "Aug 11, 2019" (using the raw date without timezone conversion).

When a post date includes a timezone offset, Jekyll converts to UTC for display purposes (or uses the site's configured timezone). Rustkyll appears to ignore the timezone offset and use the naive date.

This causes:
- `customize-the-favicon/index.html`: date shows "Aug 11" instead of "Aug 10"
- Index page sort order: posts sorted by naive date instead of UTC-normalized date

### 2. Index page post ordering

Chirpy's index.html shows posts in wrong order. Expected (newest first):
1. Customize the Favicon (Aug 11 +0800 = Aug 10 UTC)
2. Getting Started (Aug 9)
3. Writing a New Post (Aug 8 14:10 +0800)
4. Text and Typography (Aug 8 11:33 +0800)

Rustkyll shows Getting Started first instead of Customize the Favicon, suggesting the sort uses naive dates or has a different tiebreaker.

This also affects hydeout where two posts on 2010-01-07 (post-standard and post-modified) appear in different order on page3/page4.

## Root Cause

Post dates with timezone offsets are not properly normalized for sorting and display. The sort comparison should use the UTC-equivalent instant, not the local date components.

For same-date tiebreaking (hydeout), Jekyll uses a stable sort that preserves filesystem/alphabetical order when dates are identical.

## Scope

- Normalize post dates to UTC for sorting purposes
- Ensure date display in templates uses the correct timezone-adjusted value
- Verify same-date posts have stable tiebreaker order matching Jekyll

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new ones
- [ ] Post with date `2019-08-11 00:34:00 +0800` sorts as Aug 10 UTC (before a post dated Aug 9 with no offset)
- [ ] Date display for `2019-08-11 00:34:00 +0800` shows "Aug 10, 2019" when using `%b %-d, %Y` format (UTC)
- [ ] Chirpy index.html shows "Customize the Favicon" as the first (newest) post
- [ ] Chirpy `posts/customize-the-favicon/index.html` shows "Aug 10, 2019" not "Aug 11, 2019"
- [ ] Hydeout page3/page4 post order matches Jekyll (post-standard before post-modified on page3)
- [ ] DTC DOM match count must not drop below 790/790

## Test Scenarios

### Unit: Timezone-aware date sorting
- Two posts: one with `2019-08-11 00:34:00 +0800`, one with `2019-08-09 12:00:00 +0800`. Verify the Aug 11 +0800 post sorts as newer (Aug 10 16:34 UTC > Aug 9 04:00 UTC)
- Post with positive timezone offset that crosses midnight boundary: verify sort uses UTC date
- Posts with identical dates: verify stable sort order (alphabetical by filename or path)

### Unit: Date display with timezone
- Date `2019-08-11 00:34:00 +0800` formatted as `%b %-d, %Y` should output "Aug 10, 2019"
- Date `2019-08-08 14:10:00 +0800` formatted as `%b %-d, %Y` should output "Aug 8, 2019"
- Date with no timezone offset: display unchanged

### Integration: Chirpy site
- Build chirpy, verify index.html post order: Customize the Favicon, Getting Started, Write a New Post, Text and Typography
- Verify customize-the-favicon shows "Aug 10, 2019"

### Integration: Hydeout site
- Build hydeout, verify page3 shows "Post: Standard" before "Post: Modified Date"

## Dependencies

None.

## DTC Baseline

790/790 matched (must not regress)

## Notes

Be careful with timezone handling -- DTC is at 790/790 and many DTC posts have timezone offsets. Any change to date handling must be verified against the full DTC site to ensure no regressions.

Check how Jekyll handles the `timezone` config key (some sites set `timezone: America/New_York` etc.). The behavior may be: display dates in site timezone, sort by UTC instant.

## Log

### [SWE] 2026-04-02

**Fix 1: Date display -- convert timezone-offset dates to UTC/site-tz**
- Wrote test `test_expand_date_converts_positive_tz_offset_to_utc` (src/template/context.rs)
- Ran test: FAILS -- got "2019-08-11 00:34:00 +0800", expected "2019-08-10 16:34:00 +0000"
- Implemented fix in `expand_date_only_string_with_tz()` (src/template/context.rs:270): parse `YYYY-MM-DD HH:MM:SS +HHMM` dates, convert to UTC via `format_utc_to_site_tz()`. Also convert ISO 8601/RFC 3339 dates with offsets.
- Ran test: PASSES
- Also wrote: `test_expand_date_converts_negative_tz_offset_to_utc`, `test_expand_date_converts_tz_offset_to_site_tz`, `test_expand_date_converts_tz_offset_cross_day_boundary`, `test_expand_date_utc_offset_unchanged`, `test_expand_date_unicode_and_non_ascii_passthrough`

**Fix 2: UTC-normalized date sorting**
- Wrote test `test_date_sort_key_normalizes_timezone_offsets` (src/collection.rs)
- Test compiled and passed (function implemented together since it's a new function)
- Implemented `date_sort_key()` in src/collection.rs: converts dates with timezone offsets to UTC for comparison
- Updated sort calls in collection.rs, generator.rs, pagination.rs, feed.rs, archives.rs, plugin_generators.rs to use `date_sort_key()` for UTC-normalized comparison

**Fix 3: Pagination tiebreaker direction**
- Identified bug: pagination.rs used ascending slug tiebreaker for descending date sort, should be descending slug
- Fixed pagination.rs:244 and pagination.rs:694: changed `a.slug.cmp(&b.slug)` to `b.slug.cmp(&a.slug)` for reverse sort
- Also fixed feed.rs:57 (same bug)
- Verified hydeout page3/page4 now matches Jekyll (post-standard before post-modified)

**Updated existing tests:**
- `test_date_normalization_existing_full_datetime_unchanged` -> `test_date_normalization_existing_full_datetime_converts_to_utc`
- `test_date_normalization_iso8601_with_colon_tz` -- updated expected value to UTC
- `test_date_normalization_iso8601_negative_offset` -- updated expected value to UTC
- `test_slim_already_expanded_date_unchanged` -> `test_slim_date_with_tz_offset_converted_to_utc`
- `test_issue551_non_post_backfilled_date_value_matches_build_time` -- updated expected value to UTC

**Verification:**
- Chirpy customize-the-favicon: now shows "Aug 10, 2019" (was "Aug 11, 2019") -- matches Jekyll
- Hydeout page3: post-standard before post-modified -- matches Jekyll
- DTC DOM: 790/790 with 0 total diffs (no regression)
- DTC build time: 0.818s (under 1.0s)
- Chirpy DOM: 12/17 with 77 total diffs (improved from 80)

**Summary:**
- Files modified: src/template/context.rs, src/collection.rs, src/generator.rs, src/pagination.rs, src/feed.rs, src/archives.rs, src/plugin_generators.rs
- Tests added: 10 new tests (6 in context.rs, 4 in collection.rs)
- Tests updated: 5 existing tests updated to match new UTC conversion behavior
- Build results: 3880 tests pass, 0 fail, clippy clean, fmt clean
- Known limitations: chirpy index.html still shows only 2 unique posts (Getting Started + Text and Typography) instead of all 4 -- this appears to be a pre-existing pagination issue unrelated to this fix

### [PM] 2026-04-02 15:30
- Reviewed diff: 8 files changed, 198 insertions, 60 deletions
- Output verification: Built Chirpy site, confirmed customize-the-favicon shows "Aug 10, 2019" (correct UTC conversion from +0800). Built Hydeout, confirmed page3 ends with post-standard and page4 starts with post-modified (correct tiebreaker order). Built DTC site and ran DOM comparison.
- Results verified: DTC 790/790 (no regression). 4308 tests pass, 0 fail. Clippy clean.
- Acceptance criteria:
  - [x] cargo build compiles without errors
  - [x] cargo test passes with all existing tests plus new ones (4308 total)
  - [x] Post with +0800 sorts correctly as UTC
  - [x] Date display shows "Aug 10, 2019" (UTC converted)
  - [x] Chirpy customize-the-favicon/index.html shows "Aug 10, 2019"
  - [x] Hydeout page3/page4 order matches Jekyll
  - [x] DTC DOM 790/790 (no regression)
  - [~] Chirpy index.html shows Customize the Favicon as first post -- NOT MET (pre-existing: baseline also shows only 2 posts). Descoped to follow-up issue.
- Follow-up issues created: docs/tracker/574-chirpy-index-pagination-missing-posts.todo.md
- VERDICT: ACCEPT (with descoped criterion tracked as follow-up)
