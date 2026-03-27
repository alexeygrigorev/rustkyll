# Issue 389: DTC analytics-engineering intra-word emphasis matching

## Problem

`books/20231106-analytics-engineering-with-sql-and-dbt.html` was reported to have 11 DOM diffs
caused by intra-word emphasis handling. The text `sh*t` in the source markdown was expected to
trigger different behavior between kramdown and pulldown-cmark:

- kramdown: `sh` + `<em>t ton of money...</em>` (opens emphasis at `*t`)
- pulldown-cmark: `sh*t` stays as literal text (CommonMark intra-word rules)

## Current State (as of grooming, 2026-03-27)

DOM comparison shows this page now has only **1 diff** -- the common `href=''` vs
`href='https://github.com/DataTalksClub/datatalksclub.github.io'` attribute difference
shared across nearly all pages. The 11 emphasis-related diffs appear to have been resolved
by previous fixes (issues 206, 350, and related emphasis boundary work in
`fix_kramdown_emphasis_patterns`).

Both Jekyll and rustkyll produce identical output for the `sh*t` text: literal `sh*t` with
no emphasis tags.

The overall DTC DOM baseline from committed code is **788/790** (788 pages with at most
the 1 common href diff; only 2 pages -- `the-practitioners-guide-to-graph-data.html` and
`business-skills-for-data-scientists.html` -- have additional diffs).

## Scope

1. Verify that the analytics-engineering page matches Jekyll output for the emphasis-related
   content (the `sh*t` passage and surrounding emphasis patterns)
2. Write a regression test that ensures intra-word `*` in `sh*t ton of money` does NOT
   produce spurious `<em>` tags
3. Confirm the page has no emphasis-related diffs (only the common href diff is acceptable)
4. Must not regress DTC DOM baseline (788/790)
5. If the page is confirmed fixed, also close issue #383 as resolved (same underlying problem)

## Root Cause

kramdown uses different emphasis boundary rules than CommonMark. In kramdown,
`*` can open emphasis even when preceded by a word character (intra-word).
pulldown-cmark follows CommonMark spec which requires `*` to be preceded by
whitespace or punctuation to open emphasis. The existing `fix_kramdown_emphasis_patterns`
function in `src/frontmatter.rs` handles short intra-word patterns like `word*X*`.

For the `sh*t` case specifically, both kramdown and pulldown-cmark treat it as literal
text (no emphasis) because there is no closing `*` in the right position. The original
11 diffs were likely caused by other emphasis patterns in the same page that have since
been fixed.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new regression test(s)
- [ ] New unit test: markdown containing `sh*t ton of money` produces no `<em>` tags
      (literal `sh*t` in output)
- [ ] New unit test: markdown containing intra-word `*` followed by space does not
      open emphasis (e.g., `word*text more words` stays literal)
- [ ] DOM comparison of `books/20231106-analytics-engineering-with-sql-and-dbt.html`
      shows at most 1 diff (the common href attribute diff)
- [ ] No emphasis-related DOM diffs on the analytics-engineering page
- [ ] DTC DOM baseline must not drop below 788/790
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] If confirmed fixed, issue #383 is marked as resolved (rename to `.done.md`)
      with a note that the fix was delivered by earlier emphasis boundary work

## Test Scenarios

### Unit: Intra-word emphasis safety

- Parse `sh*t ton of money on setting up a RedShift cluster` -- verify no `<em>` tags
- Parse `don't wanna spend sh*t ton of money` -- verify `sh*t` is literal in output
- Parse `word*not emphasis because no closing star` -- verify literal output
- Parse `already*working*emphasis` -- verify `<em>working</em>` is produced (existing
  behavior must be preserved)

### Regression: Existing emphasis patterns still work

- Verify `straightforward*.* Even for someone` still produces correct emphasis
  (issue 350 regression test)
- Verify `word*X*` short intra-word patterns still produce `<em>X</em>`
- Verify mixed delimiter patterns (`_*text*_`) still work correctly

### Integration: Full page output

- Build DTC site and verify analytics-engineering page output matches Jekyll
- Run DOM comparison and confirm baseline holds at 788/790 or better

## Dependencies

- Related to #383 (analytics-engineering emphasis in words) -- likely same resolution
- No blocking dependencies; previous emphasis fixes (206, 350) are already committed

## Baseline

- DTC DOM: 788/790 (verified 2026-03-27 from committed code)

## Log

### [SWE] 2026-03-27
- Confirmed issue is already fixed by prior emphasis boundary work (issues 206, 350)
- Wrote 7 regression tests in src/frontmatter.rs (test_issue389_*):
  1. test_issue389_intra_word_asterisk_no_emphasis -- `sh*t` literal, no <em>
  2. test_issue389_intra_word_asterisk_in_sentence -- full sentence context
  3. test_issue389_intra_word_asterisk_no_closing_star -- lone * stays literal
  4. test_issue389_valid_intra_word_emphasis_preserved -- already*working*emphasis still works
  5. test_issue389_existing_emphasis_dot_preserved -- straightforward*.* regression check
  6. test_issue389_mixed_delimiter_emphasis_preserved -- _*text*_ still works
  7. test_issue389_intra_word_asterisk_unicode -- Unicode content with intra-word *
- All 7 tests PASS (already fixed, regression-only issue)
- Clippy clean, fmt clean
- Closed issue #383 as resolved (same underlying problem, git mv to done/)
- Files modified: src/frontmatter.rs, docs/tracker/done/383-*.done.md
