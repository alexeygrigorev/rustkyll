# Issue 490: academicpages sitemap entry ordering
## Problem
Entries in wrong order. 1 page, 46 diffs.
## Affected Sites
- academicpages
## Baseline
DTC 790/790. academicpages 27/45. Must not regress.

## Status: SUPERSEDED by #518

The ordering is now correct. The remaining 88 sitemap diffs are caused by the #474
date backfill overcorrection: non-post collection items show excerpts instead of dates
in archive-single listings. Tracked in issue #518.
