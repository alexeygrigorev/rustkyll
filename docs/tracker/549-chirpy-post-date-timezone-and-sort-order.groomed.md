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
