# Issue 201: Fix text node splitting differences (138 pages)

## Checklist Category

**Text node splitting differences** -- 138 pages

## Problem

138 pages have text split differently across child text nodes. Text after `<br>` tags appears as a child of the `<br>` instead of as a sibling text node of the parent element.

Breakdown by site:
- alexeygrigorev-mlwiki.org (114): Various text node boundary differences
- DTC (22): Text after `<br>` tags in newsletter signup sections
- alexeygrigorev-mlbookcamp-page (1): Text node difference
- mojombo-blog (1): Text node difference

## Goal

Match Jekyll's text node placement in HTML output, especially for text after `<br>` tags.

## Dependencies

None directly. Partially overlaps with issue 198 (content text ordering) for the DTC pages.

## Sub-tasks

### Sub-task 1: Investigation

1. From the DTC dom-details, the pattern is clear: `br: extra_text` means text is being placed as content of the `<br>` element rather than as a sibling text node after it. Example:
   ```
   body > div > p > br: extra_text - expected: '(none)', actual: "We'll keep you informed..."
   body > div > p: missing_text - expected: "We'll keep you informed...", actual: '(none)'
   ```
   This means the text should be a text node inside the `<p>` AFTER the `<br>`, not inside the `<br>`.

2. Check how `src/kramdown.rs` handles `<br>` tags and whether the HTML serializer places trailing text correctly.

3. For mlwiki.org, check if the 114 diffs are the same `<br>` pattern or something different.

### Sub-task 2: Fix `<br>` text placement

The HTML output should produce `<p>text<br>more text</p>` where "more text" is a text node child of `<p>`, not a child of `<br>`. The `<br>` element should be self-closing with no children.

### Sub-task 3: Fix other text node boundary issues in mlwiki.org

If the mlwiki.org diffs have a different pattern, address those separately.

## TDD Test Scenarios

### Test 1: Text after br tag is sibling, not child (write FIRST, verify it fails)

```rust
#[test]
fn test_text_after_br_is_sibling_not_child() {
    // Setup: Markdown/HTML input:
    //   <p>Sign up for our newsletter.<br>
    //   We'll keep you informed about our events.</p>
    //
    // Assert: In the output HTML, parse the DOM and verify:
    //   - <p> has 3 children: text "Sign up...", <br/>, text "We'll keep..."
    //   - <br> has NO text children
    //   - The text "We'll keep..." is a direct child of <p>, not of <br>
    //
    // Verify it FAILS before implementing.
}
```

### Test 2: Multiple br tags in sequence

```rust
#[test]
fn test_multiple_br_tags_text_placement() {
    // Setup: HTML input:
    //   <p>Line 1<br>Line 2<br>Line 3</p>
    //
    // Assert: <p> has 5 children: text, br, text, br, text
    //   Each text is a sibling of <br>, not a child.
    //
    // Verify it FAILS before implementing.
}
```

### Test 3: Markdown-generated br from double space

```rust
#[test]
fn test_markdown_line_break_text_placement() {
    // Setup: Markdown with two trailing spaces (line break):
    //   "First line  \nSecond line"
    //
    // Assert: Renders as <p>First line<br>\nSecond line</p>
    //   with "Second line" as a text child of <p>, not <br>.
    //
    // Verify it FAILS before implementing.
}
```

### Test 4 (integration, #[ignore]): Build DTC and verify br text placement

```rust
#[test]
#[ignore]
fn test_dtc_br_text_placement() {
    // Build DTC site
    // Parse blog/ai-tools-for-personal-productivity.html
    // Find the newsletter signup <p> with <br>
    // Verify text after <br> is a sibling text node
}
```

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with text node placement tests
- [ ] Text after `<br>` tags appears as a sibling text node of the parent element, not as a child of `<br>`
- [ ] DTC newsletter signup sections render correctly (22 pages)
- [ ] mlwiki.org text node diffs investigated and fixed where the same `<br>` pattern applies
- [ ] No regressions in existing HTML output

## Log

### [SWE] 2026-03-18

**Investigation:**
- Root cause: Jekyll/kramdown normalizes all void elements to XHTML-style (`<br />`, `<hr />`).
  Rustkyll was passing bare `<br>` and `<hr>` tags through from raw HTML in markdown source unchanged.
- When BeautifulSoup's html.parser encounters bare `<br>` earlier in a document, it can misparse
  later `<br/>` tags, causing text nodes to be placed as children of `<br>` instead of siblings.
- The DTC `br: extra_text` diffs were caused by bare `<br>` tags in table cells (from markdown source)
  confusing BS4's parsing of the later `<br/>` in the subscribe section.
- The HTML output for the subscribe section was actually byte-identical between Jekyll and rustkyll,
  but the parsing context (bare `<br>` vs `<br />` earlier in the document) caused BS4 misinterpretation.

**Fix:**
- Added `normalize_bare_void_elements()` function that converts bare void element tags
  (`<br>`, `<hr>`, `<img ...>`, etc.) to XHTML-style (`<br />`, `<hr />`, `<img ... />`).
- Integrated into `postprocess()`, `postprocess_for_filter()`, and `normalize_html_output()`.
- Made `is_void_element()` available to non-test code (removed `#[cfg(test)]`).

**TDD:**
- Wrote 6 failing tests first, verified they failed, then implemented the fix.
- Tests: `test_bare_br_converted_to_xhtml_style`, `test_bare_br_multiple_in_sequence`,
  `test_br_self_closing_preserved`, `test_bare_br_via_markdown_to_html`,
  `test_bare_hr_converted_to_xhtml_style`, `test_normalize_html_output_converts_bare_br`.
- Updated `test_md_raw_html_passthrough` to expect `<img ... />` instead of `<img ...>`.

**Test results:** 1523 passed, 0 failed, clippy clean, fmt clean.

**Files modified:**
- `src/kramdown.rs` -- Added `normalize_bare_void_elements()`, removed `#[cfg(test)]` from `is_void_element()`, added 6 tests
- `src/frontmatter.rs` -- Updated `test_md_raw_html_passthrough` assertion
- `docs/tracker/201-fix-text-node-splitting.in-progress.md` -- This file

**Verification:**
- Built DTC site and confirmed zero `br: extra_text` diffs in DOM comparison.
- All bare `<br>` tags in output converted to `<br />` matching Jekyll format.
