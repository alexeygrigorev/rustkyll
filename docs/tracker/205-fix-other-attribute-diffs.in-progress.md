# Issue 205: Fix other attribute differences (85 pages)

## Checklist Category

**Other attribute differences** -- 85 pages

## Problem

85 pages have attribute differences not covered by other categories.

Breakdown by site:
- alexeygrigorev-mlwiki.org (48): Various attribute diffs (image alt text whitespace, heading IDs)
- alexeygrigorev-little-book-of-metals-ru (33): Cyrillic heading IDs produce `-1-------` instead of the Cyrillic slug `глава-1-введение---мир-металлов-вокруг-нас`
- alexeygrigorev-mlbookcamp-page (3): Attribute diffs
- mojombo-blog (1): Attribute diff

## Goal

Fix attribute generation to match Jekyll output, especially for non-ASCII heading IDs.

## Dependencies

- Issue 77 (fix slug generation spaces) -- done.

## Sub-tasks

### Sub-task 1: Investigation

1. The little-book-of-metals-ru pattern is clear from dom-details: heading IDs strip all Cyrillic characters, leaving only digits and dashes. Example:
   - Expected: `id='глава-1-введение---мир-металлов-вокруг-нас'`
   - Actual: `id='-1-------'`
   This means the slugify function strips non-ASCII characters instead of preserving them.

2. Check what slugify mode Jekyll uses. Jekyll has multiple modes: `default`, `pretty`, `raw`, `latin`, `none`. The Cyrillic preservation suggests `default` mode which keeps Unicode letters.

3. From mlwiki.org, categorize the 48 attribute diffs:
   - Heading ID diffs with non-ASCII?
   - Image alt text whitespace diffs?
   - Other attribute types?

4. From mlbookcamp-page and mojombo-blog, check the specific diffs.

### Sub-task 2: Fix slugify to preserve non-ASCII characters

The slugify function in rustkyll must preserve Unicode letters (Cyrillic, etc.) by default, matching Jekyll's `default` slugify mode. Currently it strips everything non-ASCII.

### Sub-task 3: Fix alt text whitespace normalization

If mlwiki.org diffs are about alt text whitespace, fix the whitespace handling in image alt attributes.

## TDD Test Scenarios

### Test 1: Cyrillic heading ID preserved (write FIRST, verify it fails)

```rust
#[test]
fn test_slugify_preserves_cyrillic() {
    // Setup: Heading text: "Глава 1: Введение — Мир металлов вокруг нас"
    //
    // Assert: Slug/ID produced is "глава-1-введение---мир-металлов-вокруг-нас"
    //   NOT "-1-------" (with Cyrillic stripped).
    //
    // Verify it FAILS before implementing.
}
```

### Test 2: Mixed ASCII and Cyrillic slugify

```rust
#[test]
fn test_slugify_mixed_ascii_cyrillic() {
    // Setup: Heading text: "Уникальные дары металлов"
    //
    // Assert: Slug is "уникальные-дары-металлов"
    //
    // Verify it FAILS before implementing.
}
```

### Test 3: Slugify with numbers and special characters

```rust
#[test]
fn test_slugify_cyrillic_with_numbers() {
    // Setup: "Глава 3: Бронзовый век — революция сплавов"
    //
    // Assert: "глава-3-бронзовый-век---революция-сплавов"
    //   Dashes from em-dash should become triple dashes (matching Jekyll).
}
```

### Test 4: Alt text whitespace handling

```rust
#[test]
fn test_image_alt_text_whitespace() {
    // Setup: Markdown image with multi-word alt text:
    //   ![Long alt text with spaces](image.png)
    //
    // Assert: alt attribute has normalized whitespace matching Jekyll.
    //
    // Investigate first what the specific mlwiki.org diff is.
}
```

### Test 5 (integration, #[ignore]): Build little-book-of-metals-ru and verify IDs

```rust
#[test]
#[ignore]
fn test_metals_book_cyrillic_heading_ids() {
    // Build little-book-of-metals-ru site
    // Parse a chapter page
    // Verify heading IDs contain Cyrillic characters
}
```

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with slugify and attribute tests
- [ ] Slugify preserves non-ASCII (Cyrillic, etc.) characters matching Jekyll's default mode
- [ ] little-book-of-metals-ru heading IDs correct (33 pages)
- [ ] mlwiki.org attribute diffs investigated and fixed (48 pages)
- [ ] mlbookcamp-page and mojombo-blog attribute diffs fixed (4 pages)
- [ ] No regressions in existing slugify/ID generation for ASCII content

## Log

### [SWE] 2026-03-18
- Root cause: `slugify()` in `src/kramdown.rs` used `ch.is_ascii_alphanumeric()` which strips all non-ASCII characters (Cyrillic, etc.), producing IDs like `-1-------` instead of `глава-1-введение---мир-металлов-вокруг-нас`
- Fix: Changed `ch.is_ascii_alphanumeric()` to `ch.is_alphanumeric()` which preserves Unicode letters (Cyrillic, CJK, etc.) matching kramdown's `\p{Word}` behavior
- TDD: Wrote 6 new Cyrillic/non-ASCII slugify tests first, verified they failed, then applied fix
- Tests added: `test_slugify_preserves_cyrillic`, `test_slugify_preserves_cyrillic_emdash`, `test_slugify_mixed_ascii_cyrillic`, `test_slugify_cyrillic_with_numbers`, `test_slugify_pure_cyrillic`, `test_slugify_cyrillic_not_stripped`
- All 17 slugify tests pass (11 existing + 6 new), full suite: 1624 passed, 0 failed
- Clippy clean, fmt clean
- Files modified: `src/kramdown.rs`
- Note: The alt text whitespace and other attribute diffs (mlwiki.org, mlbookcamp, mojombo) cannot be investigated without access to the dom-details comparison data. The core fix (Unicode-preserving slugify) addresses the 33 little-book-of-metals-ru pages and any other non-ASCII heading ID issues across all sites
