# Issue 515: Kramdown table tfoot and multi-tbody full-width separators

## Problem

Kramdown supports table body and footer separator rows that span the full width
without per-column pipes. When a kramdown table contains:

```markdown
| Header1 | Header2 | Header3 |
|:--------|:-------:|--------:|
| cell1   | cell2   | cell3   |
| cell4   | cell5   | cell6   |
|-----------------------------|
| cell1   | cell2   | cell3   |
| cell4   | cell5   | cell6   |
|=============================|
| Foot1   | Foot2   | Foot3   |
```

Jekyll/kramdown produces:
- A `<thead>` for the header row
- Two separate `<tbody>` sections (split by the `|-----|` separator)
- A `<tfoot>` section (after the `|=====|` separator)

Rustkyll renders the separator rows as literal cell content (em-dashes and `=====`),
producing extra `<tr>` rows with dashes/equals signs instead of structural elements.

### Rustkyll output (wrong)

```html
<thead><tr><th>Header1</th><th>Header2</th><th>Header3</th></tr></thead>
<tbody>
  <tr><td>cell1</td><td>cell2</td><td>cell3</td></tr>
  <tr><td>cell4</td><td>cell5</td><td>cell6</td></tr>
  <tr><td>-----</td><td></td><td></td></tr>
  <tr><td>cell1</td><td>cell2</td><td>cell3</td></tr>
  <tr><td>cell4</td><td>cell5</td><td>cell6</td></tr>
  <tr><td>=============================</td><td></td><td></td></tr>
  <tr><td>Foot1</td><td>Foot2</td><td>Foot3</td></tr>
</tbody>
```

### Expected output (Jekyll)

```html
<thead><tr><th>Header1</th><th>Header2</th><th>Header3</th></tr></thead>
<tbody>
  <tr><td>cell1</td><td>cell2</td><td>cell3</td></tr>
  <tr><td>cell4</td><td>cell5</td><td>cell6</td></tr>
</tbody>
<tbody>
  <tr><td>cell1</td><td>cell2</td><td>cell3</td></tr>
  <tr><td>cell4</td><td>cell5</td><td>cell6</td></tr>
</tbody>
<tfoot>
  <tr><td>Foot1</td><td>Foot2</td><td>Foot3</td></tr>
</tfoot>
```

## Affected Pages

- hydeout: `markup/2012/01/11/markup-html-elements-and-formatting.html` (8 of 77 diffs are from this)
- Potentially any kramdown site using full-width table separators

## Root Cause Analysis

**Primary file:** `src/kramdown.rs`
**Function:** `convert_kramdown_pipe_tables` (line ~2585)
**Secondary:** `is_kramdown_table_line` (line ~2926), `is_standard_pipe_table_context` (line ~2986)

The rendering pipeline for kramdown-configured sites is:
1. `convert_kramdown_pipe_tables` pre-processes kramdown-only tables into HTML
2. For tables that look like "standard" GFM pipe tables (those with a `|---|---|---|` header separator), the function delegates to pulldown-cmark via `is_standard_pipe_table_context`
3. pulldown-cmark renders the table but does NOT understand kramdown full-width separators

For the hydeout table:
- The header separator `|:--------|:-------:|--------:|` is per-column, so `is_standard_pipe_table_context` returns `true`
- `convert_kramdown_pipe_tables` skips the table, letting pulldown-cmark handle it
- pulldown-cmark creates `<thead>` and `<tbody>`, but treats `|-----------------------------|` and `|=============================|` as data rows (single-cell with dashes/equals)

Note: `is_kramdown_table_line` already filters out per-column separators (lines where all chars are `-`, `:`, `|`, space). But full-width separators like `|-----------------------------|` also match this filter, so they return `false` and would break the kramdown table collection loop if the table were handled by that path.

The kramdown parser (`src/kramdown_parser/parser.rs::try_parse_separator_line`) already handles full-width separators correctly for its own table parsing. But the main rendering pipeline does not use the kramdown parser for table rendering.

**Fix approach:** Either:
- (A) Pre-process the markdown to strip full-width separator lines and convert them to markers, then post-process the HTML to split `<tbody>` and add `<tfoot>` -- similar to how other kramdown features are handled
- (B) Detect tables with full-width separators in `convert_kramdown_pipe_tables` and handle them through the kramdown table conversion path instead of delegating to pulldown-cmark
- (C) Post-process pulldown-cmark's HTML output: detect `<tr>` rows whose single cell contains only dashes or equals signs, and restructure them into `<tbody>`/`<tfoot>` boundaries

