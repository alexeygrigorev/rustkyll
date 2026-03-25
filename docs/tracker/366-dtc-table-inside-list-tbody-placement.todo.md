# Issue 366: DTC table inside list items / tbody placement

## Parent

Follow-up from #363 (RC-E).

## Problem

A comment with a markdown table inside a list context has `<tbody>` rendered outside `<table>`, and raw markdown list syntax (`- dataset:...`) leaking as text instead of being rendered.

## Affected Pages

- `books/20220425-natural-language-processing-with-transformers.html` (7 diffs)

## Acceptance Criteria

- [ ] Tables inside list item context render with correct `<table>`/`<tbody>` nesting
- [ ] Raw markdown list syntax inside table context is rendered, not leaked as text
- [ ] DTC DOM match count does not regress
- [ ] No site-specific hardcoding

## Priority

LOW
