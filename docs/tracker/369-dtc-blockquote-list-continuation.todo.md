# Issue 369: DTC blockquote + list continuation

## Parent

Follow-up from #363 (RC-H).

## Problem

Continuation text with `<br>` after blockquote produces extra `<blockquote>` elements. Also, `<ol>` inside `<li>` with `<br>` continuation text not rendering the intermediate text and nested list.

## Affected Pages

- `books/20231106-analytics-engineering-with-sql-and-dbt.html` (partial of 8 diffs) -- extra `<blockquote>` elements
- `books/20210823-business-skills-for-data-scientists.html` (9 diffs) -- missing text/`<br>`/`<ol>` inside `<li>`

## Acceptance Criteria

- [ ] Blockquote followed by list renders without extra `<blockquote>` elements
- [ ] `<ol>` inside `<li>` with `<br>` continuation preserves intermediate text and nested list
- [ ] DTC DOM match count does not regress
- [ ] No site-specific hardcoding

## Priority

MEDIUM
