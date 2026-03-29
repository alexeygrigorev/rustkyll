# Issue 486: academicpages post list page rendering
## Problem
Same metadata issues in list views (tags, year-archive, page-archive). 4 pages.
## Affected Sites
- academicpages
## Baseline
DTC 790/790. academicpages 27/45. Must not regress.

## Status: RESOLVED

As of 2026-03-29, this is already fixed in the current codebase. Blog post tag
ordering now matches Jekyll (tag sort works correctly). Year-archive and tags pages
match. Page-archive has only 1 trivial diff (missing css.map page).

Academicpages now at 38/45 (up from 27/45 at time of filing).
