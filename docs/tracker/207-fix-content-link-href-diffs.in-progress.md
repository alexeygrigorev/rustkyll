# Issue 207: Fix content link href differences (33 pages)

## Checklist Category

**Content link href differences** -- 33 pages

## Problem

33 pages have different link `href` values in page content.

Breakdown by site:
- government-github (10): Link href differences
- mojombo-blog (3): Link href differences
- choosealicense.com (2), dinky-theme (2), hacker-theme (2), leap-day-theme (2), merlot-theme (2), midnight-theme (2), opensource-guide (2), time-machine-theme (2): Link href differences
- alexeygrigorev-alexeygrigorev.github.io (1): Services page link ordering (collection sort affecting which link appears)
- alexeygrigorev-mlwiki.org (2): URL encoding of non-ASCII characters
- large-blog-3000 (1): Link difference

## Goal

Match Jekyll's link href generation exactly.

## Dependencies

- Issue 189 (permalink .html extension) -- in-progress. Some href diffs may be caused by the `.html` extension issue.

## Sub-tasks

### Sub-task 1: Investigation

1. Read dom-details for the theme sites (dinky, hacker, leap-day, merlot, midnight, slate, time-machine) -- they likely have the same pattern. Check if these are template-generated links that differ or content links.

2. Read government-github dom-details for the 10 link diffs. Categorize:
   - Are these GitHub API links?
   - Are these internal navigation links?
   - What specifically differs in the href value?

3. Read mojombo-blog dom-details for the 3 link diffs.

4. Check alexeygrigorev.github.io services.html -- this shows link ordering difference (`/services/consulting.html` vs `/services/devrel.html`), which is a collection sort issue, not a href generation issue.

5. Check mlwiki.org -- URL encoding of `]` as `%5D` vs literal `]` in href.

6. Check choosealicense.com and opensource-guide for their patterns.

### Sub-task 2: Fix URL encoding differences

rustkyll percent-encodes characters that Jekyll preserves. Fix the URL encoding to match Jekyll:
- `]` in URLs should be preserved as `]`, not encoded as `%5D`
- Non-ASCII characters in fragment IDs should be preserved
- Zero-width spaces in URLs should be handled consistently

### Sub-task 3: Fix collection sort affecting link order

The alexeygrigorev.github.io services page shows different link ordering because the collection is sorted differently. This is a collection sort issue.

### Sub-task 4: Fix theme site link patterns

If all theme sites have the same pattern, fix once.

## TDD Test Scenarios

### Test 1: URL with closing bracket preserved (write FIRST, verify it fails)

```rust
#[test]
fn test_url_bracket_not_encoded() {
    // Setup: Markdown link with ] in URL:
    //   [link](http://example.com/page.html])
    //
    // Assert: href contains literal "]", not "%5D":
    //   href="http://example.com/page.html]"
    //
    // Verify it FAILS if rustkyll encodes ] as %5D.
}
```

### Test 2: URL with `>` character preserved

```rust
#[test]
fn test_url_angle_bracket_not_encoded() {
    // Setup: From DTC books page -- URL ending with >:
    //   [link](https://learning.oreilly.com/library/view/mastering-python-for/9781098100872/>)
    //
    // Assert: href contains literal ">", not "%3E":
    //   href="https://learning.oreilly.com/.../9781098100872/>"
    //
    // Verify it FAILS if rustkyll encodes > as %3E.
}
```

### Test 3: Collection sort produces correct link order

```rust
#[test]
fn test_collection_sort_order_for_links() {
    // Setup: Create a collection with items that have different dates/titles.
    //   Render a page that iterates the collection and generates links.
    //
    // Assert: Links appear in the same order as Jekyll would sort the collection
    //   (by date descending, then by title).
    //
    // Verify it FAILS if collection sort differs.
}
```

### Test 4: Non-ASCII characters in href preserved

```rust
#[test]
fn test_non_ascii_href_preserved() {
    // Setup: Link with Cyrillic characters:
    //   [link](/page/название.html)
    //
    // Assert: href preserves Cyrillic: href="/page/название.html"
    //   NOT href="/page/%D0%BD%D0%B0%D0%B7%D0%B2%D0%B0%D0%BD%D0%B8%D0%B5.html"
    //
    // Verify it FAILS if rustkyll percent-encodes Cyrillic.
}
```

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with URL encoding and link generation tests
- [ ] Investigation documents each site's specific link href difference
- [ ] URL encoding matches Jekyll: special characters like `]`, `>` preserved where Jekyll preserves them
- [ ] Non-ASCII characters in hrefs preserved (not percent-encoded) where Jekyll preserves them
- [ ] Collection sort order matches Jekyll for link generation
- [ ] Theme site link diffs fixed
- [ ] government-github link diffs fixed (10 pages)
- [ ] No regressions in existing link generation

## Log

### [SWE] 2026-03-18

- Implemented `decode_pulldown_url_encoding()` in src/frontmatter.rs: post-processes HTML to decode percent-encoded characters in href/src attributes that pulldown-cmark encodes but Jekyll preserves
- Implemented `decode_url_for_jekyll_compat()`: decodes non-ASCII bytes (>0x7F, e.g. Cyrillic) and `]` (0x5D) back to literal UTF-8; preserves encoding for space (%20) and other ASCII chars
- Added `hex_val()` helper for hex digit conversion
- Function added to both `markdown_to_html()` and `markdown_to_html_for_filter()` pipelines
- Tests added: 5 tests (bracket decoding, Cyrillic decoding, space kept encoded, non-URL content preserved, full markdown-to-html non-ASCII URL)
- Build: 1604 lib tests pass, 0 fail. All integration tests pass. Clippy clean, fmt clean.
- Files modified: src/frontmatter.rs
- Note: Collection sort order (alexeygrigorev services page) and theme site link patterns require investigation of specific dom-details to determine root cause -- these may be separate issues beyond URL encoding
- Note: The `>` character encoding (test scenario 2 in issue) is not currently decoded because pulldown-cmark may not parse URLs ending with `>` as links at all
