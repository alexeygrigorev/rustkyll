# Issue 294: Block-level link definition and table parsing interaction

## Problem

In kramdown Ruby, link definitions are processed during block parsing, preserving block context. In rustkyll, link definitions are extracted in a pre-pass (`extract_definitions`), which removes them from the text before block parsing. This causes a difference when a link definition is immediately followed by a pipe-delimited line:

```
[5]: test
|no|table|here|
```

In kramdown Ruby: `[5]: test` is consumed as a link definition during block parsing. The remaining `|no|table|here|` inherits the paragraph context and becomes a paragraph.

In rustkyll: `[5]: test` is removed during pre-pass. `|no|table|here|` becomes a standalone block and is parsed as a single-row table.

Descoped from issue 291 (kramdown remaining ignored tests).

## Root Cause

In `span_parser.rs`, `extract_definitions()` simply skips the link definition line (via `continue`), so the output text loses the line entirely. When content immediately follows a removed link definition with no blank line between them, that content becomes a new standalone block instead of inheriting the paragraph context from the preceding block.

Specifically, lines 113-149 of `span_parser.rs`: after a link definition is parsed and stored, the loop `continue`s without emitting anything to `output_lines`. Contrast this with footnote definitions (line 300) which insert an EOB marker `"^"` to preserve block boundaries.

## Approach

Modify `extract_definitions()` in `src/kramdown_parser/span_parser.rs` so that when a link definition is removed and the next line is non-blank content (i.e., the link def was immediately followed by content without a blank line), the context is preserved. The simplest correct approach: when a link def is removed and there is non-blank content on the line(s) immediately after it, **and** non-blank content on the line(s) immediately before it, join those content blocks into a single paragraph by not inserting any blank line between them.

However, the specific failing test case has the link def at a block boundary (preceded by a blank line), so the real issue is: when a link def is removed at a block boundary, the immediately following non-blank line should NOT start a new block that could be misinterpreted as a table. In kramdown Ruby, the `|no|table|here|` line is treated as a paragraph because the link def consumed the "start of block" context.

The recommended fix: when removing a link definition line that is immediately followed by content (no blank line between them), and that content starts with `|`, treat the following line as paragraph text by emitting a non-pipe prefix or by having the block parser understand this context. The cleanest approach is likely to have `extract_definitions` leave a paragraph-continuation marker or simply not remove the link def line but replace it with an empty line (which preserves the blank-line separation and makes `|no|table|here|` a standalone paragraph since a single pipe-row after a blank is not a valid table).

Actually, looking at the expected output more carefully:

```html
<p>|no|table|here|</p>
```

All four test cases in the `errors` fixture produce `<p>` tags. The issue is only with the second case. The fix should ensure the conformance test `block/14_table/errors` passes.

## Key Files

- `src/kramdown_parser/span_parser.rs` -- `extract_definitions` pre-pass (primary change)
- `src/kramdown_parser/parser.rs` -- block parser (may need adjustment)
- `src/kramdown_parser/tests.rs` -- change `conformance_test_deferred!` to `conformance_test!`
- `src/kramdown_parser/testcases/block/14_table/errors.text` -- test input
- `src/kramdown_parser/testcases/block/14_table/errors.html` -- expected output

## Dependencies

None. This is a standalone kramdown parser fix.

## Acceptance Criteria

- [ ] The conformance test `block/14_table/errors` passes (change from `conformance_test_deferred!` to `conformance_test!` in `tests.rs`)
- [ ] The input `[5]: test\n|no|table|here|` produces `<p>|no|table|here|</p>` (not a table)
- [ ] The link definition `[5]: test` is still correctly extracted and usable for references like `[text][5]`
- [ ] All existing kramdown conformance tests continue to pass (`cargo test` in the `kramdown_parser` module)
- [ ] No regressions in full site generation tests (`cargo test` overall)
- [ ] `cargo clippy -- -D warnings` passes

## Test Scenarios

### Unit: Link definition extraction with adjacent pipe content
- Input `[5]: test\n|no|table|here|` should produce HTML `<p>|no|table|here|</p>` (not a table)
- Input `[5]: test\n|no|table|here|` should still register link def `5` with url `test`
- Input `[id]: http://example.com\n|a|b|c|` should produce a paragraph, not a table

### Unit: Link definition extraction without adjacent content (no regression)
- Input `[5]: test\n\n|col1|col2|\n|---|---|\n|a|b|` should still produce a table (blank line separates, so table is valid)
- Input `[5]: test` alone should extract the link def and produce empty output
- Input with link def between two paragraphs: `para1\n\n[5]: test\n\npara2` should produce two separate paragraphs

### Conformance: block/14_table/errors
- All four cases in the test fixture must produce the expected `<p>` output:
  1. `No table body\n\n|-|-|-` produces `<p>|-|-|-</p>` (no table body = not a table)
  2. `[5]: test\n|no|table|here|` produces `<p>|no|table|here|</p>` (link def context)
  3. `|no|table|here|\nparagraph` produces `<p>|no|table|here|\nparagraph</p>` (paragraph continuation)
  4. `|-|-|\n|-|-|` produces `<p>|-|-|\n|-|-|</p>` (separator-only = not a table)

