# Issue 265: GFM table before non-pipe text should not render as table (kramdown compat)

## Problem

Descoped from issue 248. In kramdown, a pipe table (even with a `|---|---|` separator row) is NOT rendered as a table if it is not followed by a block boundary. For example:

```markdown
| A | B |
|---|---|
| 1 | 2 |
not a pipe
```

Kramdown renders this as a paragraph (plain text), not a table. But pulldown-cmark with the GFM tables extension renders it as a `<table>` followed by a `<p>`.

This causes false-positive table rendering on sites converted from kramdown.

## Root Cause

The `|---|---|` separator row triggers pulldown-cmark's built-in GFM table parsing, which happens AFTER the kramdown preprocessor runs. The preprocessor's `is_standard_pipe_table_context()` function correctly detects this as a GFM table and skips it, but pulldown-cmark then parses it as a table regardless of what follows.

The current logic at line ~1801 in `kramdown.rs`:
```rust
if is_kramdown_table_line(trimmed) && !is_standard_pipe_table_context(&lines, i) {
    // ... block boundary checks only apply to kramdown tables
```

When `is_standard_pipe_table_context` returns `true`, the line is passed through unchanged -- pulldown-cmark then renders it as a GFM table with no block boundary check. The fix must detect GFM tables NOT followed by a block boundary and escape/neutralize the separator row so pulldown-cmark does not parse them as tables.

## Approach

When the preprocessor encounters a GFM table context (detected by `is_standard_pipe_table_context`), it must additionally check if the table block is followed by a block boundary. If it is NOT, the separator row (`|---|---|`) must be escaped or transformed so that pulldown-cmark does not recognize it as a GFM table. The content should then be rendered as plain paragraph text, matching kramdown behavior.

Possible escape strategies:
- Replace `|` with `\|` in the separator row
- Remove the separator row entirely and let remaining lines become paragraph text
- Insert a zero-width space or other invisible character to break the pattern

The chosen strategy must preserve the visible text content in the final output.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all new and existing tests
- [ ] Input `"| A | B |\n|---|---|\n| 1 | 2 |\nnot a pipe\n"` does NOT produce `<table>` in output -- must render as paragraph text
- [ ] Input `"| A | B |\n|---|---|\n| 1 | 2 |\n\ntext\n"` (blank line after table) STILL produces `<table>` -- block boundary present
- [ ] Input `"| A | B |\n|---|---|\n| 1 | 2 |\n"` (EOF after table) STILL produces `<table>` -- EOF is a block boundary
- [ ] Input `"| A | B |\n|---|---|\n| 1 | 2 |\n# Heading\n"` (heading after table) STILL produces `<table>` -- heading is a block boundary
- [ ] Input `"| A | B |\n|---|---|\n| 1 | 2 |\n---\n"` (HR after table) STILL produces `<table>` -- HR is a block boundary
- [ ] Input `"text before\n| A | B |\n|---|---|\n| 1 | 2 |\n"` (text before, no blank line) does NOT produce `<table>` -- no block boundary before
- [ ] Input with multiple GFM tables in one document: one with block boundary (renders as table) and one without (renders as paragraph) -- both handled correctly in the same document
- [ ] Unicode content in GFM table cells is preserved regardless of whether table renders or not (e.g., `"| Kolonne | Vaerdi |\n|---|---|\n| Tekst | Nummer |\nfortsaetter\n"`)
- [ ] No regression on existing table tests (issues 200, 212, 248 and all `test_*table*` tests)
- [ ] The visible text content is preserved in the paragraph output when a GFM table is suppressed (the pipe characters and cell content should appear as text)

## Test Scenarios

### Unit: GFM table suppression when not at block boundary

