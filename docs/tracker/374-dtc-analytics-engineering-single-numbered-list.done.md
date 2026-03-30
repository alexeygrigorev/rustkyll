# Issue 374: DTC analytics-engineering single numbered item list rendering

## Problem

On `books/20231106-analytics-engineering-with-sql-and-dbt.html` (8 diffs), numbered list patterns with single items or items starting at numbers other than 1 are not recognized as ordered lists. The current `insert_paragraph_break_before_numbered_list` heuristic requires 2+ consecutive numbered items, so single-item cases are skipped.

## Prior Attempt

Issue #370 attempted changing the heuristic to insert paragraph breaks for single numbered items, but this caused widespread regressions (DOM dropped from 780 to 776) on other book pages where tight lists became loose. The single-item case requires a more targeted approach.

## Scope

1. Fix single-item numbered list recognition for book archive threads
2. Must not regress existing tight list rendering on other pages
3. DTC DOM baseline must not drop below 780/790

## Affected Pages

- `books/20231106-analytics-engineering-with-sql-and-dbt.html` (8 diffs)

## Dependencies

- Follow-up from #370
