# Issue 147: Fix extra target='_blank' on links

## Problem

Rustkyll adds `target='_blank'` to some anchor elements where Jekyll does not. 3 instances across 2 files.

Discovered in issue #119 DOM diff audit.

## Acceptance criteria

- Links only have `target='_blank'` when Jekyll also adds it
- No regressions
