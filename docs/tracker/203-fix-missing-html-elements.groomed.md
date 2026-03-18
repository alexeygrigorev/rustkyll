# Issue 203: Fix missing HTML elements in rustkyll output (126 pages)

## Checklist Category

**Missing HTML elements in rustkyll output** -- 126 pages

## Problem

126 pages are missing HTML elements (`<p>`, `<a>`, `<span>`, `<div>`, `<script>`) that Jekyll includes.

Breakdown by site:
- alexeygrigorev-mlwiki.org (103): Missing `<p>`, `<a>`, `<span>` elements from markdown rendering differences
- DTC (17): Missing `<p>` tags, missing `<div>` containers, missing `<a>` links from unrendered markdown, missing `<script>` tags
- government-github (2): Missing elements
- jekyll-docs-docs (2): Missing elements
- opensource-guide (2): Missing elements

## Goal

Generate all expected HTML elements matching Jekyll output.

## Dependencies

- Issue 199 (markdown block structure) -- overlaps. Some missing `<p>` elements are caused by block structure differences.
- Issue 206 (markdown inline formatting) -- overlaps. Some missing `<a>` and `<em>` elements are caused by inline markdown not being processed.
- Issue 196 (layout not applied) -- some missing elements may be in pages where layout itself is missing.

## Sub-tasks

### Sub-task 1: Investigation

1. From DTC dom-details, categorize the 17 pages with missing elements:
   - Missing `<p>`: How many? Is the text present but not wrapped in `<p>`?
   - Missing `<a>`: How many? Are these from markdown links not being parsed?
   - Missing `<div>`: How many? Are these container divs from the layout/template?
   - Missing `<script>`: How many? (FAQ schema scripts?)
   - How many overlap with issues 199 and 206?

2. From mlwiki.org dom-details, sample 10-15 pages to determine the pattern:
   - Are these definition list elements that kramdown generates?
   - Are these link/emphasis elements from MediaWiki markup?
   - How many are unique patterns vs repeated patterns?

3. Check government-github, jekyll-docs, opensource-guide for their 2 pages each.

### Sub-task 2: Fix missing `<p>` wrapper elements

Content that appears as text should be wrapped in `<p>` elements where kramdown would wrap them.

### Sub-task 3: Fix missing `<a>` elements from markdown links

Markdown links that are not being parsed (likely due to kramdown attribute syntax `{:target="_blank"}`) should produce `<a>` elements.

### Sub-task 4: Fix missing container elements

Missing `<div>` and `<script>` elements from template rendering.

## TDD Test Scenarios

### Test 1: Content wrapped in `<p>` tags (write FIRST, verify it fails)

```rust
#[test]
fn test_content_wrapped_in_paragraph() {
    // Setup: Markdown content that Jekyll/kramdown wraps in <p>:
    //   Some text content.
    //
    //   More content after blank line.
    //
    // Assert: Both text blocks are wrapped in <p> tags.
    //
    // Verify it FAILS if the text appears without <p> wrapper.
}
```

### Test 2: Markdown link produces `<a>` element

```rust
#[test]
fn test_markdown_link_produces_anchor() {
    // Setup: Markdown with kramdown-style link:
    //   [Source](https://example.com){:target="_blank"}
    //
    // Assert: Produces <a href="https://example.com" target="_blank">Source</a>
    //   The link must be rendered, not left as raw markdown text.
    //
    // Verify it FAILS before implementing.
}
```

### Test 3: FAQ schema script tag present

```rust
#[test]
fn test_faq_schema_script_present() {
    // Setup: Page with FAQ layout that should generate a JSON-LD <script> tag.
    //
    // Assert: Output contains <script type="application/ld+json"> with FAQ schema.
    //
    // Verify it FAILS if the script tag is missing.
}
```

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with missing element tests
- [ ] Investigation documents categorization of all 126 missing elements by type and root cause
- [ ] Missing `<p>` elements fixed where content exists but is not wrapped
- [ ] Missing `<a>` elements fixed where markdown links are not being parsed
- [ ] DTC missing elements addressed (17 pages)
- [ ] Elements that are missing due to overlapping issues (199, 206, 196) are documented as such
- [ ] No regressions in existing HTML output
