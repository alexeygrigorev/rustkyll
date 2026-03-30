# Issue 366: DTC table inside list items / tbody placement

## Parent

Follow-up from #363 (RC-E).

## Problem

In the `books/20220425-natural-language-processing-with-transformers.html` page, a comment contains the text `<tel:100-1000|100-1000>` inside a markdown list context (via `newline_to_br` / markdownify). Kramdown interprets the pipe `|` as a table column separator and renders this as `<table><tbody><tr><td>...|...</td></tr></tbody></table>` nested inside a `<li>`. Rustkyll's markdownify instead renders the `<tbody>` outside the `<table>` element, and the subsequent `- dataset:...` and `- release:...` lines leak as raw text instead of being rendered as `<li>` elements.

### What Jekyll Produces

In the Jekyll output (lines 536-550 of the built page), the structure is:

```html
<ul>
  <li>
    <table>
    <tbody>
    <tr>
    <td>engineering: ... &lt;tel:100-1000</td>
    <td>100-1000&gt;s of GPUs. ...</td>
    </tr>
    </tbody>
    </table>
  </li>
  <li>dataset: training these beadts requires ...</li>
  <li>release: how can one responsibly ...</li>
</ul>
```

The source text (from the YAML archive comment) is:

```
- engineering: ... <tel:100-1000|100-1000>s of GPUs. ...
- dataset: training these beadts requires ...
- release: how can one responsibly ...
```

Kramdown sees the `|` in the first line as a table separator and wraps it in `<table><tbody><tr><td>`. The remaining `- ` lines become normal `<li>` items.

### Root Cause

The markdownify filter pipeline in `src/frontmatter.rs` (`markdown_to_html_for_filter`) preprocesses the markdown through several transforms before feeding it to pulldown-cmark. The issue is that the pipe character inside `<tel:100-1000|100-1000>` is not being recognized as a kramdown table pattern in the list context. Kramdown's pipe-table detection treats any line with `|` as a potential single-row table. The `convert_kramdown_pipe_tables` function in `src/kramdown.rs` may not handle this case when the pipe is inside angle brackets within a list item.

The specific problems:
1. **Table not formed**: The `<tel:...|...>` line is not being converted to `<table><tbody><tr><td>` structure
2. **List items leaking**: The subsequent `- dataset:` and `- release:` lines are not rendered as `<li>` elements, suggesting the list context is broken by the malformed table attempt

### Files to Investigate

- `src/kramdown.rs` -- `convert_kramdown_pipe_tables()` function (line ~2585)
- `src/frontmatter.rs` -- `markdown_to_html_for_filter()` pipeline (line ~970)
- `src/template/filters/markdownify.rs` -- the filter entry point

## Affected Pages

- `books/20220425-natural-language-processing-with-transformers.html` -- 7 diffs related to this issue (table/tbody placement and raw list syntax leaking). Note: the current DOM diff report for this page shows 17 total diffs, but 10 of those are `missing_attribute` for `class='highlighter-rouge language-plaintext'` on `<code>` elements, which is a separate issue.

## Dependencies

None. Self-contained change to the markdownify/kramdown preprocessing pipeline.

## Acceptance Criteria

- [ ] When markdownify processes text containing `<tel:100-1000|100-1000>` inside a list context, the pipe is treated as a table column separator and rendered as `<table><tbody><tr><td>...</td><td>...</td></tr></tbody></table>` inside the `<li>`, matching kramdown behavior
- [ ] `<tbody>` is always nested inside `<table>`, never rendered outside it
- [ ] Subsequent `- text` lines after the table line render as `<li>` elements, not as raw text
- [ ] The fix is generic: any pipe `|` in a list-item line that looks like a single-row kramdown table should be handled, not just `<tel:...>` specifically
- [ ] No site-specific hardcoding
- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` produces no changes
- [ ] `cargo test` passes with no regressions
- [ ] DTC DOM baseline: 790/790 -- match count must not drop below this

## Test Scenarios

### Unit: pipe-as-table in list context

- Input to markdownify: `"- engineering: training <tel:100-1000|100-1000>s of GPUs\n- dataset: training requires data\n- release: how to release"`
- Expected: output contains `<table>` with `<tbody>` inside it, and separate `<li>` elements for "dataset:" and "release:" lines
- Verify `<tbody>` is a child of `<table>`, not a sibling

### Unit: normal list without pipe

- Input: `"- item one\n- item two\n- item three"`
- Expected: normal `<ul><li>` rendering, no `<table>` elements
- Verify no regression for lists without pipes

### Unit: pipe in non-list context

- Input: `"text with <tel:100-1000|100-1000> inline"`
- Expected: kramdown-style table rendering (or passthrough depending on context), verify `<tbody>` is inside `<table>` if a table is created

### Unit: Unicode content in table cells

- Input: `"- Beschreibung: <info:deutsch|Ubersetzung>s Modell"`
- Expected: correct table/list rendering with non-ASCII content

### Integration: DTC output verification

- Build the DTC site
- Inspect `books/20220425-natural-language-processing-with-transformers.html`
- Verify the `<table><tbody>` structure is correctly nested inside the `<li>`
- Verify `dataset:` and `release:` lines appear as `<li>` elements
- Run DOM comparison, verify 790/790 is maintained

## DOM Baseline

- Current: 790/790 matched
- Expected after fix: 790/790 matched (this page currently matches; the 7 original diffs from RC-E may have been resolved by prior fixes or may be counted differently now)

## Priority

LOW

## Log

### [SWE] 2026-03-30

- Investigated the issue: the `<tel:100-1000|100-1000>` pipe-as-table in list context is already handled correctly by the existing `RKPIPEMARK` mechanism in `escape_non_standard_autolink_schemes` + `convert_pipe_markers_to_tables`.
- Verified the actual DTC page output (`books/20220425-natural-language-processing-with-transformers.html`) matches Jekyll output exactly: `<table><tbody>` is correctly nested inside `<li>`, and "dataset:" / "release:" lines render as separate `<li>` elements.
- The issue was resolved by prior fixes (likely issue 388 which added the RKPIPEMARK mechanism).
- Added 5 regression tests in `src/frontmatter.rs`:
  1. `test_issue366_pipe_in_list_item_produces_table_inside_li` - pipe in list produces table inside li
  2. `test_issue366_pipe_in_list_after_newline_to_br` - simulates actual DTC pipeline (newline_to_br then markdownify)
  3. `test_issue366_normal_list_without_pipe_no_table` - no regression for normal lists
  4. `test_issue366_pipe_in_non_list_context_table` - tbody inside table in non-list context
  5. `test_issue366_unicode_content_in_table_cells` - Unicode content preserved
- All 5 tests pass, all existing tests pass (full suite green)
- Clippy clean, fmt clean
- DTC DOM baseline: 788/790 (2 files with differences are from other in-progress issues #348 and #349, not related to this issue)
- Files modified: `src/frontmatter.rs` (tests only, no implementation changes needed)
