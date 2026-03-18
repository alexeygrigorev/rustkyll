# Issue 229: Fix theme sites remaining diffs

## Problem

10 theme sites still have diffs after issue 213 fixes: dinky, hacker, leap-day, merlot, midnight, primer, time-machine all at 0/2. The remaining issues are primarily syntax highlighting token class differences (syntect vs Rouge) and possibly JSON-LD field ordering.

## Scope

1. Build each affected theme site and compare against Jekyll reference
2. Categorize diffs: syntax highlighting classes, JSON-LD ordering, other
3. For syntax highlighting: map syntect token classes to Rouge equivalents
4. For JSON-LD: fix field ordering to match Jekyll output
5. Fix other systematic patterns

## Acceptance Criteria

- [ ] Syntax highlighting token classes match Rouge output for code blocks
- [ ] JSON-LD field ordering matches Jekyll output
- [ ] Theme sites achieve higher match rates (ideally 2/2 each)
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all new and existing tests

## Dependencies

- Issue 213 (theme site fixes) -- already done

## Log

- 2026-03-18: Created from cross-site comparison analysis.
