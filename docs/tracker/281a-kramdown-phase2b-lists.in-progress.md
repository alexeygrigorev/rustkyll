# Issue 281a: Kramdown parser Phase 2b - Lists (ordered, unordered, nested)

## Problem

Lists are the most complex block element in kramdown. They require detecting list markers, calculating indentation for nesting, handling lazy continuation lines, determining when items get paragraph wrapping (compact vs loose), and supporting block content inside items (blockquotes, code blocks, headers, nested lists).

## Scope

Implement ordered and unordered list parsing and HTML rendering. This covers:

- **Unordered lists**: `*`, `+`, `-` markers
- **Ordered lists**: `1.`, `10.` etc. markers (with tabs and spaces)
- **Nesting**: indentation-based nesting of lists within lists
- **Compact vs loose**: items separated by blank lines get `<p>` wrapping
- **Lazy continuation**: lines not indented to the list item level that continue the item
- **Block content in items**: blockquotes, code blocks, headers, nested lists inside items
- **List item IAL**: `{:.cls}` at start of item applies to the `<li>`
- **Edge cases**: empty items, single items, escaping (`\-`, `1984\.`), mixed marker types, list vs HR disambiguation, EOB markers between lists
- **nomarkdown extension in items**: `{::nomarkdown}` inside list items

## Dependencies

- Issue #280 (Phase 2a) must be `.done.md` -- provides paragraph, header, blockquote, code block, HR, blank line, EOB, basic IAL parsing

## Test Cases to Pass

All `.text`/`.html` pairs in `block/08_list/` that have corresponding `.html` files:

| Test file | What it tests |
|-----------|---------------|
| `single_item` | Minimal case: one-item list |
| `simple_ul` | Basic unordered list, para wrapping logic, multi-line items, indentation handling |
| `simple_ol` | Ordered list with varying numbers, tabs, para wrapping |
| `nested` | Two-level nesting with `^` EOB separator |
| `lazy` | Lazy continuation lines (not indented to item level) |
| `lazy_and_nested` | Lazy continuation interacting with nesting (tricky: some lines look nested but are lazy) |
| `mixed` | Mixed `*`/`+`/`-` markers, tabs and spaces, nested ordered/unordered, compact vs loose |
| `special_cases` | Blockquotes inside items, paragraph continuation rules, compact vs normal lists, empty items, items without content |
| `escaping` | `\-` and `1984\.` escapes prevent list detection, paragraph continuation rules |
| `list_and_hr` | `* * *` is HR not list, disambiguating list marker from HR |
| `list_and_others` | Lists followed by blockquotes, list items containing blockquotes/headers/code blocks |
| `other_first_element` | First element in item is code block, blockquote, header, or nested list |
| `item_ial` | `{:.cls}` IAL at start of item, nomarkdown extension in items |

**Note:** `brackets_in_item` and `nested_compact` are skipped (no `.html` reference files exist).

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes all existing Phase 2a tests (no regressions)
- [ ] Conformance tests pass for all 13 list test cases listed above
- [ ] Unordered lists render with `<ul>` / `<li>`, ordered with `<ol>` / `<li>`
- [ ] Compact lists (no blank lines between items) render items without `<p>` wrapping
- [ ] Loose lists (blank lines between items) wrap item content in `<p>` tags
- [ ] Nested lists produce nested `<ul>`/`<ol>` inside parent `<li>`
- [ ] Lazy continuation lines (insufficient indentation) are correctly included in the current item
- [ ] Block content inside items (blockquotes, code blocks, headers) renders correctly
- [ ] List item IAL (`{:.cls}` at start of item) applies attributes to the `<li>` element
- [ ] Escaped list markers (`\-`, `1984\.`) do not start lists
- [ ] `* * *` is parsed as HR, not a list item
- [ ] EOB marker (`^`) between lists creates separate lists
- [ ] Empty list items (`* ` followed by newline) produce `<li></li>`

## Test Scenarios

### Unit: List detection
- Line starting with `* ` detected as unordered list marker
- Line starting with `1. ` detected as ordered list marker
- Line starting with `\- ` is NOT a list marker (escaped)
- Line `* * *` is HR, not list
- Line `1984. text` in middle of paragraph is NOT a list start

### Unit: Indentation calculation
- `* item` has content indent of 2
- `*   item` has content indent of 4
- `10. item` has content indent of 4
- Tab after marker expands correctly

### Unit: Compact vs loose detection
- Items with no blank lines between them: compact (no `<p>`)
- Items with blank lines between them: loose (wrap in `<p>`)
- Mixed: if any item has a blank line before it, the whole list is loose

### Integration: Full list rendering
- Parse `simple_ul.text`, compare HTML output to `simple_ul.html`
- Parse `nested.text`, verify nested `<ul>` inside `<li>`
- Parse `mixed.text`, verify all combinations of markers and nesting
- Parse `special_cases.text`, verify blockquotes-in-items, empty items, compact detection
- Parse `item_ial.text`, verify class attributes on `<li>` elements

### Integration: Lists with other blocks
- Parse `list_and_others.text`, verify list items containing blockquotes, headers, code blocks
- Parse `list_and_hr.text`, verify HR between two separate lists
- Parse `other_first_element.text`, verify code block / blockquote / header as first element in item

## Log

### [SWE] 2026-03-20
- Started implementation. Found compilation error: `parse_paragraph_in_list_context_with_lazy` was called but not defined.
- Wrote `parse_paragraph_in_list_context_with_lazy` function (TDD: tests failed without it, compiled and ran after).
- Ran tests: 10/13 list tests passed. 3 failures: lazy, lazy_and_nested, other_first_element.
- Fixed `lazy` test: lazy continuation lines must preserve original indentation (not stripped to content_indent).
- Fixed `other_first_element` test: when first line of a list item is empty, use content_indent of at least 4 for correct code block detection.
- Fixed `lazy_and_nested` test: tracked `nested_list_found` per list item; when a lazy continuation line looks like a list marker after a nested list was detected, pad it to prevent it from being parsed as a new list marker in nested context (matching kramdown Ruby behavior).
- All 13 list conformance tests pass.
- No regressions in other tests (2176 passed, 116 pre-existing failures in unimplemented HTML block/span parsing).
- Clippy clean (only pre-existing vendor warnings). Format clean.
- Files modified: src/kramdown_parser/parser.rs (added parse_paragraph_in_list_context_with_lazy, fixed lazy line handling, fixed empty-first-line content_indent, added nested_list_found tracking with lazy line padding)