Approach (C) is likely simplest and least invasive.

## Scope

Detect and handle full-width kramdown table body/footer separators (`|[-]+|` and
`|[=]+|` patterns) so they produce correct `<tbody>` splits and `<tfoot>` wrapping
instead of data rows with dashes/equals content.

## Dependencies

None.

## DTC DOM Baseline

- Current: 790/790
- Must not drop below: 790/790

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes clean
- [ ] `cargo fmt` produces no changes
- [ ] A kramdown table with `|----|` full-width separator produces two `<tbody>` sections
- [ ] A kramdown table with `|====|` full-width separator produces a `<tfoot>` section
- [ ] A kramdown table with both `|----|` and `|====|` produces correct multi-tbody + tfoot
- [ ] Per-column separator rows (`|---|---|---|`) continue to work (no regression)
- [ ] DTC DOM match count must not drop below 790/790
- [ ] Hydeout DOM match count improves (target: reduce 8 table diffs to 0)
- [ ] No regression on any other site's DOM score

## Test Scenarios

### Unit: Full-width body separator

- Input: table with `|----|` between data rows
- Expected: two `<tbody>` sections, each containing the data rows on its side
- Also test with longer dashes: `|-----------------------------|`
- Also test with leading/trailing spaces: `| --------- |`

### Unit: Full-width footer separator

- Input: table with `|====|` before last row
- Expected: `<tfoot>` wrapping the last row(s)
- Also test with longer equals: `|=============================|`

### Unit: Combined separators (exact hydeout pattern)

Input:
```markdown
| Header1 | Header2 | Header3 |
|:--------|:-------:|--------:|
| cell1   | cell2   | cell3   |
| cell4   | cell5   | cell6   |
|-----------------------------|
| cell1   | cell2   | cell3   |
| cell4   | cell5   | cell6   |
|=============================|
| Foot1   | Foot2   | Foot3   |
```

Expected output must contain:
- Exactly 1 `<thead>`
- Exactly 2 `<tbody>` sections
- Exactly 1 `<tfoot>`
- Column alignment preserved (`text-align: left/center/right`)
- No `<tr>` with dashes or equals as cell content

### Unit: No regression on per-column separators

- Parse table with `|---|---|---|` separator, verify it still works as header separator
- Parse table with `|:--|:--:|--:|` alignment separator, verify alignment preserved
- Parse table with per-column footer `|===|===|===|`, verify `<tfoot>` still works

### Unit: No false positive on dash content

- A table cell that legitimately contains dashes (e.g. `| ---N/A--- |`) must NOT be treated as a separator
- A separator must match the pattern: `|` followed by only `-` or `=` and spaces, ending with `|`

### Integration: Hydeout site

- Build hydeout site, verify `markup-html-elements-and-formatting.html` table renders with correct structure
- Run DOM comparison against Jekyll output, verify the 8 table-related diffs are resolved
- No regression on other hydeout pages

## Priority

MEDIUM -- affects hydeout and potentially other kramdown sites using full-width table separators

## Log

### [SWE] 2026-03-30
- TDD: Wrote 5 failing tests first (fullwidth body, footer, combined, per-column, false positive)
- Ran tests: FAILS as expected -- all 3 main tests fail, 2 regression tests pass
- Implemented `restructure_kramdown_table_separators()` in src/kramdown.rs
- Approach (C): post-processes pulldown-cmark HTML to detect separator `<tr>` rows
- Detects dashes (`-`), em-dashes (`\u{2014}`), en-dashes (`\u{2013}`) for tbody split
- Detects equals (`=`) for tfoot boundary
- Added `tfoot` to CONTAINER_TAGS and BLOCK_TAGS in `wrap_bare_text_in_paragraphs` to prevent `<tfoot>` being wrapped in `<p>`
- Called from all 3 markdown rendering functions in frontmatter.rs
- Added 2 additional unit tests for direct function testing
- Ran tests: 7/7 issue 515 tests PASS
- Full suite: 3378 passed, 3 failed (pre-existing from other in-progress issues 449, 537)
- Clippy clean, fmt clean
- Hydeout DOM: markup-html-elements page diffs reduced from 77 to 70 (7 table-related diffs resolved)
- DTC DOM: 790/790 with my changes alone (788/790 includes regressions from other in-progress changes 516, 537, 348 in working tree)
- Files modified: src/kramdown.rs, src/frontmatter.rs
