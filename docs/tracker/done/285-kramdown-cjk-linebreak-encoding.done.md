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

## Log

### [SWE] 2026-03-21

**Root causes identified:**

1. **CJK trailing newline**: The CJK test expected `\n\n` at end of output (because input ended with `\n\n`), but `extract_definitions()` in span_parser.rs was stripping trailing blank lines. Also, `to_html_with_options()` didn't preserve trailing `\n\n` from input.

2. **Encoding - missing blank between DL and table**: The definition list parser consumed trailing blank lines that should have been left for the parent parser to create Blank elements (needed for inter-block spacing).

3. **Encoding - span-mode HTML block rendering**: `<p markdown='1'>` content was rendered with `<p>\n` + content + `\n</p>` instead of kramdown's `<p>content</p>` format.

**Fixes applied:**

- `src/kramdown_parser/span_parser.rs`: Preserve trailing `\n\n` in `extract_definitions()` output when original input had trailing blank lines
- `src/kramdown_parser/parser.rs`: In DL parser, restore `*pos` to the blank line position when breaking out of the DL loop, so parent parser creates proper Blank elements
- `src/kramdown_parser/html.rs`: (a) Fixed span-mode HTML block rendering to output `<tag>content</tag>` inline; (b) Added trailing blank detection in `convert_with_context` for documents ending with Blank elements
- `src/kramdown_parser/mod.rs`: In `to_html_with_options`, preserve trailing `\n\n` when original input ended with blank lines

**Tests added:** 9 unit tests
- 4 CJK line break tests (Chinese joined, Japanese joined, Latin preserves space, disabled by default)
- 5 encoding tests (German umlauts, emphasis with umlauts, header with non-ASCII, CJK in emphasis, emoji)

**Test results:** 2 conformance tests fixed (kramdown_cjk_line_break, kramdown_encoding), 9 new unit tests pass, 0 regressions from my changes. Clippy clean, fmt clean.

**Files modified:**
- src/kramdown_parser/span_parser.rs
- src/kramdown_parser/parser.rs
- src/kramdown_parser/html.rs
- src/kramdown_parser/mod.rs
- src/kramdown_parser/tests.rs

**Note:** Another agent is concurrently modifying parser.rs and span_parser.rs (issue 282 - ALD support). Their changes introduced a separate regression in `kramdown_block_13_definition_list_auto_ids` (bare words in IAL treated as ALD references instead of attributes). This is not caused by issue 285 changes.
