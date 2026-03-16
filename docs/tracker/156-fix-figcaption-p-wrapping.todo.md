# Issue 156: Fix figcaption <p> wrapping (196 diffs)

## Problem

Jekyll wraps text inside `<figcaption>` in `<p>` tags. Rustkyll doesn't in some cases. 196 DOM diffs.

Note: Issue #152 partially fixed this but some cases remain.

## Acceptance criteria

- All figcaption content matches Jekyll's <p> wrapping
- 196 DOM diffs eliminated
- TDD: failing test, fix, test passes
