# Issue 202: Fix JSON-LD other value differences (212 pages)

## Checklist Category

**JSON-LD other value differences** -- 212 pages

## Problem

212 pages have various JSON-LD field value differences.

Breakdown by site:
- DTC (202): description truncation logic, special character handling (`$` getting stripped or space removed), headline format, author description trailing whitespace/markdown
- Theme sites (1 each, ~10 total): JSON-LD value differences

## Goal

Match jekyll-seo-tag JSON-LD output for all field values.

## Dependencies

- Issue 184 (fix jekyll-seo-tag JSON-LD fields: @type, url, name) -- in-progress. Covers theme site JSON-LD structural diffs.
- Issue 185 (fix JSON-LD whitespace in markdownify) -- in-progress. Covers FAQ answer text whitespace and author description trailing newline/markdown.
- Issue 153 (fix JSON-LD remaining diffs) -- done.

**Important overlap**: Many of the 212 DTC diffs may be the same author description trailing `\n` diffs that issue 185 covers. After 185 is done, recount.

## Sub-tasks

### Sub-task 1: Investigation

1. From the DTC dom-details file, categorize the JSON-LD diffs:
   - Author description trailing `\n` -- how many? (likely overlaps with issue 185)
   - Author description markdown links not rendered -- how many? (also issue 185)
   - Description truncation at different point -- how many?
   - `$` character handling (space removed: `$500,000` vs `a$500,000`) -- how many?
   - Headline differences -- how many?
   - Other value differences

2. For theme sites, check if issue 184 already covers these diffs.

3. Document the exact count for each sub-type so we know what remains after issues 184 and 185 are resolved.

### Sub-task 2: Fix description truncation logic

jekyll-seo-tag truncates descriptions at 200 characters on a word boundary. Verify rustkyll's truncation matches:
- Same character limit (200)
- Same word-boundary behavior (don't cut mid-word)
- Same ellipsis handling (`...` suffix)

### Sub-task 3: Fix `$` character handling in JSON-LD

The sample diff shows `a $500,000` vs `a$500,000` -- a space before `$` is being stripped. This may be in the description extraction or JSON encoding logic.

### Sub-task 4: Fix headline generation

If headline field differs from Jekyll's output, fix the generation logic in `src/jsonld.rs`.

## TDD Test Scenarios

### Test 1: Description truncation at 200 chars on word boundary (write FIRST, verify it fails)

```rust
#[test]
fn test_jsonld_description_truncation_word_boundary() {
    // Setup: Create a page with a long description (>200 chars):
    //   "Dan started his data science career by finishing 2nd (out of 1353 teams)
    //    in a kaggle competition with a $500,000 grand prize. Since then, he's
    //    worked as a data scientist at Google and was Product Director at Consumers
    //    Energy. He is now doing freelance data science."
    //
    // Assert: JSON-LD description is truncated at 200 chars on word boundary
    //   with "..." appended. Must match Jekyll's exact truncation point.
    //   The $500,000 must have the space before $ preserved.
    //
    // Verify it FAILS before implementing.
}
```

### Test 2: Special characters preserved in JSON-LD values

```rust
#[test]
fn test_jsonld_special_characters_preserved() {
    // Setup: Front matter description containing "$500,000":
    //   "won a $500,000 prize"
    //
    // Assert: JSON-LD description contains "a $500,000 prize"
    //   (space before $ preserved, $ not stripped).
    //
    // Verify it FAILS if the space/$ is being mangled.
}
```

### Test 3: Headline matches jekyll-seo-tag format

```rust
#[test]
fn test_jsonld_headline_format() {
    // Setup: Page with title "My Great Article"
    //
    // Assert: JSON-LD headline field is "My Great Article"
    //   (matches jekyll-seo-tag format exactly).
    //
    // Check if there's truncation at 200 chars for headline too.
}
```

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with JSON-LD value tests
- [ ] Investigation documents exact count of each sub-type of JSON-LD value diff
- [ ] Description truncation matches jekyll-seo-tag: 200 chars, word boundary, `...` suffix
- [ ] `$` and other special characters preserved correctly in JSON-LD values
- [ ] Headline field matches jekyll-seo-tag output
- [ ] After issues 184 and 185 are resolved, remaining diffs addressed or documented
- [ ] No regressions in existing JSON-LD tests
