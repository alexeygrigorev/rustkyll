# Issue 37: Implement Missing Jekyll Filters

## Problem

Cross-site testing (Issue 32) revealed that rustkyll does not implement all Jekyll built-in Liquid filters. Sites using these filters fail to build.

Missing filters discovered:
- `normalize_whitespace` -- collapses multiple whitespace characters into a single space (used by `little-book-of-metals-ru`)

Additionally, unknown filters cause a hard build failure. Ideally, unknown filters should produce a warning and pass through the value unchanged, rather than crashing the entire build.

## Found In

- `alexeygrigorev/little-book-of-metals-ru` -- uses `normalize_whitespace`
- `alexeygrigorev/mlbookcamp-page` -- uses `erl_encode` (likely a typo for `url_encode`, but the hard failure is the problem)

## Requirements

- Implement the `normalize_whitespace` filter
- Consider graceful handling of unknown filters (warning instead of error)

## Dependencies

- None
