# Issue 198: Fix content text/ordering differences (358 pages)

## Checklist Category

**Content text/ordering differences (collection sort, markdown)** -- 358 pages

## Problem

358 pages have text content or ordering differences.

Breakdown by site:
- alexeygrigorev-mlwiki.org (325): MediaWiki-style markup (`'''bold'''`, `''italic''`) not converted to HTML bold/italic -- text appears with raw quote marks instead
- DTC (24): Zero-width space (\u200b) handling around emphasis markers causes text to not be split into separate elements; text after `<br>` tags merges incorrectly
- mojombo-blog (4): Post content differences (smart quote handling)
- alexeygrigorev-kids-horror-stories-ru (2): Unicode curly quote normalization
- large-blog-3000 (1): Content ordering
- alexeygrigorev.github.io (1): Services page link ordering (collection sort)
- alexeygrigorev-mlbookcamp-page (1): Content text difference

## Goal

Match Jekyll's text output for all affected pages.

## Dependencies

None directly. Some overlap with issue 199 (markdown block structure) and issue 206 (inline formatting) where text differences are caused by structural differences.

## Sub-tasks

### Sub-task 1: Investigation (do this FIRST)

1. Read `docs/comparison/dom-details/alexeygrigorev-mlwiki.org.txt` and count how many diffs are the `'''bold'''` / `''italic''` pattern vs other text diffs. The mlwiki.org site was exported from MediaWiki and uses MediaWiki markup conventions inside markdown files.

2. Read `docs/comparison/dom-details/DataTalksClub-datatalksclub.github.io.txt` and extract the 24 text content diffs. Categorize:
   - Zero-width space around emphasis: how many pages?
   - Text after `<br>` merging: how many pages? (may overlap with issue 201)
   - Other text differences

3. Read `docs/comparison/dom-details/mojombo-blog.txt` for the 4 post content diffs.

4. Read `docs/comparison/dom-details/alexeygrigorev-kids-horror-stories-ru.txt` for quote normalization.

5. Document findings before writing code.

### Sub-task 2: Fix zero-width space handling around emphasis in DTC

Jekyll/kramdown correctly splits text at zero-width space boundaries when emphasis markers (`_word_`) appear after `\u200b`. Rustkyll's markdown parser does not recognize `\u200b` as a word boundary for emphasis.

### Sub-task 3: Fix MediaWiki-style bold/italic in mlwiki.org

The `'''bold'''` and `''italic''` patterns are MediaWiki conventions. Jekyll's kramdown converts these because it treats `'''` as bold markers. Determine if pulldown-cmark handles this or if post-processing in `src/kramdown.rs` is needed.

### Sub-task 4: Fix curly quote normalization

Jekyll normalizes certain Unicode curly quotes. Check if rustkyll does the same.

## TDD Test Scenarios

### Test 1: Zero-width space around emphasis (write FIRST, verify it fails)

```rust
#[test]
fn test_zero_width_space_emphasis_boundary() {
    // Setup: Markdown input:
    //   "connect with \u{200b}_everyone_. \u{200b} People laugh."
    //
    // Assert: Produces HTML with:
    //   "connect with \u{200b}" as first text node
    //   <em>everyone</em>
    //   ". \u{200b} People laugh." as trailing text
    //
    // The emphasis markers should be recognized even after \u{200b}.
    // Verify it FAILS before implementing.
}
```

### Test 2: MediaWiki triple-quote bold (write FIRST, verify it fails)

```rust
#[test]
fn test_mediawiki_triple_quote_bold() {
    // Setup: Markdown input:
    //   "This is '''bold text''' here"
    //
    // Assert: kramdown/Jekyll converts triple quotes to bold,
    //   producing: "This is <b>bold text</b> here"
    //   OR the text contains the literal quotes as Jekyll would render them.
    //
    // First investigate what Jekyll actually outputs for this pattern.
    // Verify it FAILS before implementing.
}
```

### Test 3: Smart quote handling in blog posts

```rust
#[test]
fn test_smart_quote_consistency() {
    // Setup: Markdown with curly quotes (U+2018, U+2019):
    //   "it\u{2019}s a test"
    //
    // Assert: Output matches Jekyll's handling -- quotes preserved as-is
    //   in HTML output, not double-encoded or normalized.
    //
    // Verify it FAILS before implementing if there's a mismatch.
}
```

### Test 4 (integration, #[ignore]): Build DTC and verify text content

```rust
#[test]
#[ignore]
fn test_dtc_text_content_matches() {
    // Build DTC site
    // Parse data-narrative.html
    // Find the paragraph with "connect with"
    // Assert it contains <em>everyone</em> properly
}
```

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with new tests for each text difference pattern
- [ ] Investigation documents categorization of all 358 page diffs by root cause
- [ ] DTC zero-width space + emphasis handling matches Jekyll (24 pages)
- [ ] mojombo-blog text diffs fixed (4 pages)
- [ ] kids-horror-stories-ru quote normalization fixed (2 pages)
- [ ] mlwiki.org MediaWiki-style markup: document what Jekyll actually does with `'''bold'''`; fix if feasible, create sub-issue if complex
- [ ] alexeygrigorev.github.io collection sort order matches Jekyll (1 page)
