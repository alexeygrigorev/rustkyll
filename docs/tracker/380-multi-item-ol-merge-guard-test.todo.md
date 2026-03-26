# Issue 380: Add guard test for multi-item ol merge behavior

## Background

Descoped from issue 379 (AC5). The `merge_consecutive_same_type_lists` function
merges ALL consecutive same-type lists unconditionally, including multi-item ones.
This is pre-existing behavior from the original inline code. Issue 379's spec
called for a test verifying multi-item `<ol>` elements are NOT merged, but the
implementation intentionally kept the broader merge behavior since it causes no
DOM regression.

## Scope

1. Decide whether multi-item `<ol>` merging is correct behavior or a latent bug.
2. If correct: add a test documenting this intentional behavior.
3. If incorrect: add the guard test and fix `merge_consecutive_same_type_lists`
   to only merge single-item lists, then verify no DOM regression.

## Acceptance Criteria

- [ ] A test exists that exercises consecutive multi-item `<ol>` elements
- [ ] The behavior (merge or not) is explicitly documented in the test name/comment
- [ ] DTC DOM baseline is not regressed

## Dependencies

- Issue 379 (done)
