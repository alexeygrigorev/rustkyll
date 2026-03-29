# Issue 487: academicpages portfolio item metadata
## Problem
Wrong p class, missing itemprop attributes. 3 pages.
## Affected Sites
- academicpages
## Baseline
DTC 790/790. academicpages 27/45. Must not regress.

## Status: SUPERSEDED by #518

Root cause is #474 date backfill overcorrection. Non-post collection items no longer
expose `post.date`, causing `archive-single.html` to show excerpt instead of date.
Tracked in issue #518.
