# Issue 204: Fix extra HTML elements in rustkyll output (90 pages)

## Checklist Category

**Extra HTML elements in rustkyll output** -- 90 pages

## Problem

90 pages have extra HTML elements not present in Jekyll's output. rustkyll generates additional wrapper elements.

Breakdown by site:
- alexeygrigorev-mlwiki.org (56): Extra `<p>`, `<ul>`, `<h3>`, `<h4>` elements from markdown rendering
- DTC (17): Extra `<p>` elements inside `<li>` items (loose vs tight list rendering), extra `<figcaption>` and `<canvas>` elements
- muan-blog (16): Extra elements from layout/structural differences
- alexeygrigorev-mlbookcamp-page (1): Extra element

## Goal

Remove extra wrapper elements to match Jekyll output.

## Dependencies

- Issue 124 (kramdown loose list wrapping) -- done. Previous fix for `<p>` inside `<li>`.
- Issue 196 (layout not applied) -- muan-blog's 16 extra element diffs may be caused by layout issues.

## Sub-tasks

### Sub-task 1: Investigation

1. From DTC dom-details, the extra `<p>` inside `<li>` pattern is clear:
   ```
   ul > li: expected_text_got_element - expected: 'Then you should...', actual: '<p>'
   ul > li > p: extra_element - expected: '(none)', actual: '<p>'
   ```
   This means rustkyll wraps list item text in `<p>` (loose list) when kramdown keeps it as direct text (tight list). Count how many DTC pages have this pattern.

2. The `<canvas>` / `<figcaption>` ordering in the `how-do-professionals-use-llm-tools-and-frameworks.html` page is a different issue -- figure children appearing in different order.

3. From mlwiki.org, check if the 56 extra elements follow the same loose-list pattern or are different.

4. From muan-blog, check if the 16 extra elements are from pages with missing layout (issue 196).

### Sub-task 2: Fix loose vs tight list detection

kramdown's list rendering rules differ from pulldown-cmark's CommonMark rules. In kramdown, a list is "tight" (no `<p>` wrapping) unless items are separated by blank lines AND contain multiple paragraphs. The heuristic in `src/kramdown.rs` may need refinement.

### Sub-task 3: Fix figure/figcaption ordering

If `<figcaption>` appears after `<canvas>` in rustkyll but before it in Jekyll, the template or markdown rendering order needs adjustment.

## TDD Test Scenarios

### Test 1: Tight list items have no `<p>` wrapper (write FIRST, verify it fails)

```rust
#[test]
fn test_tight_list_no_paragraph_wrapper() {
    // Setup: Markdown list (tight -- no blank lines between items):
    //   - First item text
    //   - Second item text
    //   - Third item with longer text here
    //
    // Assert: Produces <ul><li>First item text</li><li>Second item text</li>...
    //   WITHOUT <p> wrapping inside <li>.
    //   Jekyll/kramdown output: <li>text directly</li>
    //   NOT: <li><p>text</p></li>
    //
    // Verify it FAILS if rustkyll adds <p> inside <li>.
}
```

### Test 2: List with sub-paragraphs stays tight per kramdown rules

```rust
#[test]
fn test_kramdown_tight_list_with_continuation() {
    // Setup: kramdown-style list where items have continuation lines
    //   but are still considered "tight":
    //   - Then you should use several platforms to show yourself.
    //     For example, after an achievement writes on LinkedIn.
    //   - You must connect to recruiters or professionals.
    //
    // Assert: No <p> wrapping inside <li> -- kramdown treats this as tight.
    //
    // Verify it FAILS before implementing.
}
```

### Test 3: Figure element child ordering

```rust
#[test]
fn test_figure_children_ordering() {
    // Setup: HTML with <figure> containing both <canvas> and <figcaption>:
    //   <figure>
    //     <canvas class="ai-chart" data-type="bar">...</canvas>
    //     <figcaption>Chart description</figcaption>
    //   </figure>
    //
    // Assert: In output, <canvas> appears first, then <figcaption>.
    //   Not the reverse.
    //
    // Verify it FAILS if ordering is wrong.
}
```

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with list tightness and extra element tests
- [ ] Investigation documents categorization of all 90 extra element diffs
- [ ] Tight lists do not have extra `<p>` wrappers inside `<li>` elements where kramdown keeps them tight
- [ ] DTC extra `<p>` in list items fixed (17 pages)
- [ ] Figure/figcaption ordering fixed for DTC chart pages
- [ ] muan-blog extra elements: document overlap with issue 196 (layout)
- [ ] No regressions in existing kramdown list tests

## Log

### [SWE] 2026-03-18

**Investigation:**
- DTC (17 pages): Extra `<p>` inside `<li>` from loose vs tight list mismatch, extra figcaption/canvas elements
- mlwiki.org (56+ pages per issue, found 164 pages): Two root causes:
  1. Partially loose lists: blank lines between some (but not all) list items cause CommonMark to make entire list loose (all items get `<p>`), while kramdown only wraps items before blank lines in `<p>`
  2. Headings inside list context: kramdown treats `#### heading` directly after a list item (no blank line) as text, while pulldown-cmark treats it as a heading element, breaking the list and creating extra `<h3>`, `<h4>`, `<ul>` elements
- muan-blog (16 pages): Remaining 10 extra `<p>` tags are from structural/layout differences, likely overlap with issue 196 (layout not applied)
- mlbookcamp-page (1 page): No Jekyll reference output available to compare

**Root causes fixed:**
1. Partially loose lists: Added `collapse_blank_lines_between_list_items()` pre-processing that removes blank lines between list items only in "partially loose" lists (where not all consecutive items have blank lines). Fully loose lists (ALL items separated by blanks) keep their blank lines since kramdown also wraps all items in `<p>` for those.
2. Headings in list context: Added `escape_headings_in_list_context()` pre-processing that escapes `#` heading markers appearing immediately after list items (no blank line between), matching kramdown behavior where they are treated as text.

**Results:**
- DTC: 0 pages with extra elements (was 17 per issue, 0 found even before fix)
- mlwiki.org: 122 pages with extra elements (down from 164 before fix, 42 pages fixed)
- muan-blog: 10 pages with extra `<p>` (structural/layout issues, overlap with issue 196)
- Figure/figcaption ordering: No issues found in DTC site (pages match Jekyll perfectly)
- Remaining mlwiki diffs are mostly from kramdown-specific behaviors (headings requiring blank line before them in all contexts, definition list syntax, etc.) that are hard to replicate in CommonMark

**Tests added:** 8 tests in kramdown.rs
- `test_issue204_tight_list_no_p_wrapper`
- `test_issue204_kramdown_tight_list_with_continuation`
- `test_issue204_kramdown_per_item_loose_tight`
- `test_issue204_heading_after_list_item_no_blank_line`
- `test_issue204_collapse_blank_lines_between_list_items`
- `test_issue204_collapse_preserves_blank_after_list`
- `test_issue204_escape_headings_in_list`
- `test_issue204_heading_after_blank_line_not_escaped`

**Build:** 1523+ tests pass, 0 fail, clippy clean, fmt clean
**Files modified:** src/kramdown.rs, src/frontmatter.rs
