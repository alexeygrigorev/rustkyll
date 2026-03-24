# Issue 342: Liquid sort filter stable tiebreak (match Jekyll)

## Problem

The Liquid `sort` filter in rustkyll does not match Jekyll's stable sort behavior for equal values. When a Liquid template does `site.posts | sort: "date" | reverse`, posts with the same date appear in a different order than Jekyll produces.

This was discovered in issue 337 sub-issue A: the `free-machine-learning-courses.html` page uses a custom `related-posts.html` include that sorts via Liquid `sort: "date" | reverse`. The 6 DOM diffs on this page are caused by the Liquid sort filter's tiebreak behavior, not by `site.related_posts`.

Jekyll's Liquid sort filter uses a stable sort, so equal-key items retain their original order from `site.posts` (which is date descending, path ascending for same-date). Rustkyll's Liquid sort filter does not preserve this stability.

## Affected pages

- `blog/free-machine-learning-courses.html` (6 DOM diffs from this issue)

## Origin

Descoped from issue 337 sub-issue A. Original acceptance criteria required 0 DOM diffs on this page, but the root cause was misdiagnosed as FAQ accordion whitespace / `site.related_posts` tiebreak.

## Dependencies

- None
