# Issue 370: DTC thread comment ordering / sorting

## Parent

Follow-up from #363 (RC-I).

## Problem

The `<h3>` and `<p>` elements for thread headers are rendering outside the comment `<div>` instead of inside the `<ul><li>` structure.

## Affected Pages

- `books/20210222-ml-algotrading-2ed.html`
- `books/20230807-driving-data-quality-with-data-contracts.html`
- `books/20241017-build-large-language-model-from-scratch.html` (8 diffs)

## Acceptance Criteria

- [ ] Thread header elements (`<h3>`, `<p>`) render inside the correct comment `<div>` / `<ul><li>` structure
- [ ] Comment ordering matches Jekyll output
- [ ] DTC DOM match count does not regress
- [ ] No site-specific hardcoding

## Priority

MEDIUM
