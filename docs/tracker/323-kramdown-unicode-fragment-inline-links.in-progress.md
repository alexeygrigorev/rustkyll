# Issue 323: Kramdown inline links with Unicode URL fragments not parsed

## Problem

Inline markdown links whose URL contains non-ASCII characters in the fragment
(after `#`) are not being parsed by the kramdown span parser. They appear as raw
markdown text in the HTML output instead of being rendered as `<a>` elements.

**Example** (Bulgarian source):

```markdown
[споделят собствеността върху проекта](../building-community/#споделете-собствеността-върху-вашия-проект)
```

**Expected output:**
```html
<a href="../building-community/#споделете-собствеността-върху-вашия-проект">споделят собствеността върху проекта</a>
```

**Actual output:**
```html
[споделят собствеността върху проекта](../building-community/#споделете-собствеността-върху-вашия-проект)
```

Links with ASCII-only fragments work fine in the same pages. Links with Unicode
text in the display portion also work fine. Only the URL fragment portion causes
the failure.

## Impact

- 67 pages in opensource-guide have at least one raw markdown link in the output
- Estimated ~250-570 DOM diffs caused by this (missing `<a>`, `<em>`, `<strong>` elements; `text_differs` with raw markdown syntax)
- Affects all languages with non-ASCII heading IDs: Arabic, Bulgarian, Bengali, Chinese, German, Greek, Spanish, Farsi, Hindi, Hungarian, Indonesian, Italian, Japanese, Korean, Polish, Portuguese, Romanian, Russian, Thai, Turkish, Ukrainian, Vietnamese, zh-Hant
- This is a core engine bug affecting any Jekyll site with non-English internal anchor links

## Root Cause Investigation

The link parsing lives in `src/kramdown_parser/span_parser.rs`, function
`try_parse_inline_link` (line ~3311). The function uses `chars: &[char]` which
should handle Unicode correctly at the character level.

The scanning loop for balanced parentheses (lines ~3425-3447) iterates over
`chars` looking for `(`, `)`, and `\\` only, so Unicode should pass through.

Possible causes to investigate:
1. The `end` parameter passed to `try_parse_inline_link` may be calculated
   incorrectly (byte length vs char count confusion somewhere upstream)
2. Some earlier span-parsing step may be splitting/truncating the text at the
   Unicode boundary before `try_parse_link` is called
3. A line-splitting or paragraph-splitting step may break on multi-byte chars
4. The URL validation or escaping step may reject URLs with non-ASCII chars

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] Inline links with non-ASCII URL fragments render as `<a>` elements
- [ ] Inline links with non-ASCII characters in both path and fragment work
- [ ] Links with mixed ASCII path + Unicode fragment work: `(../page/#кириллица)`
- [ ] Links with pure Unicode fragment work: `(#кириллица)`
- [ ] Links with relative paths and Unicode fragments work: `(../building-community/#споделете-собствеността-върху-вашия-проект)`
- [ ] Existing ASCII-only links continue to work (no regression)
- [ ] `cargo test` passes with new tests covering all Unicode link patterns
- [ ] Building opensource-guide produces no raw markdown links in HTML output for the affected 67+ pages

## Test Scenarios

### Unit: Kramdown span parser
- Parse `[text](../page/#кириллица)` -- verify produces `<a href="...">text</a>`
- Parse `[text](#споделете-собствеността)` -- verify Cyrillic fragment works
- Parse `[text](../page/#日本語)` -- verify CJK fragment works
- Parse `[text](../page/#العربية)` -- verify Arabic fragment works
- Parse `[text](url#ascii)` alongside `[text](url#кириллица)` in same paragraph -- verify both render
- Parse `[**bold text**](../page/#юникод)` -- verify emphasis inside link with Unicode URL works
- Regression: parse `[text](../page/#ascii-anchor)` still works
- Regression: parse `[text](https://example.com)` still works

### Integration: opensource-guide site build
- Build the Bulgarian best-practices page, verify no raw `[...](` patterns in output
- Build all BG pages, verify all inline links are rendered as `<a>` elements
- Run DOM comparison on opensource-guide, verify the text_differs and missing_element counts for body content drop significantly

## Output Verification

- Build opensource-guide with rustkyll
- Run `grep -r '\[.*\](' websites/opensource-guide/_site_rustkyll/bg/ | grep -v 'class=\|script\|style'` -- should return zero matches
- Run DOM comparison: the ~250 content-related diffs (text_differs, missing a/em/strong elements) in article body should be eliminated
- Compare a specific page (bg/best-practices) to verify the link renders identically to Jekyll output

## Dependencies

None -- this is a standalone kramdown parser bug fix.

## Log

### [SWE] 2026-03-23
- Root cause: NOT in kramdown_parser/span_parser.rs as spec suggested. The bug is in
  `src/frontmatter.rs` function `protect_non_ascii_in_link_urls` (issue 313 code).
  It used `\x01` control characters as placeholders. pulldown-cmark rejects URLs
  containing control characters, so links with non-ASCII fragments appeared as raw text.
- Wrote 6 failing tests in frontmatter.rs (Cyrillic, CJK, Arabic, Chinese, mixed) -- all FAIL
  as expected: output is `<p>[text](../page/#...)` instead of `<a href="...">`.
- Wrote 10 tests in kramdown_parser/tests.rs -- all PASS (kramdown parser handles Unicode fine).
- Fix: Changed `URL_NON_ASCII_PREFIX`/`SUFFIX` from `"\x01U"`/`"U\x01"` to
  `"XNONASCII"`/`"XENDNA"` (URL-safe alphanumeric placeholders).
- Ran all 6 frontmatter tests: PASS
- Ran all 10 kramdown_parser tests: PASS
- Full test suite: 2615+ tests pass, 0 failures
- Clippy: clean (no warnings with -D warnings)
- Format: clean (cargo fmt --check passes)
- Files modified: src/frontmatter.rs (fix + 6 tests), src/kramdown_parser/tests.rs (10 tests)
