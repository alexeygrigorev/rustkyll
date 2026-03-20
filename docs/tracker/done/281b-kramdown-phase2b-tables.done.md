# Issue 281b: Kramdown parser Phase 2b - Tables

## Problem

Kramdown pipe tables support headers, footers, multiple bodies, column alignment, escaped pipes, and code spans containing pipes. The parser must correctly split cells, detect separator lines, and produce well-structured `<table>` / `<thead>` / `<tbody>` / `<tfoot>` HTML.

## Scope

Implement kramdown table parsing and HTML rendering:

- **Basic tables**: pipe-separated cells, with or without leading/trailing pipes
- **Header rows**: detected by a separator line (`|---|---| ` or `|:---:|---:|`) below the first row
- **Footer rows**: separator line using `=` instead of `-` (`|===|===|`)
- **Multiple bodies**: separator lines within the table create new `<tbody>` sections
- **Column alignment**: `:---` (left), `:---:` (center), `---:` (right), `---` (default), rendered as `style="text-align: ..."` on `<th>` and `<td>`
- **Escaped pipes**: `\|` inside cells, `\|` at end of line
- **Code spans in cells**: pipes inside backtick spans are not cell separators
- **Missing cells**: rows with fewer cells than the header get empty `<td>` cells
- **Table IAL**: `{:.cls}` before or after table applies to `<table>`
- **Not-a-table detection**: lines that look like tables but aren't (no body, escaped, etc.)
- **Empty HTML tags in cells**: with `html_to_native` option

## Dependencies

- Issue #280 (Phase 2a) must be `.done.md`
- Issue #281a (Lists) is NOT required -- tables are independent of lists

## Test Cases to Pass

All `.text`/`.html` pairs in `block/14_table/`:

| Test file | What it tests | Options |
|-----------|---------------|---------|
| `simple` | Basic pipe tables, missing cells, escaped pipes, code spans in cells, IAL, tables without leading bar, separator line creating header | none |
| `header` | Simple/full header separators, alignment (left/center/right/default), leading sep line, multiple bodies, tab in separator | none |
| `footer` | Footer separator (`=`), footer with body separators, empty footer | none |
| `escaping` | Escaped pipes, code spans containing pipes, multi-line code spans across rows | none |
| `errors` | Non-tables: no body, followed by paragraph, consecutive separator lines | none |
| `no_table` | Fully escaped pipes produce paragraph, not table | none |
| `empty_tag_in_cell` | `<br>` in cell with `html_to_native: true` converts to `<br />` | `html_to_native: true` |
| `table_with_footnote` | Table cell containing footnote reference (depends on footnote support) | none |

**Note:** `table_with_footnote` requires footnote support. If footnotes are not yet implemented, this test should be deferred and tracked.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes all existing tests (no regressions)
- [ ] Conformance tests pass for `simple`, `header`, `footer`, `escaping`, `errors`, `no_table` (6 tests minimum)
- [ ] Tables render with `<table>` / `<tbody>` / `<tr>` / `<td>` structure
- [ ] Header separator line produces `<thead>` with `<th>` cells
- [ ] Footer separator line produces `<tfoot>`
- [ ] Column alignment renders as `style="text-align: left|center|right"` on cells
- [ ] Multiple body separators produce separate `<tbody>` elements
- [ ] Escaped pipes (`\|`) render as literal `|` in cell content
- [ ] Pipes inside code spans (`` `code | span` ``) do not split cells
- [ ] Rows with fewer cells than the maximum get empty `<td>` filler cells containing ` ` (space)
- [ ] Lines that don't form valid tables (no body after separator, etc.) render as paragraphs
- [ ] IAL before or after table applies attributes to the `<table>` element
- [ ] `empty_tag_in_cell` test passes with `html_to_native` option support
- [ ] If `table_with_footnote` cannot pass (footnotes not implemented), a follow-up issue is created

## Test Scenarios

### Unit: Table line detection
- Line `| cell1 | cell2 |` detected as table row
- Line `cell1 | cell2` detected as table row (no leading pipe)
- Line `|---|---|` detected as separator (header)
- Line `|===|===|` detected as separator (footer)
- Line `|:---|:---:|---:|` detected as separator with alignment
- Line `\| not | a table` is NOT a table (escaped first pipe)

### Unit: Cell splitting
- `| cell1 | cell2 |` splits to ["cell1", "cell2"]
- `| cell1 \| continued | cell2 |` splits to ["cell1 | continued", "cell2"]
- `` | cell `code | span` | cell2 | `` splits to ["cell `code | span`", "cell2"]
- `| cell1 ||` splits to ["cell1", "", ""]

### Unit: Alignment parsing
- `---` = default (no style)
- `:---` = left
- `:---:` = center
- `---:` = right
- Superfluous alignment defs (more columns in separator than in data) are ignored

### Integration: Full table rendering
- Parse `simple.text`, compare to `simple.html`
- Parse `header.text`, verify `<thead>` / `<th>` with alignment styles
- Parse `footer.text`, verify `<tfoot>` sections
- Parse `errors.text`, verify non-tables render as `<p>`
