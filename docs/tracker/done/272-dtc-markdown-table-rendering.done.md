# Issue 272: Kramdown table detection too strict -- requires trailing pipe

## Problem

On 4 DTC book review pages, text containing pipe characters is rendered as `<p>` by rustkyll but as `<table>` by Jekyll/kramdown. The DOM comparison shows `tag_name_differs: expected 'table', actual 'p'`.

## Root Cause

In kramdown, ANY line that contains a `|` character is treated as a potential table row, even if the `|` is in the middle of the line and the line does not start or end with `|`. The only requirement is that the line appears at a block boundary (preceded by blank line / SOF and followed by blank line / EOF / block element).

Rustkyll's `is_kramdown_table_line()` function in `src/kramdown.rs` (line ~1007) requires `content.ends_with('|')`, which is too strict. This causes lines with embedded pipes to be skipped by the kramdown table converter and rendered as paragraphs instead.

## Affected Pages (4)

All failures are in book archive Q&A threads where text from YAML front matter goes through the `{{ text | newline_to_br | markdownify }}` pipeline:

1. **books/20211018-blueprints-for-text-analytics-using-python.html**
   - Text: `I think <#C01AXGTRESH|books> would be a better channel for this`
   - Slack channel reference with pipe

2. **books/20220627-designing-machine-learning-systems.html**
   - Text: `...branches in ML (NLP  | CV | Time series | ...), but unable to choose...`
   - Literal pipe-separated list in prose

3. **books/20220815-fundamentals-of-data-engineering.html**
   - Text: `enroll in <#C01FABYF2RG|course-data-engineering> ?`
   - Slack channel reference with pipe

4. **books/20221010-managing-machine-learning-projects.html**
   - Text: `...email me at <mailto:simon.2.thompson@gmail.com|simon.2.thompson@gmail.com> if you want...`
   - Mailto link with pipe

## Fix

Relax `is_kramdown_table_line()` to detect lines that CONTAIN a `|` anywhere, not just lines that END with `|`. Specifically:

1. Remove or relax the `content.ends_with('|')` check on line ~1007
2. Replace with a check that `content` contains at least one `|` character (after stripping list prefixes)
3. Keep the existing separator-line exclusion (lines that are all `-`, `:`, `|`, space should still be excluded)

**Caution:** Relaxing this check could cause false-positive table detection for lines that contain `|` but are not intended as tables. The existing block-boundary checks (`is_after_block_boundary` / `is_before_block_boundary`) and the `is_standard_pipe_table_context` check should prevent most false positives, but new tests must verify no regressions on existing pages.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all new and existing tests
- [ ] `is_kramdown_table_line()` detects lines with embedded pipes (not just trailing pipes)
- [ ] `markdownify` filter on `I think <#C01AXGTRESH|books> would be a better channel for this` produces `<table>` with 2 `<td>` cells
- [ ] `markdownify` filter on `branches in ML (NLP  | CV | Time series | ...)` produces `<table>` with 4 `<td>` cells
- [ ] `markdownify` filter on `email me at <mailto:a@b.com|a@b.com> if you want` produces `<table>` with 2 `<td>` cells
- [ ] Line with `|` followed by non-pipe text (not at block boundary) is NOT treated as table
- [ ] Line with `|` preceded by non-pipe text (not at block boundary) is NOT treated as table
- [ ] No regressions on existing table tests (issues 200, 212, 248)
- [ ] Build the DTC site and verify all 4 affected pages now show `<table>` elements matching Jekyll output

## Test Scenarios

### Unit: is_kramdown_table_line relaxation

- `text | more text` returns true (pipe in middle, no trailing pipe)
- `<#C01AXGTRESH|books> would be better` returns true (single embedded pipe)
- `NLP  | CV | Time series | ...` returns true (multiple embedded pipes)
- `|---|---|` returns false (separator line, existing exclusion)
- `no pipe here` returns false (no pipe at all)
- `just text` returns false (no pipe)
- Line with only whitespace and dashes returns false

### Unit: markdownify with embedded-pipe table lines

- Single line `text | more text` at block boundary produces `<table>` with 2 cells
- Single line `<#C01AXGTRESH|books> text` produces `<table>` with 2 cells
- Single line `a | b | c | d` produces `<table>` with 4 cells
- Single line `<mailto:a@b.com|a@b.com> more` produces `<table>` with 2 cells
- Non-ASCII/Unicode content with embedded pipes produces correct `<table>`

### Unit: no false positives from relaxation

- `text | more\nnon-pipe continuation` does NOT produce table (not at block boundary)
- `paragraph text\nhas | pipe\nmore text` does NOT produce table (preceded by non-blank)
- Lines containing `|` inside code blocks are NOT treated as tables
- Existing issue 248 tests still pass (pipe lines followed by non-pipe text)

### Integration: DTC book pages

- Build the full DTC site (ignored test) and compare the 4 affected pages against Jekyll output
- Verify `<table>` presence in all 4 pages
- Verify cell content matches Jekyll (e.g., `<td>I think &lt;#C01AXGTRESH</td>` and `<td>books&gt; would be a better channel for this</td>`)

## Dependencies

- Issue 200 (done) -- markdown table rendering
- Issue 212 (done) -- multi-row standard pipe table fix
- Issue 248 (done) -- kramdown pipe table rules

## Notes

- Issue 265 (todo) is the OPPOSITE problem: GFM tables that should NOT render because they lack a block boundary after. Issue 272 is about kramdown-style tables (no separator line) that SHOULD render but are being missed.
- The `split_kramdown_table_cells` function already handles splitting on `|` correctly; only the detection (`is_kramdown_table_line`) needs to change.
- The `<...>` angle brackets in Slack references and mailto links will be HTML-entity-escaped by the markdown processor, matching Jekyll's output (e.g., `&lt;#C01AXGTRESH`).

## Log

### [SWE] 2026-03-20
- TDD Step 1: Wrote 16 tests (7 unit tests for `is_kramdown_table_line`, 5 markdownify integration tests, 4 no-false-positive tests)
- TDD Step 2: Ran tests -- 8 failed as expected (the ones requiring embedded pipe detection)
- TDD Step 3: Changed `content.ends_with('|')` to `content.contains('|')` in `is_kramdown_table_line()` (line 1007)
- TDD Step 4: All 16 new tests pass
- One pre-existing test `test_200_no_false_table` failed: it asserted a standalone line with embedded pipe should NOT be a table, but kramdown DOES treat it as a table. Updated test to match correct kramdown behavior.
- Full test suite: 2017 lib + all integration tests pass, 0 failures
- Clippy: pre-existing errors in `liquid-core` dependency only, no errors in rustkyll code
- Fmt: clean
- Files modified: `src/kramdown.rs` (1 line fix + 16 new tests + 1 updated test)
