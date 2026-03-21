# Issue 287: Kramdown HTML-to-native conversion and remaining block HTML fixes

## Problem

19 block HTML conformance tests (category `block/09_html`) fail. These break down into:

- **9 html_to_native tests**: The `html_to_native: true` option converts HTML elements (`<p>`, `<em>`, `<strong>`, `<code>`, `<h1>`-`<h6>`, `<ul>`, `<ol>`, `<table>`) to native kramdown elements so they are processed as markdown. This feature is not yet implemented.
- **10 other block HTML tests**: Remaining failures in `parse_block_html`, `simple`, `xml`, `cdata_section`, `markdown_attr`, `parse_as_raw`, `parse_as_span`, `content_model_deflists`, `content_model_tables`, `html5_attributes`. These involve deep markdown-in-HTML parsing, content model awareness, and edge cases not yet handled.

## Scope

### html_to_native conversion (9 tests)

Implement the `html_to_native` option which converts HTML block elements to native kramdown elements before rendering. When enabled:

- `<p>text</p>` becomes a native Paragraph element with `text` parsed as spans
- `<em>text</em>` / `<strong>text</strong>` become native emphasis elements
- `<code>text</code>` becomes a native code span
- `<h1>text</h1>` through `<h6>text</h6>` become native Header elements
- `<ul><li>...</li></ul>` becomes a native unordered List
- `<ol><li>...</li></ol>` becomes a native ordered List
- `<table>...</table>` becomes a native Table (simple and normal variants)
- HTML comments are preserved
- Typographic entities (`&ldquo;`, `&mdash;`, etc.) are converted to their character equivalents

| Test name | Testcase path | Options |
|-----------|---------------|---------|
| `kramdown_block_09_html_html_to_native_code` | `block/09_html/html_to_native/code` | `html_to_native: true` |
| `kramdown_block_09_html_html_to_native_comment` | `block/09_html/html_to_native/comment` | `html_to_native: true` |
| `kramdown_block_09_html_html_to_native_emphasis` | `block/09_html/html_to_native/emphasis` | `html_to_native: true` |
| `kramdown_block_09_html_html_to_native_header` | `block/09_html/html_to_native/header` | `html_to_native: true, auto_ids: true` |
| `kramdown_block_09_html_html_to_native_list_ol` | `block/09_html/html_to_native/list_ol` | `html_to_native: true` |
| `kramdown_block_09_html_html_to_native_list_ul` | `block/09_html/html_to_native/list_ul` | `html_to_native: true` |
| `kramdown_block_09_html_html_to_native_paragraph` | `block/09_html/html_to_native/paragraph` | `html_to_native: true` |
| `kramdown_block_09_html_html_to_native_table_simple` | `block/09_html/html_to_native/table_simple` | `html_to_native: true` |
| `kramdown_block_09_html_html_to_native_typography` | `block/09_html/html_to_native/typography` | `html_to_native: true` |

### Remaining block HTML fixes (10 tests)

| Test name | Testcase path | Options | What needs fixing |
|-----------|---------------|---------|-------------------|
| `kramdown_block_09_html_simple` | `block/09_html/simple` | `parse_block_html: true` | Deep markdown-in-HTML: nested divs with markdown parsing, inline elements inside parsed blocks |
| `kramdown_block_09_html_parse_block_html` | `block/09_html/parse_block_html` | `parse_block_html: true` | `<DIV>` case-insensitive, nested divs, code blocks in divs with proper indentation |
| `kramdown_block_09_html_xml` | `block/09_html/xml` | none | XML namespaced tags case-sensitive matching |
| `kramdown_block_09_html_cdata_section` | `block/09_html/cdata_section` | none | CDATA content stripping / handling in block and inline contexts |
| `kramdown_block_09_html_markdown_attr` | `block/09_html/markdown_attr` | none | `markdown="block"`, `markdown="span"`, `markdown="1"`, `markdown="0"` per-element parsing |
| `kramdown_block_09_html_parse_as_raw` | `block/09_html/parse_as_raw` | `parse_block_html: true` | Script/style content preserved literally even with parse_block_html |
| `kramdown_block_09_html_parse_as_span` | `block/09_html/parse_as_span` | `parse_block_html: true` | `<p>` content parsed as span-level markdown |
| `kramdown_block_09_html_content_model_deflists` | `block/09_html/content_model/deflists` | none | Definition lists inside/around block HTML |
| `kramdown_block_09_html_content_model_tables` | `block/09_html/content_model/tables` | none | Tables inside/around block HTML |
| `kramdown_block_09_html_html5_attributes` | `block/09_html/html5_attributes` | none | Boolean attributes (`<p class>`), unquoted values, mixed quote styles |

