# Issue 371: DTC residual numbered list rendering in books pages

## Problem

After issue #363 fixed the numbered list preprocessing for sequences of 2+
items starting at N > 1, two books pages still have remaining diffs caused by
numbered list patterns that don't match the current heuristic:

- `books/20220926-graph-algorithms-for-data-science.html` (2 diffs)
- `books/20240715-ai-data-privacy-and-protection.html` (12 diffs)

These pages have RC-A patterns (numbered text after `<br />`) that use single
numbered items or lists starting at 1, which the current
`insert_paragraph_break_before_numbered_list()` deliberately skips to avoid
making existing tight lists loose.

## Scope

1. Investigate the specific numbered list patterns on both pages
2. Fix without regressing existing tight list rendering
3. DTC DOM baseline must not drop below 776/790

## Discovered In

Issue #363 (DTC books comment text and mixed content rendering)

## Dependencies

- Follow-up from #363
