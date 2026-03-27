# Issue 383: DTC analytics-engineering intra-word emphasis diffs

## Problem

`books/20231106-analytics-engineering-with-sql-and-dbt.html` has remaining DOM
diffs related to emphasis handling inside words. Kramdown (Jekyll) treats `*`
characters inside words (e.g., `sh*t`) as opening `<em>` tags, while
pulldown-cmark does not apply intra-word emphasis by default.

This was descoped from issue #381 (blockquote splitting), which fixed the 3
extra blockquote diffs but could not address the emphasis diffs.

## Scope

1. Fix intra-word emphasis parsing so that `sh*t` and similar patterns produce
   `<em>` tags matching kramdown's behavior
2. The fix must be generic (not hardcoded to specific words)
3. Must not regress DTC DOM baseline (782/790 as of issue #381)

## Dependencies

- Issue #381 (blockquote splitting) must be done first

## Baseline

- DTC DOM: 782/790
