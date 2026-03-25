# Issue 361: Fix where filter datetime-vs-string comparison (Yat theme)

## Problem

The Yat theme's archives page uses `site.posts | where: 'date', '2018'` to group posts by year. In Jekyll, this works because the `where` filter coerces dates to strings for comparison. In rustkyll, datetime objects don't match string values, so the archives page renders year headings but no posts under them.

## Discovered In

Issue #243 (Yat theme benchmark)

## Acceptance Criteria

- [ ] `where` filter can compare datetime fields against string values (via string coercion)
- [ ] The Yat theme's archives page renders posts under each year heading
- [ ] No DTC DOM regression (baseline: 771/790)
