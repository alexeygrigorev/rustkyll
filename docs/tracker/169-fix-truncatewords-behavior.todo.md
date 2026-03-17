# Issue 169: Fix truncatewords to match Jekyll behavior exactly

## Problem

People JSON-LD descriptions are truncated at slightly different points between Jekyll and rustkyll. 9 files affected. The `truncatewords` filter cuts at different word boundaries.

Example: Jekyll `"$500,000 grand prize"` vs rustkyll `"$500,000grand prize"` (space stripped) and truncation ends at different word.

## Acceptance criteria

- truncatewords produces identical output to Jekyll for all test cases
- People JSON-LD descriptions match Jekyll exactly
- TDD: failing test, fix, test passes