### Regression: Existing table parsing
- Valid tables with header rows, separator rows, and body rows must still parse correctly
- Tables preceded by blank lines must still parse correctly
- The other `block/14_table/*` conformance tests must continue to pass

## Log

### [SWE] 2026-03-24

**TDD Step 1: Write failing tests**
- Changed `conformance_test_deferred!` to `conformance_test!` for `kramdown_block_14_table_errors`
- Added 5 unit tests:
  - `test_issue294_link_def_followed_by_pipe_line_is_paragraph`
  - `test_issue294_link_def_still_extracted`
  - `test_issue294_link_def_with_blank_line_then_table_still_works`
  - `test_issue294_link_def_between_paragraphs_no_regression`
  - `test_issue294_unicode_link_def_followed_by_pipe`
- Ran tests: 2 FAIL as expected (pipe lines parsed as tables instead of paragraphs)
- Conformance test also FAILS: `|no|table|here|` rendered as `<table>` instead of `<p>`

**TDD Step 2: Implement fix**

Two-part fix:

1. **span_parser.rs**: When `extract_definitions` removes a link definition and the immediately following line starts with `|` (pipe), emit a `LINKDEF_REMOVED_MARKER` sentinel. This tells the block parser that the next content was originally preceded by a link def and should not be parsed as a table (matching kramdown Ruby's block-level link def processing).

2. **parser.rs**:
   - Added `LINKDEF_REMOVED_MARKER` recognition in `parse_blocks_with_lazy` and `parse_blocks_list_context` (treated as EOB + sets `inhibit_table` flag)
   - Added marker break checks in all paragraph parsers
   - Added marker check in `try_parse_table`'s post-scan validation
   - When table parsing is inhibited (`skip_table`), pipe-starting lines fall through to paragraph parsing
   - When table parser rejects a block that starts with an actual table line (has `|`), collect all rejected lines into a single paragraph instead of re-breaking them individually. This fixes the `|-|-|\n|-|-|` case (all-separator lines) and the `|no|table|here|\nparagraph` case (non-table continuation).

**TDD Step 3: Run tests, verify pass**
- All 5 unit tests: PASS
- Conformance test `block/14_table/errors`: PASS
- Full test suite: 2785 lib tests + all integration tests, 0 failures
- Clippy: clean (no warnings)
- Format: clean

**Files modified:**
- `src/kramdown_parser/span_parser.rs` -- added `LINKDEF_REMOVED_MARKER` constant, emit marker in `extract_definitions`
- `src/kramdown_parser/parser.rs` -- marker handling in block parsers, `inhibit_table` flag, rejected-table-to-paragraph logic
- `src/kramdown_parser/tests.rs` -- changed deferred test to active, added 5 unit tests

### [QA] 2026-03-24

**Test Results:**
- Full test suite: 2785 lib tests + all integration tests, 0 failures
- `cargo clippy -- -D warnings`: clean (no project warnings)
- `cargo fmt --check`: clean

**Acceptance Criteria:**
1. Conformance test `block/14_table/errors` passes (promoted from deferred): PASS
2. `[5]: test\n|no|table|here|` produces `<p>|no|table|here|</p>`: PASS
3. Link definition still extracted and usable for references: PASS
4. All existing kramdown conformance tests pass: PASS
5. No regressions in full site generation tests: PASS
6. `cargo clippy -- -D warnings` passes: PASS

**Code Review:**
- Marker approach (`LINKDEF_REMOVED_MARKER`) follows existing `"^"` EOB pattern for footnotes
- No unwrap in library code
- `inhibit_table` flag properly consumed after one line
- Rejected-table-to-paragraph fallback handles edge cases correctly
- 5 unit tests + 1 conformance test, including Unicode (Cyrillic) content
- TDD log shows proper red-green cycle

VERDICT: **PASS**

### [PM] 2026-03-24

**Acceptance Criteria Verification:**

1. Conformance test `block/14_table/errors` passes (promoted from deferred): VERIFIED
2. `[5]: test\n|no|table|here|` produces `<p>|no|table|here|</p>`: VERIFIED
3. Link definition still extracted and usable for references: VERIFIED (test checks `[click][5]` resolves)
4. All existing kramdown conformance tests pass: VERIFIED (2785 passed, 0 failed)
5. No regressions in full site generation tests: VERIFIED
6. `cargo clippy -- -D warnings` passes: VERIFIED (clean)

**Code Review:**

- Marker approach (`LINKDEF_REMOVED_MARKER`) follows existing `"^"` EOB pattern for footnotes -- consistent with codebase conventions
- Marker is only emitted when the next line starts with `|`, minimizing unnecessary impact
- `inhibit_table` flag is single-use (consumed after one content line) -- correct scoping
- Rejected-table-to-paragraph fallback handles all four error cases in the conformance test
- No `unwrap` in library code
- 5 unit tests + 1 conformance test; includes Unicode content and regression scenarios
- TDD cycle followed properly (red-green confirmed in SWE log)

**Descoping Check:** All 6 acceptance criteria met. No descoping.

VERDICT: **ACCEPT**
