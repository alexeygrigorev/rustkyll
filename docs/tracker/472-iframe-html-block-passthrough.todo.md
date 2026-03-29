# Issue 472: iframe HTML block passthrough (muan-blog)
## Problem
Raw `<iframe>` blocks wrapped in `<p>` instead of passed through. 6 files.
Note: Previous #449 fix was reverted because adding img to BLOCK_TAGS regressed DTC docs. Need a more targeted approach.
## Affected Sites
- muan-blog (6 files)
## Baseline
DTC 790/790. DTC docs 57/57. Must not regress either.
