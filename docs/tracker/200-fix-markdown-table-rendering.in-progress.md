# Issue 200: Fix markdown table rendering failure (109 pages)

## Checklist Category

**Markdown table rendering failure** -- 109 pages

## Problem

109 pages have tables not rendered as HTML `<table>` elements. Instead, table content appears as raw text within other elements.

Breakdown by site:
- alexeygrigorev-mlwiki.org (108): Tables inside list items, wiki-style tables with `|` pipes, tables with non-standard formatting
- DTC (1): Table inside a list item in a book review page

## Goal

Render markdown tables correctly in all contexts, especially inside list items.

## Dependencies

- Issue 199 (markdown block structure) -- related but separate. Block structure handles non-table elements; this issue focuses specifically on `<table>` rendering.

## Sub-tasks

### Sub-task 1: Investigation

1. Read `docs/comparison/dom-details/alexeygrigorev-mlwiki.org.txt` and extract all table-related diffs (`expected: '<table>'` or `missing_element - expected: '<table>'`).
2. For 3-5 sample pages, compare the actual markdown source with the expected HTML output. Determine:
   - Are these standard pipe tables (`| col1 | col2 |`)?
   - Are they inside list items (`- item\n  | col1 | col2 |`)?
   - Are they MediaWiki-style tables?
3. Read the DTC book page source to see the specific table pattern.
4. Check what pulldown-cmark does with tables inside list items.

### Sub-task 2: Fix tables inside list items

pulldown-cmark may not recognize table syntax when indented inside a list item. This needs either:
- A pulldown-cmark option/extension to enable
- Post-processing in `src/kramdown.rs` to detect and convert

### Sub-task 3: Fix non-standard table formats

If mlwiki.org uses wiki-style table syntax that neither pulldown-cmark nor kramdown supports natively, document as known limitation.

## TDD Test Scenarios

### Test 1: Standard pipe table renders correctly (baseline, should pass)

```rust
#[test]
fn test_standard_pipe_table_renders() {
    // Setup: Markdown:
    //   | Header 1 | Header 2 |
    //   |----------|----------|
    //   | Cell 1   | Cell 2   |
    //
    // Assert: Produces <table> with <thead>, <tbody>, <tr>, <th>, <td>.
    // This should already pass -- it's a baseline test.
}
```

### Test 2: Table inside list item (write FIRST, verify it fails)

```rust
#[test]
fn test_table_inside_list_item() {
    // Setup: Markdown:
    //   - Item with table:
    //
    //     | Col A | Col B |
    //     |-------|-------|
    //     | val1  | val2  |
    //
    // Assert: Produces <ul><li> containing text AND a <table> element.
    //   The table should NOT appear as raw text like "| Col A | Col B |".
    //
    // Verify it FAILS before implementing.
}
```

### Test 3: Wiki-style pipe table (investigate first)

```rust
#[test]
fn test_wiki_style_pipe_table() {
    // Setup: Use actual markdown from an mlwiki.org page that has
    //   a table diff. Extract the exact markdown source.
    //
    // Assert: Whatever Jekyll produces for this input.
    //
    // This test requires investigation first to determine the exact input format.
}
```

### Test 4 (integration, #[ignore]): Build DTC and verify table

```rust
#[test]
#[ignore]
fn test_dtc_book_page_table_renders() {
    // Build DTC site
    // Parse books/20220425-natural-language-processing-with-transformers.html
    // Verify <table> element exists inside the list item
}
```

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with table rendering tests
- [ ] Investigation documents the exact table formats used across affected pages
- [ ] Standard pipe tables continue to render correctly (no regressions)
- [ ] Tables inside list items render as `<table>` elements, not raw text
- [ ] DTC book page table fixed (1 page)
- [ ] mlwiki.org tables: fix standard markdown tables that are inside list items; document any wiki-only table formats as known limitations

## Log

### [SWE] 2026-03-18

**Investigation findings:**

1. **mlwiki.org (108 pages)**: The tables are kramdown-specific -- any line ending with `|` is treated as a table row. The `|` characters split the line into `<td>` cells. No header separator line (`|---|---|`) is needed. Examples:
   - Single cell: `- can use Prim's algo |` becomes `<li><table><tbody><tr><td>can use Prim's algo</td></tr></tbody></table></li>`
   - Multi-cell: `text1 | text2 | text3 |` splits into 3 `<td>` cells
   - These appear inside list items at various nesting depths
   - Many contain mathematical notation ($x$ and LaTeX)
   - Some contain Russian/Cyrillic text (Bayes_Theorem page)

2. **DTC (1 page)**: The book page `20220425-natural-language-processing-with-transformers` has a list item where `<tel:100-1000|100-1000>` gets treated by kramdown as containing a table cell because of the `|` character. This is the same kramdown pipe-table behavior.

3. **Standard pipe tables** (with header separator line) already work correctly via pulldown-cmark's `ENABLE_TABLES` option.

4. **Tables inside list items** with standard format (indented by 2+ spaces with separator line) already work in pulldown-cmark.

**Implementation:**

- Added `convert_kramdown_pipe_tables()` as a pre-processing step in `src/kramdown.rs`
- The function detects lines ending with `|` (after stripping list item prefixes)
- Converts them to raw HTML `<table><tbody><tr><td>...</td></tr></tbody></table>` before pulldown-cmark
- Correctly avoids converting standard CommonMark pipe tables (those with separator lines)
- Handles list item context, indentation, and multi-row tables
- Wired into both `markdown_to_html()` and `markdown_to_html_for_filter()` in `src/frontmatter.rs`

**Known limitation:** The mlwiki.org wiki-style tables use kramdown's pipe table syntax which is not a separate "wiki format" -- it IS kramdown's table format. All 108 mlwiki pages use this same pattern (line ending with `|`). This implementation handles all of them.

**Tests:** 11 new tests, all passing:
- `test_200_standard_pipe_table_renders` - baseline, standard tables still work
- `test_200_pipe_table_unicode` - Cyrillic/accented chars in standard tables
- `test_200_table_inside_list` - standard table inside list item
- `test_200_table_inside_list_unicode` - same with Cyrillic content
- `test_200_kramdown_trailing_pipe_in_list` - kramdown `- text |` in list
- `test_200_kramdown_multi_pipe_in_list` - kramdown `- a | b | c |` in list
- `test_200_kramdown_pipe_unicode` - kramdown pipe with Russian text
- `test_200_no_false_table` - pipe in middle of text is NOT a table
- `test_200_trailing_pipe_not_in_list` - kramdown pipe outside list context
- `test_200_multi_row_pipe` - consecutive pipe lines form multi-row table
- `test_200_no_double_convert` - standard tables not double-converted

**Build:** 1575 tests pass (11 new), 4 pre-existing failures from other in-progress issues (198/206/207). Clippy clean, fmt clean.

**Files modified:**
- `src/kramdown.rs` - Added `convert_kramdown_pipe_tables()` and helper functions, 11 tests
- `src/frontmatter.rs` - Wired pipe table conversion into `markdown_to_html()` and `markdown_to_html_for_filter()`
