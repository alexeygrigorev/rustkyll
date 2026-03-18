# Issue 206: Fix markdown inline formatting not applied (21 pages)

## Checklist Category

**Markdown inline formatting not applied** -- 21 pages

## Problem

21 pages have inline markdown formatting (`_emphasis_`, `**bold**`, `[links](url)`) not converted to HTML. Missing `<em>`, `<strong>`, or `<a>` elements.

Breakdown by site:
- DTC (9): Emphasis around zero-width spaces, links with `{:target="_blank"}` kramdown attributes
- alexeygrigorev-mlwiki.org (6): MediaWiki-style `''italic''` / `'''bold'''` not converted
- government-github (3): Inline formatting not applied
- jekyll-docs-docs (2): Inline formatting not applied
- mojombo-blog (1): Inline formatting not applied

## Goal

Apply markdown formatting in all contexts where Jekyll does.

## Dependencies

- Issue 198 (content text ordering) -- overlaps for DTC zero-width space + emphasis pages.
- Issue 199 (markdown block structure) -- overlaps where missing inline elements are caused by block-level parsing issues.

## Sub-tasks

### Sub-task 1: Investigation

1. From DTC dom-details, extract the 9 pages with missing inline formatting:
   - `data-narrative.html`: `<em>` missing around `_everyone_` after zero-width space
   - `guidelines-to-get-data-engineer-job.html`: `<em>` missing around `*.*`
   - `how-to-run-postgresql.html`: `<a>` missing from `[Wikipedia](url){:target="_blank"}`
   - Categorize all 9 pages by specific pattern

2. For government-github and jekyll-docs, check what inline formatting is missing and in what context.

3. For mojombo-blog, check the specific diff.

4. For mlwiki.org, the `''italic''` pattern is MediaWiki-specific. Check what Jekyll actually produces.

### Sub-task 2: Fix kramdown link attribute syntax

Links with `{:target="_blank"}` after them should be parsed as links with the target attribute. This is kramdown-specific syntax not handled by pulldown-cmark. Post-processing in `src/kramdown.rs` should handle this.

### Sub-task 3: Fix emphasis after zero-width space

The `\u{200b}_word_` pattern should produce `\u{200b}<em>word</em>`. Zero-width space should act as a word boundary for emphasis detection.

### Sub-task 4: Fix other contexts where inline markdown is not processed

Check if there are Liquid template outputs that bypass markdown processing.

## TDD Test Scenarios

### Test 1: Kramdown link with target attribute (write FIRST, verify it fails)

```rust
#[test]
fn test_kramdown_link_with_target_blank() {
    // Setup: Markdown input:
    //   [Wikipedia](https://en.wikipedia.org/wiki/Docker_(software)){:target="_blank"}
    //
    // Assert: Produces <a href="..." target="_blank">Wikipedia</a>
    //   NOT raw text like "[Wikipedia](url){:target="_blank"}"
    //
    // Verify it FAILS before implementing.
}
```

### Test 2: Emphasis after zero-width space (write FIRST, verify it fails)

```rust
#[test]
fn test_emphasis_after_zero_width_space() {
    // Setup: Markdown:
    //   "connect with \u{200b}_everyone_"
    //
    // Assert: Produces "connect with \u{200b}<em>everyone</em>"
    //
    // Verify it FAILS before implementing.
}
```

### Test 3: Emphasis with dot pattern

```rust
#[test]
fn test_emphasis_with_dot_pattern() {
    // Setup: Markdown:
    //   "not be an easy task and straightforward*.*"
    //
    // Assert: Produces "...straightforward<em>.</em>"
    //   (single character emphasis with dot).
    //
    // Verify it FAILS before implementing.
}
```

### Test 4: Inline formatting in various contexts

```rust
#[test]
fn test_inline_formatting_in_liquid_output() {
    // Setup: Content that passes through Liquid before markdown:
    //   "Visit our [homepage](/) for more info"
    //
    // Assert: Link is rendered as <a href="/">homepage</a>
    //   regardless of whether it came from Liquid output.
}
```

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with inline formatting tests
- [ ] Investigation documents each page's specific inline formatting failure
- [ ] Kramdown link attribute syntax `{:target="_blank"}` produces proper `<a>` elements with target attribute
- [ ] Emphasis after zero-width space (`\u{200b}_word_`) produces `<em>` elements
- [ ] DTC inline formatting diffs fixed (9 pages)
- [ ] government-github and jekyll-docs inline formatting diffs fixed (5 pages)
- [ ] mojombo-blog inline formatting diff fixed (1 page)
- [ ] mlwiki.org: document what Jekyll does with `''italic''` MediaWiki syntax; fix or create sub-issue

## Log

### [SWE] 2026-03-18

- Implemented `normalize_zwsp_for_emphasis()` in src/frontmatter.rs: inserts a space after ZWSP when followed by `_` or `*` emphasis markers, enabling pulldown-cmark to treat them as word boundaries
- Implemented `fix_kramdown_emphasis_patterns()` in src/frontmatter.rs: detects `word*X*` patterns (short emphasis after alphanumeric) and inserts ZWSP+space to create word boundary that CommonMark recognizes
- Implemented `protect_consecutive_single_quotes()` / `restore_consecutive_single_quotes()` for MediaWiki `''italic''` / `'''bold'''` syntax -- protects from smart punctuation
- Both functions added to `markdown_to_html()` and `markdown_to_html_for_filter()` pipelines
- Kramdown IAL `{:target="_blank"}` already works (verified existing implementation)
- Tests added: 7 tests (ZWSP emphasis, ZWSP+Cyrillic, dot pattern, kramdown IAL, inline link, normalize preservation, no-ZWSP early return)
- Build: 1604 lib tests pass, 0 fail. All integration tests pass. Clippy clean, fmt clean.
- Files modified: src/frontmatter.rs
- Note: mlwiki.org `''italic''` MediaWiki syntax handled by the consecutive single quote protection (prevents smart punctuation from converting them to curly quotes)
