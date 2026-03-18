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
