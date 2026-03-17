# Issue 172: Fix related posts ordering to match Jekyll

## Problem

On large-blog-3000, every single one of 3,001 files differs because the "related posts" links point to different posts than Jekyll produces. Jekyll's `site.related_posts` returns the 10 most recent posts (sorted by date descending), but rustkyll produces a different ordering.

This causes 119,860 total DOM diffs (the largest diff count of any site) -- all from `text_differs` and `attribute_differs` in the related post links.

## Root cause

The related posts list in rustkyll is either:
1. Not sorted by date descending, or
2. Using a different tiebreaking mechanism than Jekyll when dates are equal (many synthetic posts share the same date)

Jekyll sorts related posts by date descending, then by slug/path alphabetically for posts with the same date.

## Affected sites

| Site | Files affected | Diffs |
|------|---------------|-------|
| large-blog-3000 | 3001/3001 | 119,860 |
| mojombo-blog | 1/17 | ~3 (one page has wrong related post date/href) |

## Acceptance criteria

- [ ] `site.related_posts` returns posts sorted by date descending, with stable tiebreaking matching Jekyll (alphabetical by slug/path for same-date posts)
- [ ] large-blog-3000 related post links match Jekyll output (spot-check 5 pages)
- [ ] The current post is excluded from its own related posts list
- [ ] Existing tests continue to pass

## Dependencies

Depends on issue #42 (site.related_posts) which is already done.
