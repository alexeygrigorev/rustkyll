# Issue 492: academicpages collection archive wrappers
## Problem
Missing h2 and div wrappers. 1 page.
## Affected Sites
- academicpages
## Baseline
DTC 790/790. academicpages 27/45. Must not regress.

## Status: SUPERSEDED by #518

Collection-archive now renders all 13 items with correct h2 headers and div wrappers.
The remaining 10 diffs are from the #474 date backfill overcorrection (showing excerpt
instead of date). Tracked in issue #518.
