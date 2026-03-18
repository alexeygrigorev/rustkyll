# Issue 222: Fix muan-blog notes tags concatenation

## Problem

The notes.html page has 1795 diffs. Tags in the notes filter form are concatenated (e.g., "BookMental health" instead of separate "Book" and "Mental health"). The Liquid `map` filter or `uniq` filter isn't properly splitting multi-tag arrays. Notes with multiple tags produce concatenated strings instead of flat individual tags.

## Scope

1. Identify how tags are collected and flattened in Liquid templates (likely `map` + `uniq` chain)
2. Fix the array flattening so multi-tag posts produce individual tag entries
3. Verify the notes.html filter form renders all tags separately

## Acceptance Criteria

- [ ] Tags from multi-tag notes are individual entries, not concatenated
- [ ] notes.html tag filter form matches Jekyll output
- [ ] 1795 diffs on notes.html are substantially reduced
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all new and existing tests
- [ ] Tests include posts with multiple tags to verify flattening

## Log

- 2026-03-18: Created from muan-blog comparison analysis.
