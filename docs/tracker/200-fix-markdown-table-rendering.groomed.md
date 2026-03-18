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
