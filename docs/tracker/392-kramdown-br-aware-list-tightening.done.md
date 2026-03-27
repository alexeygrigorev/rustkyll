# Issue 392: Add br-aware list tightening to kramdown parser

## Problem

The kramdown parser produces loose lists (content wrapped in `<p>`) by default.
For the markdownify pipeline where input has `<br />` tags from `newline_to_br`,
lists should be tight (no `<p>` wrapping) to match Jekyll output.

When kramdown parses `- item one\n- item two\n`, each list item's content is a
`Paragraph` child element. The HTML converter (`src/kramdown_parser/html.rs`)
renders `Paragraph` elements as `<p>...</p>`, producing loose lists:

```html
<ul>
  <li>
    <p>item one</p>
  </li>
</ul>
```

For the `newline_to_br | markdownify` pipeline, the expected output is tight:

```html
<ul>
  <li>item one</li>
</ul>
```

## Scope

1. Add a boolean option `tight_lists` to `Options` (`src/kramdown_parser/options.rs`), defaulting to `false`
2. When `tight_lists` is `true`, the HTML converter must unwrap single-`Paragraph` list items -- rendering the paragraph's text content directly inside `<li>` without `<p>` tags
3. Multi-paragraph list items (2+ `Paragraph` children) should still render with `<p>` tags even when `tight_lists` is true (they are genuinely loose)
4. Non-paragraph block children (blockquote, code block, nested list, etc.) are unaffected by this option
5. This is a new opt-in mode only -- default behavior (`tight_lists: false`) must not change at all
6. The option is NOT wired into markdownify yet (that is #390); this issue only adds and tests the option

## Implementation Notes

The key function is `convert_list_item` at `src/kramdown_parser/html.rs:975`. Currently when
`has_block_children` is true and the first child is a `Paragraph`, the paragraph is rendered
with `<p>` tags via `convert_element -> convert_paragraph`.

The fix should:
- Pass the `tight_lists` flag through to `convert_list_item` (it already receives `options: &Options`)
- When `tight_lists` is true and a list item has exactly one `Paragraph` child (possibly followed by `Blank`/`Eob`), render the paragraph's text content inline (like the existing simple-item path) instead of as a `<p>` block
- The `Options` struct already has `tight_lists: false` as the default, so parsing from `.options` files should also handle the `tight_lists` key

## Dependencies

- Prerequisite for #390 (kramdown parser in markdownify)
- No blocking dependencies -- can be implemented immediately

## DTC DOM Baseline

787/790 -- must not change. This issue adds a new option that defaults to `false`, so existing rendering is completely unaffected.

## Acceptance Criteria

- [ ] `Options` struct has a `tight_lists: bool` field, defaulting to `false`
- [ ] `Options::parse_options_str` recognizes `tight_lists: true` and `tight_lists: false`
- [ ] With `tight_lists: false` (default), list rendering is identical to current behavior -- no output changes
- [ ] With `tight_lists: true`, a list item containing a single `Paragraph` child renders as `<li>content</li>` (no `<p>` wrapping)
- [ ] With `tight_lists: true`, a list item containing 2+ `Paragraph` children still renders each paragraph with `<p>` tags (genuinely loose)
- [ ] With `tight_lists: true`, list items with non-paragraph block children (code blocks, blockquotes, nested lists) render the same as without the option
- [ ] Ordered lists (`<ol>`) respect `tight_lists` the same way as unordered lists (`<ul>`)
- [ ] The `to_html_with_options` public API works correctly with `tight_lists: true`
- [ ] `cargo build` compiles without errors
- [ ] `cargo fmt` produces no changes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `./scripts/cargo-safe test` passes with all existing tests plus new tests
- [ ] DTC DOM baseline remains 787/790 (no regression from default behavior)

## Test Scenarios

### Unit: Options parsing
- Parse `tight_lists: true` from options string, verify `opts.tight_lists == true`
- Parse `tight_lists: false` from options string, verify `opts.tight_lists == false`
- Default options have `tight_lists == false`

### Unit: Tight list rendering (tight_lists: true)
- Single-item unordered list: `- hello` produces `<ul>\n  <li>hello</li>\n</ul>\n` (no `<p>`)
- Multi-item unordered list: `- a\n- b\n- c` produces three `<li>` without `<p>` wrapping
- Single-item ordered list: `1. hello` produces `<ol>\n  <li>hello</li>\n</ol>\n` (no `<p>`)
- List item with inline formatting: `- **bold** and *italic*` renders emphasis inside `<li>` without `<p>`
- List item with `<br />` in content: `- line1<br />line2` renders the br inside `<li>` without `<p>`

### Unit: Loose list items preserved (tight_lists: true)
- List item with two paragraphs (blank line between items creating loose list with multiple paragraphs per item) still renders `<p>` tags
- List item containing a code block still renders normally
- List item containing a blockquote still renders normally
- List item containing a nested list still renders the nested list normally

### Unit: Default behavior unchanged (tight_lists: false)
- Same inputs as tight-list tests above, but with default options -- verify `<p>` wrapping is present as before
- Compare output of default options against a known-good baseline string

### Integration: Public API
- Call `kramdown_parser::to_html_with_options(input, &opts)` with `tight_lists: true`, verify tight output
- Call `kramdown_parser::to_html(input)` (default), verify loose output unchanged

## Log

### [SWE] 2026-03-27
- Read issue, understood scope: add `tight_lists: bool` option (default false) to kramdown Options
- TDD: wrote 15 tests first covering all acceptance criteria scenarios
- Tests failed to compile (field does not exist) -- confirmed TDD red phase
- Added `tight_lists: bool` field to `Options` struct with default `false`
- Added `tight_lists` key parsing in `parse_options_str`
- Added `is_single_paragraph_item` helper in html.rs to detect single-Paragraph list items
- Added tight-list rendering path in `convert_list_item`: when `tight_lists: true` and item has exactly one Paragraph child (ignoring Blank/Eob), render paragraph content inline without `<p>` wrapper
- Adjusted 2 default-behavior tests: simple `- item\n` already renders without `<p>` in kramdown (no Paragraph child in AST); tight_lists matters for blank-line-separated items which DO create Paragraph children
- All 15 tight_lists tests pass
- Full test suite: 3326 passed, 0 failed, 2 ignored
- `cargo fmt --check`: clean
- `cargo clippy -- -D warnings`: clean
- Files modified:
  - `src/kramdown_parser/options.rs` -- added `tight_lists` field + parsing
  - `src/kramdown_parser/html.rs` -- added `is_single_paragraph_item` helper + tight rendering path in `convert_list_item`
  - `src/kramdown_parser/tests.rs` -- added 15 tests for tight_lists feature

### [QA] 2026-03-27
- Both issues 391 and 392 verified together (share tests.rs file)
- `./scripts/cargo-safe test`: all tests pass (0 failures)
- `./scripts/cargo-safe clippy -- -D warnings`: clean (only upstream lint rename warnings)
- `cargo fmt --check`: clean
- DOM baseline: 787/790 (matches required baseline exactly)
- Acceptance criteria:
  - Options struct has `tight_lists: bool` field, default false: PASS
  - `parse_options_str` recognizes `tight_lists: true` and `tight_lists: false`: PASS (2 tests)
  - Default behavior unchanged (tight_lists: false): PASS (test_tight_lists_default_unchanged, test_tight_lists_default_multi_item_unchanged)
  - Single-paragraph list items render tight with option enabled: PASS (test_tight_lists_single_item_ul, test_tight_lists_multi_item_ul)
  - Multi-paragraph items stay loose even with tight_lists: true: PASS (test_tight_lists_multi_paragraph_stays_loose)
  - Non-paragraph block children unaffected: PASS (test_tight_lists_code_block_unaffected, test_tight_lists_nested_list_unaffected)
  - Ordered lists respect tight_lists: PASS (test_tight_lists_single_item_ol)
  - `to_html_with_options` API works correctly: PASS (all tight_lists tests use it)
  - cargo build: PASS
  - cargo fmt: PASS
  - cargo clippy: PASS
  - All tests pass: PASS
  - DTC DOM baseline 787/790: PASS
- 15 issue-392-specific tests all pass, covering option parsing, tight rendering (ul/ol), inline formatting, br content, loose multi-paragraph, code blocks, nested lists, unicode
- Code quality: `is_single_paragraph_item` helper is clean and well-scoped; tight rendering path in `convert_list_item` correctly delegates to span_parser for inline content
- VERDICT: **PASS**

### [PM] 2026-03-27 -- Final Acceptance Review

Reviewed the code diff and QA report for issue 392.

**Code review summary:**
- `options.rs`: `tight_lists: bool` field added to `Options` struct with default `false`. Parsing added in `parse_options_str`. Clean.
- `html.rs`: New `is_single_paragraph_item` helper checks if a list item has exactly one `Paragraph` child (ignoring `Blank`/`Eob`). In `convert_list_item`, when `tight_lists` is true and the item is a single-paragraph item, the paragraph content is rendered inline via `span_parser::spans_to_html` without `<p>` wrapper. Multi-paragraph items and items with non-paragraph block children fall through to the existing code path. Logic is well-scoped and correct.
- `tests.rs`: 15 tests covering option parsing (true/false/default), tight rendering for ul and ol, inline formatting preservation, br content, multi-paragraph items staying loose, code blocks and nested lists unaffected, default behavior unchanged, blank-separated items, and unicode content.

**Acceptance criteria verification:**
- [x] `Options` struct has `tight_lists: bool`, default false
- [x] `parse_options_str` recognizes `tight_lists: true` and `tight_lists: false`
- [x] Default behavior (tight_lists: false) unchanged -- loose list rendering preserved
- [x] tight_lists true: single-paragraph items render as `<li>content</li>` without `<p>`
- [x] tight_lists true: multi-paragraph items still render with `<p>` tags
- [x] tight_lists true: non-paragraph block children (code blocks, nested lists) unaffected
- [x] Ordered lists respect tight_lists the same as unordered
- [x] `to_html_with_options` API works correctly with tight_lists
- [x] cargo build: PASS
- [x] cargo fmt: PASS
- [x] cargo clippy: PASS
- [x] All tests pass (including 15 new)
- [x] DTC DOM baseline 787/790: unchanged

**Tests are meaningful:** Tests validate actual HTML output structure, not just compilation. Edge cases (multi-paragraph loose, nested lists, code blocks, unicode) are well covered. The `test_tight_lists_default_multi_item_unchanged` test correctly verifies that simple items (no Paragraph AST children) produce identical output regardless of the option.

**No descoping:** All acceptance criteria from the groomed spec are met. No items dropped.

VERDICT: **ACCEPT**
