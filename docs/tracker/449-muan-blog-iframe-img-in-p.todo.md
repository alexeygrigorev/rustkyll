# Issue 449: muan-blog iframe/img wrapped in <p> tags (9 diffs, 7 files)

## Problem
Block-level HTML elements (iframe, img) are wrapped in <p> tags by
the markdown parser instead of being passed through as-is.

## Scope
Fix markdown HTML block passthrough for iframe and img elements.

## Baseline
DTC 790/790. muan-blog 2194/2218.
