# Issue 285: Kramdown parser - CJK line break removal and encoding handling

## Problem

Two root-level conformance tests fail that are not covered by any existing phase issue:

1. **CJK line break removal** (`cjk_line_break` test): In kramdown, when two lines of CJK (Chinese/Japanese/Korean) text are joined, the newline between them should be removed rather than converted to a space. This is because CJK text does not use spaces between words, so a line break in the source should not introduce a space in the output.

2. **Encoding handling** (`encoding` test): Proper handling of non-ASCII/UTF-8 encoded content in kramdown parsing and HTML output.

## Scope

- Implement CJK line break removal in the span parser or text normalization step
- Ensure encoding is handled correctly throughout the parsing pipeline
- These are small, isolated fixes

## Test files

| Test | Location | What it tests |
|------|----------|---------------|
| `cjk_line_break` | `src/kramdown_parser/testcases/cjk-line-break.text` | CJK characters across line breaks should not get a space inserted |
| `encoding` | `src/kramdown_parser/testcases/encoding.text` | Non-ASCII/UTF-8 content processed correctly |

## Dependencies

- Issue 282 (Phase 3 spans) should ideally be done first, since CJK line break removal happens during span/text processing
- Can be done in parallel with 281c/281d if the span parser is stable

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes all existing tests (no regressions)
- [ ] `kramdown_cjk_line_break` conformance test passes
- [ ] `kramdown_encoding` conformance test passes
- [ ] CJK characters (Chinese, Japanese, Korean) at line boundaries are joined without inserting a space
- [ ] Non-CJK text at line boundaries still gets a space (standard behavior preserved)
- [ ] UTF-8 encoded content with multibyte characters renders correctly in HTML output
- [ ] Non-ASCII content in headers, links, emphasis, and other elements works correctly

## Test Scenarios

### Unit: CJK line break detection
- Two lines of Chinese characters joined without space: `\u4e2d\u6587\n\u6587\u5b57` -> `\u4e2d\u6587\u6587\u5b57`
- Two lines of Japanese characters joined without space
- CJK character followed by Latin character: space preserved
- Latin character followed by CJK character: space preserved
- Mixed CJK and Latin on same line: spaces preserved where they exist in source

### Unit: Encoding
- UTF-8 content with multibyte characters (Chinese, Japanese, Korean, Arabic, emoji) renders correctly
- HTML entities for non-ASCII characters are preserved
- Attribute values with non-ASCII characters are correctly quoted

### Integration
- Parse `cjk-line-break.text` and compare output to `cjk-line-break.html`
- Parse `encoding.text` and compare output to `encoding.html`
