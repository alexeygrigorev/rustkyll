# Issue 146: Fix ordered list start attribute

## Problem

Rustkyll adds `start='N'` attributes to `<ol>` elements where Jekyll does not. 33 instances across ~5 files.

This happens when markdown has ordered list items that don't start at 1, or when the list is split across HTML blocks. Kramdown may not emit `start` attributes in these cases.

Discovered in issue #119 DOM diff audit.

## Acceptance criteria

- `<ol>` elements match Jekyll's `start` attribute behavior
- No regressions
