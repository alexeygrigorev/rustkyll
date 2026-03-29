# Issue 488: academicpages head meta tag ordering
## Problem
Meta tags reordered in portfolio detail pages. 2 pages.
## Affected Sites
- academicpages
## Baseline
DTC 790/790. academicpages 27/45. Must not regress.

## Status: SUPERSEDED by #518

Root cause is #474 date backfill overcorrection. Missing `article:published_time`
meta tag causes all subsequent head elements to shift, creating 23 cascading diffs
per page. Tracked in issue #518.
