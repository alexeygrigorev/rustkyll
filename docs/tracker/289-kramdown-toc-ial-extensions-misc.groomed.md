# Issue 289: Kramdown TOC, IAL ordering, mid-document options, HTML spans, math, and def list auto_ids

## Problem

15 conformance tests fail across several block and span categories that all involve "polish" fixes -- features that are partially implemented but have edge cases or ordering issues preventing the tests from passing.

## Scope

### TOC (4 failing tests)

TOC generation is partially implemented but has issues with attribute ordering on headers, nesting indentation, toc_levels filtering, footnote stripping from TOC links, and duplicate header ID suffixing.

| Test name | Testcase path | Options | What it tests |
|-----------|---------------|---------|---------------|
| `kramdown_block_16_toc_toc_exclude` | `block/16_toc/toc_exclude` | `auto_ids: true` | Full TOC with `.no_toc` exclusion, proper nesting, `markdown-toc-*` IDs on TOC links |
| `kramdown_block_16_toc_toc_levels` | `block/16_toc/toc_levels` | `toc_levels: 2..3, auto_ids: true` | Only h2 and h3 headers appear in TOC |
| `kramdown_block_16_toc_toc_with_footnotes` | `block/16_toc/toc_with_footnotes` | `auto_ids: true` | Footnote markers in headers stripped from TOC links but present in header itself |
| `kramdown_block_16_toc_toc_with_links` | `block/16_toc/toc_with_links` | `auto_ids: true, auto_id_stripping: true` | Link refs in headers, duplicate header IDs get `-1` suffix |

### IAL attribute ordering (2 failing tests)

IAL attributes are stored in a HashMap which produces non-deterministic ordering. Kramdown outputs attributes in insertion order (the order they appear in the IAL).

| Test name | Testcase path | Options | What it tests |
|-----------|---------------|---------|---------------|
| `kramdown_block_11_ial_simple` | `block/11_ial/simple` | none | IAL on paragraphs, blockquotes, lists, code blocks, headers; ALD reference resolution; attribute ordering |
| `kramdown_block_11_ial_nested` | `block/11_ial/nested` | none | IAL before/after HTML blocks and blockquotes; pending IAL injection into raw HTML |

### Mid-document option changes (3 failing tests)

The `{::options key="value" /}` extension modifies parser behavior for subsequent content. Currently options are not threaded as mutable state through the parser.

| Test name | Testcase path | Options | What it tests |
|-----------|---------------|---------|---------------|
| `kramdown_block_12_extension_options` | `block/12_extension/options` | none | `{::options parse_block_html="true" /}` changes parsing mid-document, `footnote_nr`, `template` ignored |
| `kramdown_block_12_extension_options2` | `block/12_extension/options2` | none | Options + footnote interaction (footnote_nr change affects numbering) |
| `kramdown_block_12_extension_options3` | `block/12_extension/options3` | none | Options + syntax highlighting configuration |

### Definition list auto_ids (1 failing test)

| Test name | Testcase path | Options | What it tests |
|-----------|---------------|---------|---------------|
| `kramdown_block_13_definition_list_auto_ids` | `block/13_definition_list/auto_ids` | none | Auto-generated IDs on definition terms based on term text, with prefix support |

### HTML spans (3 failing tests)

Inline HTML handling in the span parser needs fixes.

| Test name | Testcase path | Options | What it tests |
|-----------|---------------|---------|---------------|
| `kramdown_span_05_html_button` | `span/05_html/button` | none | `<button>` element inline in paragraph |
| `kramdown_span_05_html_markdown_attr` | `span/05_html/markdown_attr` | none | `markdown="1"` on inline HTML elements triggers markdown parsing of content |
| `kramdown_span_05_html_normal` | `span/05_html/normal` | none | Inline HTML tags `<b>`, `<i>`, `<span>` pass through, proper nesting |

### Extension options (span-level) (1 failing test)

| Test name | Testcase path | Options | What it tests |
|-----------|---------------|---------|---------------|
| `kramdown_span_extension_options` | `span/extension/options` | none | Inline `{::options}` extension modifying span parser behavior |

### Math (span-level) (1 failing test)

| Test name | Testcase path | Options | What it tests |
|-----------|---------------|---------|---------------|
| `kramdown_span_math_normal` | `span/math/normal` | none | Inline math `$$...$$` renders as `\(...\)`, display math on its own line renders as `\[...\]` |

## Approach

1. **IAL ordering**: Replace `HashMap` with `IndexMap` (or `Vec<(String, String)>`) for attribute storage to preserve insertion order
2. **TOC fixes**: Debug each TOC test by diffing actual vs expected:
   - Fix nesting indentation (spaces)
   - Implement `toc_levels` range filtering
   - Strip footnote markers from TOC link text
   - Implement duplicate header ID suffixing (`header-1`)
   - Add `markdown-toc-*` ID prefix to TOC links