- Input: `"| A | B |\n|---|---|\n| 1 | 2 |\nnot a pipe\n"` -- assert output does NOT contain `<table>`, assert output contains `| A | B |` as text
- Input: `"| H1 | H2 | H3 |\n|---|---|---|\n| a | b | c |\ncontinuation text\n"` -- three-column table followed by text, assert no `<table>`
- Input: `"| A | B |\n|:---:|---:|\n| 1 | 2 |\nnot a pipe\n"` -- separator with alignment markers (`:---:`, `---:`), assert no `<table>`
- Input: `"some text\n| A | B |\n|---|---|\n| 1 | 2 |\nmore text\n"` -- no block boundary before OR after, assert no `<table>`

### Unit: GFM table preserved at block boundary

- Input: `"| A | B |\n|---|---|\n| 1 | 2 |\n\nParagraph\n"` -- blank line after, assert `<table>` present
- Input: `"| A | B |\n|---|---|\n| 1 | 2 |\n"` -- EOF after, assert `<table>` present
- Input: `"\n| A | B |\n|---|---|\n| 1 | 2 |\n\n"` -- blank lines before and after, assert `<table>` present
- Input: `"| A | B |\n|---|---|\n| 1 | 2 |\n# Heading\n"` -- block-level element after, assert `<table>` present
- Input: `"| A | B |\n|---|---|\n| 1 | 2 |\n---\n"` -- HR after, assert `<table>` present

### Unit: Mixed document with both table types

- Input containing one GFM table followed by blank line AND one GFM table followed by plain text -- assert first produces `<table>`, second does not

### Unit: Unicode preservation

- Input: `"| Spalte | Wert |\n|---|---|\n| Buecher | Zahlen |\nWeiter geht es\n"` -- assert no `<table>`, assert Unicode text preserved in output
- Input: `"| Spalte | Wert |\n|---|---|\n| Buecher | Zahlen |\n\n"` -- assert `<table>` with Unicode content

### Integration: end-to-end markdown_to_html

- Call `markdown_to_html()` with GFM table not at block boundary, verify no `<table>` in result
- Call `markdown_to_html()` with GFM table at block boundary, verify `<table>` in result

## Dependencies

- Issue 248 (done)

## Notes

- This is a higher-risk change because it modifies content that pulldown-cmark's built-in GFM parser would normally handle. The separator row may need to be escaped (e.g., replace `|` with `\|`) or transformed to prevent GFM table detection while preserving the text content.
- The fix should be in the `preprocess_kramdown_tables()` function in `src/kramdown.rs`, specifically in the code path where `is_standard_pipe_table_context` returns `true` (currently the lines just fall through to the `else` branch at line ~1876).
- The engineer should add a new code path: when `is_standard_pipe_table_context` is true, collect the full GFM table block, check `is_before_block_boundary` on the line after, and if NOT at a block boundary, escape the separator row before outputting.

## Log

### [SWE] 2026-03-24
- Wrote 13 tests covering all acceptance criteria and test scenarios (TDD step 1)
- Ran tests: 7 FAIL as expected (suppression cases), 6 PASS (preservation cases)
- Implemented fix in `src/kramdown.rs` `convert_kramdown_pipe_tables()`:
  - Added new `else if` branch for `is_standard_pipe_table_context` returning true
  - Walks backward to find table block start, forward to find end
  - Checks both `is_after_block_boundary` and `is_before_block_boundary`
  - If at proper boundaries: passes through unchanged for pulldown-cmark
  - If NOT at boundaries: escapes separator row leading `|` with `\|` to prevent GFM table detection
- Ran tests: all 13 PASS
- Fixed clippy warnings (identical if blocks, needless range loops)
- Fixed formatting with `cargo fmt`
- Full test suite: 2745 lib tests + all integration tests pass, 0 failures
- Clippy clean, fmt clean
- Files modified: `src/kramdown.rs`

### [QA] 2026-03-24
- Full test suite: 2745 lib tests passed, 0 failed; all integration test crates green
- Clippy: clean (only upstream warnings from liquid-lib, not our code)
- Formatting: `cargo fmt --check` clean
- 13 issue-265 tests all pass

