# Issue 291: Fix 11 remaining ignored kramdown conformance tests

## Problem

Kramdown conformance reaches 643/643 (100%) on non-ignored tests. However, 15 tests are marked `#[ignore]`, of which 11 still fail and 4 now pass (and should be un-ignored). These are edge cases deferred during the kramdown rewrite.

## Current State

Running `cargo test -- --ignored` shows:
- **4 passing** (should be un-ignored): `line_break_last_line`, `table_with_footnote`, `highlighting`, `highlighting_opts`
- **11 failing** (need fixing), grouped by category below

## Failing Tests by Category

### Category A: Auto-IDs for Headers (4 tests)

Options parsing for `auto_ids`, `auto_id_prefix`, `auto_id_stripping` already exists in `src/kramdown_parser/options.rs` but the HTML renderer does not use them to generate IDs.

| Test | What it needs |
|------|---------------|
| `block_04_header_with_auto_ids` | When `:auto_ids: true`, auto-generate `id` attributes on all headers using kramdown's slugify algorithm. Must handle duplicate IDs by appending `-1`, `-2`, etc. Must handle numeric-only headers as `section`, `section-1`. Options file also sets `:transliterated_header_ids: true`. |
| `block_04_header_with_auto_id_prefix` | When `:auto_id_prefix: hallo_`, prepend prefix to all auto-generated IDs |
| `block_04_header_with_auto_id_stripping` | When `:auto_id_stripping: true`, strip HTML tags from header content before generating the ID (e.g., `<em class="none">This is a header</em>` becomes `this-is-a-header`) |
| `block_04_header_with_header_links` | When `:header_links: true`, wrap an `<a href="#id"></a>` anchor inside each header that has an ID. Headers with `id=""` get no link. Headers without an ID (when `auto_ids: false`) get no link. |

**Key files:** `src/kramdown_parser/html.rs` (header rendering), `src/kramdown.rs` (slugify function), `src/kramdown_parser/options.rs` (options already parsed)

### Category B: Rouge Syntax Highlighting (3 tests)

Rouge syntax highlighting exists (`src/syntax.rs`, issue 290 done) but has gaps:

| Test | What it needs |
|------|---------------|
| `block_06_codeblock_rouge_simple` | PHP `start_inline=1` language parameter not parsed -- `language-php?start_inline=1` should become `language-php`. The `?params` in fenced code block lang should be stripped for class but passed to highlighter. |
| `block_06_codeblock_rouge_multiple` | Same PHP issue, plus: option `formatter: RougeHTMLFormatters` adds an extra `<div class="custom-class">` wrapper. Token `s2` splitting: kramdown uses single `<span class="s2">"Hello"</span>` while rustkyll splits into `dl`+`s2`+`dl`. This is a syntect-vs-rouge token mapping difference. |
| `block_06_codeblock_guess_lang_css_class` | When `syntax_highlighter_opts.guess_lang: true` but NO syntax_highlighter is set, code blocks should get `highlighter-rouge` wrapper classes. Currently only applied when syntax_highlighter is explicitly set. |

**Key files:** `src/syntax.rs`, `src/kramdown_parser/html.rs` (code block rendering), `src/kramdown_parser/span_parser.rs` (guess_lang handling)

### Category C: Standalone Image / Figure (1 test)

| Test | What it needs |
|------|---------------|
| `block_03_paragraph_standalone_image` | When an image with `{:standalone}` IAL is alone in a paragraph, convert it to a `<figure>` element with `<figcaption>`. Block-level IAL should apply to the figure, image-level IAL to the img. This is a kramdown-specific feature not commonly used. |

**Key files:** `src/kramdown_parser/html.rs` (paragraph rendering), `src/kramdown_parser/span_parser.rs` (image parsing)

### Category D: html_to_native Paragraph (1 test)

| Test | What it needs |
|------|---------------|
| `block_03_paragraph_with_html_to_native` | When `:html_to_native: true`, `<p><img ...></p> some text` should merge the HTML paragraph's content with the following text into a single `<p>`. The `html_to_native` module exists but misses this paragraph-merging case. |

**Key files:** `src/kramdown_parser/html_to_native.rs`, `src/kramdown_parser/parser.rs`

### Category E: Code Block Whitespace (1 test)

| Test | What it needs |
|------|---------------|
| `block_06_codeblock_whitespace` | When `{:.show-whitespaces}` IAL is applied to a code block, render each space/tab with special `<span>` elements: `ws-tab`, `ws-space`, `ws-space-r` (trailing), `ws-space-l` (leading). This is a kramdown-specific rendering mode. |

**Key files:** `src/kramdown_parser/html.rs` (code block rendering)

### Category F: Table Error Handling (1 test)

| Test | What it needs |
|------|---------------|
| `block_14_table_errors` | Link definitions (e.g., `[5]: test`) appearing between table separator and body should break the table. Currently the parser does not recognize link definitions as table-breaking elements. The expected output shows these should be rendered as paragraphs, not tables. |

**Key files:** `src/kramdown_parser/parser.rs` (table parsing)

## Dependencies

None -- kramdown 643/643 is stable. These are incremental improvements.

## Priority Guidance

Categories A and B have the highest real-world impact:
- **Auto-IDs (A)** are used by most Jekyll sites with TOC or anchor links
- **Rouge highlighting (B)** affects code-heavy sites

Categories C and E are kramdown-specific features rarely used in practice. If time-constrained, these can be deferred with follow-up issues.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes all existing tests (no regressions)
- [ ] The 4 already-passing ignored tests (`line_break_last_line`, `table_with_footnote`, `highlighting`, `highlighting_opts`) are un-ignored and run in the normal suite
- [ ] All 4 auto-ID header tests pass (Category A)
- [ ] At least 2 of 3 Rouge tests pass (Category B) -- the `rouge_multiple` custom formatter wrapper is acceptable to defer
- [ ] The table errors test passes (Category F)
- [ ] At least 8 of the 11 failing tests pass total
- [ ] Any tests that cannot be fixed are documented with a clear root cause comment in the test file
- [ ] For any tests not fixed, a follow-up `.todo.md` issue is created in `docs/tracker/`
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` is clean
- [ ] Tests include non-ASCII content where applicable (e.g., auto-ID generation with special characters)

## Test Scenarios

### Unit: Auto-ID generation
- Header `# This is a header` with `auto_ids: true` produces `id="this-is-a-header"`
- Duplicate headers get `-1`, `-2` suffixes
- Numeric-only header `# 23232` gets `id="section"`, second one gets `id="section-1"`
- `auto_id_prefix: hallo_` prepends to all IDs
- `auto_id_stripping: true` strips HTML tags before ID generation
- `header_links: true` adds `<a href="#id"></a>` inside headers with IDs
- Header with `id=""` (empty) gets no link even with `header_links: true`

### Unit: Rouge language parameter parsing
- `php?start_inline=1` in fenced block lang becomes class `language-php`
- Code block with no explicit lang but `guess_lang: true` gets `highlighter-rouge` wrapper

### Unit: Table error recovery
- Input with `[5]: test` between separator row and body row breaks the table
- Broken table renders as paragraphs

### Integration: Conformance suite
- Run `cargo test -- --ignored` and verify at least 8 of 11 now pass
- Run `cargo test` (without --ignored) and verify the 4 un-ignored tests are included and pass