3. **Mid-document options**: Make the Options struct mutable during parsing, apply `{::options}` changes to the parser state for subsequent content
4. **Definition list auto_ids**: Fix the ALD reference resolution that broke auto_ids (bare words in IAL treated as ALD references instead of attributes)
5. **HTML spans**: Debug each failing test and fix span_parser.rs handling of inline HTML
6. **Math normal**: Debug inline math rendering vs expected output

## Dependencies

- Issue 281c (HTML blocks) and 281d (ALD/IAL/extensions/TOC) should be done -- this issue fixes remaining bugs from those implementations
- Issue 282 (Phase 3 spans) should be done -- this issue fixes remaining span bugs
- Issue 288 (footnotes) should be done before `toc_with_footnotes` can pass (TOC needs to strip footnote markers, which requires footnote support)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes all existing tests (no regressions)
- [ ] All 4 TOC tests pass:
  - [ ] `kramdown_block_16_toc_toc_exclude`
  - [ ] `kramdown_block_16_toc_toc_levels`
  - [ ] `kramdown_block_16_toc_toc_with_footnotes`
  - [ ] `kramdown_block_16_toc_toc_with_links`
- [ ] Both IAL tests pass:
  - [ ] `kramdown_block_11_ial_simple`
  - [ ] `kramdown_block_11_ial_nested`
- [ ] All 3 extension options tests pass:
  - [ ] `kramdown_block_12_extension_options`
  - [ ] `kramdown_block_12_extension_options2`
  - [ ] `kramdown_block_12_extension_options3`
- [ ] Definition list auto_ids test passes:
  - [ ] `kramdown_block_13_definition_list_auto_ids`
- [ ] All 3 HTML span tests pass:
  - [ ] `kramdown_span_05_html_button`
  - [ ] `kramdown_span_05_html_markdown_attr`
  - [ ] `kramdown_span_05_html_normal`
- [ ] Span extension options test passes:
  - [ ] `kramdown_span_extension_options`
- [ ] Span math normal test passes:
  - [ ] `kramdown_span_math_normal`
- [ ] Total: 15/15 failing tests fixed
- [ ] IAL attributes render in insertion order (not random HashMap order)
- [ ] TOC generates correct nested `<ul>` structure with `id="markdown-toc-*"` on links
- [ ] `{::options}` mid-document changes take effect on subsequent content
- [ ] No regressions in currently-passing tests
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` is clean

## Test Scenarios

### Unit: IAL attribute ordering
- `{:.class1 #myid .class2}` renders attributes in order: `class="class1 class2" id="myid"`
- Multiple IAL on same element merge in document order

### Unit: TOC generation
- Headers h1, h2, h3 produce nested `<ul>` with correct indentation
- `{:.no_toc}` on a header excludes it from TOC
- `toc_levels: 2..3` includes only h2 and h3
- Duplicate header text "Foo" produces IDs `foo` and `foo-1`
- Footnote `[^1]` in header text is stripped from TOC link but kept in header

### Unit: Mid-document options
- `{::options parse_block_html="true" /}` followed by `<div>markdown</div>` parses the div content
- `{::options footnote_nr="5" /}` followed by footnotes starts numbering at 5

### Unit: Definition list auto_ids
- Term text "My Term" gets ID `my-term` on the `<dt>` element
- Auto ID prefix is applied when specified

### Unit: HTML spans
- `<button>Click</button>` in paragraph passes through as inline HTML
- `<b markdown="1">*text*</b>` renders emphasis inside the `<b>` tag
- `<span class="x">text</span>` passes through with attributes preserved

### Unit: Inline math
- `$$x^2$$` in text renders as `\(x^2\)`
- Display math on own line renders as `\[...\]`

### Integration
- Parse each test `.text` file with its `.options` and compare output to expected `.html`
- Run all 15 test names and verify 0 failures

## Ruby reference files

- `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/converter/html.rb` (TOC generation, attribute rendering)
- `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/parser/kramdown/extensions.rb` (options extension)
- `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/parser/kramdown/html.rb` (span HTML)
- `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/parser/kramdown/math.rb` (inline math)

## Notes

- `toc_with_footnotes` depends on footnote support being correct (Issue 288). If 288 is not done yet, this test may need to be addressed after 288.
- `options2` depends on footnote numbering, `options3` depends on syntax highlighting -- both require those features to be working.
- The IAL ordering fix (HashMap to IndexMap) may touch many files but is a mechanical change.