## Approach

1. Add `html_to_native` field to Options struct
2. Implement an HTML-to-native conversion pass that runs before rendering:
   - Parse HTML block content as an HTML DOM (can use a simple tag-based parser)
   - Convert recognized HTML elements to kramdown Element tree nodes
   - Re-render via the normal kramdown HTML converter
3. For the 10 remaining block HTML tests, debug each one by diffing actual vs expected and fixing the specific parsing/rendering issue
4. Reference the Ruby implementation at `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/parser/kramdown/html.rb`

## Dependencies

- Issue 281c (HTML blocks) must be done -- provides the HTML block parsing infrastructure this issue builds on
- Issue 281a (Lists) and 281b (Tables) must be done -- html_to_native needs native list and table elements

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes all existing tests (no regressions)
- [ ] All 9 html_to_native tests pass:
  - [ ] `kramdown_block_09_html_html_to_native_code`
  - [ ] `kramdown_block_09_html_html_to_native_comment`
  - [ ] `kramdown_block_09_html_html_to_native_emphasis`
  - [ ] `kramdown_block_09_html_html_to_native_header`
  - [ ] `kramdown_block_09_html_html_to_native_list_ol`
  - [ ] `kramdown_block_09_html_html_to_native_list_ul`
  - [ ] `kramdown_block_09_html_html_to_native_paragraph`
  - [ ] `kramdown_block_09_html_html_to_native_table_simple`
  - [ ] `kramdown_block_09_html_html_to_native_typography`
- [ ] All 10 remaining block HTML tests pass:
  - [ ] `kramdown_block_09_html_simple`
  - [ ] `kramdown_block_09_html_parse_block_html`
  - [ ] `kramdown_block_09_html_xml`
  - [ ] `kramdown_block_09_html_cdata_section`
  - [ ] `kramdown_block_09_html_markdown_attr`
  - [ ] `kramdown_block_09_html_parse_as_raw`
  - [ ] `kramdown_block_09_html_parse_as_span`
  - [ ] `kramdown_block_09_html_content_model_deflists`
  - [ ] `kramdown_block_09_html_content_model_tables`
  - [ ] `kramdown_block_09_html_html5_attributes`
- [ ] Total: 19/19 failing block_09_html tests fixed
- [ ] `html_to_native: true` option recognized in Options and `.options` files
- [ ] No regressions in the 12 block_09_html tests that already pass (comment, html_after_block, html_and_codeblocks, html_and_headers, html_to_native_entity, html_to_native_list_dl, html_to_native_table_normal, invalid_html_1, invalid_html_2, not_parsed, processing_instruction, textarea)
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` is clean

## Test Scenarios

### Unit: html_to_native conversion
- `<p>Some text</p>` with `html_to_native: true` renders as `<p>Some text</p>` (native paragraph)
- `<em>text</em>` in a paragraph with `html_to_native: true` renders as `<em>text</em>` (native emphasis)
- `<h2>Header</h2>` with `html_to_native: true` and `auto_ids: true` gets an auto-generated ID
- `<ul><li>item</li></ul>` with `html_to_native: true` renders as native list with proper indentation
- `<table><tr><td>cell</td></tr></table>` with `html_to_native: true` renders as native table

### Unit: Block HTML edge cases
- `<DIV>` (uppercase) detected as block HTML with `parse_block_html: true`
- `<![CDATA[content]]>` CDATA sections handled correctly
- `<p class>` boolean attribute normalized correctly
- `<script>` content never parsed as markdown even with `parse_block_html: true`
- `markdown="span"` on `<p>` parses content as inline markdown

### Integration
- Parse each test `.text` file with its `.options` and compare output to expected `.html`
- Run `./scripts/cargo-safe test --lib kramdown_block_09_html` and verify 0 failures

## Ruby reference files

- `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/parser/kramdown/html.rb`
- `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/converter/html.rb`