Acceptance criteria verification:
- [x] `cargo build` compiles without errors
- [x] `cargo test` passes with all new and existing tests (2745 lib + integration)
- [x] `"| A | B |\n|---|---|\n| 1 | 2 |\nnot a pipe\n"` does NOT produce `<table>` -- test_265_gfm_table_no_block_boundary_after_suppressed PASS
- [x] `"| A | B |\n|---|---|\n| 1 | 2 |\n\ntext\n"` (blank line after) produces `<table>` -- test_265_gfm_table_blank_line_after_preserved PASS
- [x] `"| A | B |\n|---|---|\n| 1 | 2 |\n"` (EOF after) produces `<table>` -- test_265_gfm_table_eof_after_preserved PASS
- [x] Heading after table produces `<table>` -- test_265_gfm_table_heading_after_preserved PASS
- [x] HR after table produces `<table>` -- test_265_gfm_table_hr_after_preserved PASS
- [x] Text before without blank line does NOT produce `<table>` -- test_265_gfm_table_text_before_no_blank_line PASS
- [x] Mixed document (one table with boundary, one without) -- test_265_mixed_document_gfm_tables PASS
- [x] Unicode content preserved -- test_265_gfm_table_unicode_suppressed and test_265_gfm_table_unicode_preserved PASS
- [x] No regression on existing table tests -- full suite passes
- [x] Visible text preserved in paragraph output -- test_265_gfm_table_no_block_boundary_after_suppressed checks for "| A | B |" PASS

TDD verification: SWE log shows tests written first (13 tests), 7 failed as expected, then implementation, then all 13 pass. TDD cycle followed.

Code review notes:
- Implementation is clean: new `else if` branch in `convert_kramdown_pipe_tables()` at line 1876
- Logic is sound: walks backward to find table start, forward to find end, checks both boundaries
- Escape strategy (replacing leading `|` with `\|` in separator rows) correctly prevents pulldown-cmark GFM detection
- No unwrap in library code, no hardcoded values
- Note: diff also includes unrelated issue 275b tests and deletion of issue 275 todo file -- these are out of scope for this review but do not affect issue 265 correctness

VERDICT: **PASS**

### [PM] 2026-03-24

Acceptance review of issue 265: GFM table before block boundary fix.

Acceptance criteria verification (all 11 met):
- [x] `cargo build` compiles without errors
- [x] `cargo test` passes with all new and existing tests (2745 lib + integration)
- [x] Table followed by non-pipe text suppressed (no `<table>`) -- test PASS
- [x] Blank line after table preserves `<table>` -- test PASS
- [x] EOF after table preserves `<table>` -- test PASS
- [x] Heading after table preserves `<table>` -- test PASS
- [x] HR after table preserves `<table>` -- test PASS
- [x] Text before table without blank line suppresses `<table>` -- test PASS
- [x] Mixed document: one table rendered, one suppressed -- test PASS
- [x] Unicode content preserved in both cases -- tests PASS
- [x] No regression on existing table tests -- full suite green
- [x] Visible text preserved in paragraph output -- test checks for pipe content in output

Implementation review:
- Single new `else if` branch in `convert_kramdown_pipe_tables()` -- clean, focused change
- Walks backward/forward to find table block boundaries, checks both `is_after_block_boundary` and `is_before_block_boundary`
- Escape strategy (leading `|` to `\|` on separator rows) is sound and minimal
- 13 new tests cover all specified scenarios including suppression, preservation, mixed docs, unicode, and alignment markers
- TDD followed: tests written first, 7 failed as expected, then implementation, all 13 pass
- No silent descoping -- all criteria from the groomed spec are addressed

Note: diff includes unrelated issue 275b test additions and deletion of issue 275 todo file. These are out of scope but do not affect issue 265 correctness.

VERDICT: **ACCEPT**
