# Issue 282: Kramdown parser Phase 3 - Fix remaining span element parsing

## Problem

The span parser (`src/kramdown_parser/span_parser.rs`, 3282 lines) is already wired into the module (`pub mod span_parser` in mod.rs) and integrated into the HTML converter (`html.rs` calls `span_parser::spans_to_html()` for text in paragraphs, headers, list items, table cells, etc.). It also already extracts definitions via `span_parser::extract_definitions()` in the top-level `to_html()` function.

However, 37 of 62 span-level conformance tests fail. The 25 passing tests cover simpler features (empty links, angle bracket links, imagelinks, empty/normal/error codespans, escaped chars, span extensions, IAL, some text substitutions, math no_engine). The 37 failing tests cover more complex span parsing that needs to be fixed or completed.

## Current state

- **Passing (25 tests):** empty links, imagelinks, angle bracket links, codespans (normal/empty/errors/normal_css_class/rouge_disabled), escaped chars, span extensions (comment/ignored/nomarkdown), span IAL, text substitutions (entities/entities_as_input/entities_numeric/entities_symbolic/greaterthan/lowerthan/typography/typography_subst), math no_engine, invalid HTML spans, link_with_mailto, mark_element
- **Failing (37 tests):** see list below

## Scope

Fix the span parser to pass all 37 currently-failing span conformance tests. This is a translation/debugging task -- the Ruby source in `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/parser/kramdown/` contains the reference implementations for each span type.

### Failing tests by category

**Links (5 tests):**
- `span_01_link_inline` -- inline links `[text](url)` and `[text](url "title")`
- `span_01_link_reference` -- reference-style links `[text][ref]`
- `span_01_link_link_defs` -- link definition blocks `[ref]: url`
- `span_01_link_link_defs_with_ial` -- link definitions with IAL `{:.class}`
- `span_01_link_image_in_a` -- images nested inside links

**Emphasis (4 tests):**
- `span_02_emphasis_normal` -- basic `*em*`, `**strong**`, `_em_`, `__strong__`
- `span_02_emphasis_empty` -- empty emphasis markers
- `span_02_emphasis_errors` -- invalid emphasis patterns
- `span_02_emphasis_nesting` -- nested emphasis `**a *b* c**`

**Code spans (2 tests):**
- `span_03_codespan_highlighting` -- code spans with syntax highlighting
- `span_03_codespan_rouge_simple` -- Rouge-style highlighting in code spans

**Footnotes (11 tests):**
- `span_04_footnote_definitions` -- `[^name]: content`
- `span_04_footnote_markers` -- `[^name]` reference markers
- `span_04_footnote_placement` -- footnote section placement
- `span_04_footnote_inside_footnote` -- nested footnotes
- `span_04_footnote_backlink_text` -- custom backlink text
- `span_04_footnote_backlink_inline` -- inline backlink style
- `span_04_footnote_footnote_nr` -- starting footnote number
- `span_04_footnote_footnote_prefix` -- footnote ID prefix
- `span_04_footnote_footnote_link_text` -- custom link text
- `span_04_footnote_regexp_problem` -- edge cases in footnote regex
- `span_04_footnote_without_backlink` -- footnotes without backlinks

**HTML spans (6 tests):**
- `span_05_html_normal` -- inline HTML tags `<b>`, `<i>`, etc.
- `span_05_html_across_lines` -- HTML tags spanning multiple lines
- `span_05_html_button` -- button element inline
- `span_05_html_markdown_attr` -- `markdown="1"` on inline HTML
- `span_05_html_raw_span_elements` -- raw span elements passed through
- `span_05_html_xml` -- XML namespaced inline tags

**Abbreviations (4 tests):**
- `span_abbreviations_abbrev` -- `*[abbr]: expansion` definitions
- `span_abbreviations_abbrev_defs` -- abbreviation definition handling
- `span_abbreviations_abbrev_in_html` -- abbreviations inside HTML
- `span_abbreviations_in_footnote` -- abbreviations inside footnotes

**Other (5 tests):**
- `span_autolinks_url_links` -- `<http://example.com>` autolinks
- `span_extension_options` -- inline extension options
- `span_line_breaks_normal` -- trailing spaces / backslash line breaks
- `span_math_normal` -- inline math `$$...$$`
- `span_text_substitutions_entities_as_char` -- entity-as-character substitutions

## Approach

This is a single SWE task. The span parser code already exists (3282 lines). The work is:

1. Run each failing test, compare actual vs expected output
2. Identify the parsing bug in `span_parser.rs`
3. Fix by referencing the kramdown Ruby source for the corresponding span type
4. Repeat until all 37 tests pass

The Ruby reference files are at:
- `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/parser/kramdown/emphasis.rb`
- `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/parser/kramdown/link.rb`
- `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/parser/kramdown/codespan.rb`
- `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/parser/kramdown/footnote.rb`
- `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/parser/kramdown/html.rb`
- `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/parser/kramdown/abbreviation.rb`
- `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/parser/kramdown/autolink.rb`
- `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/parser/kramdown/math.rb`
- `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/parser/kramdown/line_break.rb`
- `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/parser/kramdown/typographic_symbol.rb`
- `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/parser/kramdown/smart_quotes.rb`
- `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/parser/kramdown/escaped_chars.rb`
- `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/parser/kramdown/extensions.rb`

And the HTML converter:
- `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/converter/html.rb`

## Dependencies

- Issue 281a (Lists) -- DONE
- Issue 281b (Tables) -- DONE
- No dependency on 281c or 281d -- span parsing is independent of block-level HTML, definition lists, ALD, extensions, and TOC

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes all existing tests (no regressions on the 25 currently-passing span tests)
- [ ] All 5 link tests pass (`inline`, `reference`, `link_defs`, `link_defs_with_ial`, `image_in_a`)
- [ ] All 4 emphasis tests pass (`normal`, `empty`, `errors`, `nesting`)
- [ ] Both code span tests pass (`highlighting`, `rouge_simple`)
- [ ] All 11 footnote tests pass
- [ ] All 6 HTML span tests pass (`normal`, `across_lines`, `button`, `markdown_attr`, `raw_span_elements`, `xml`)
- [ ] All 4 abbreviation tests pass
- [ ] All 5 other span tests pass (`autolinks`, `extension_options`, `line_breaks`, `math_normal`, `entities_as_char`)
- [ ] Total: 62/62 span-level conformance tests pass (25 existing + 37 newly fixed)
- [ ] No regressions in block-level tests (116 block tests still pass)
- [ ] `cargo clippy -- -D warnings` passes (excluding pre-existing vendor warnings)
- [ ] `cargo fmt` is clean

## Test Scenarios

### Unit: Link parsing
- `[text](url)` renders as `<a href="url">text</a>`
- `[text](url "title")` renders with `title` attribute
- `[text][ref]` with `[ref]: url` renders correctly
- `[text][ref]{:.cls}` applies class to the link
- `[![alt](img)](url)` renders image inside link

### Unit: Emphasis parsing
- `*em*` renders as `<em>em</em>`
- `**strong**` renders as `<strong>strong</strong>`
- `**a *b* c**` renders as `<strong>a <em>b</em> c</strong>`
- Empty `**` markers handled gracefully
- Invalid nesting patterns produce correct output

### Unit: Footnote parsing
- `[^name]` creates a superscript link to footnote
- `[^name]: content` creates footnote definition at document end
- Nested footnotes (footnote referencing another footnote) work
- Custom backlink text and footnote numbering options work
- Footnotes without backlinks render correctly

### Unit: HTML span parsing
- `<b>text</b>` passes through as inline HTML
- HTML tags spanning multiple lines are handled
- `markdown="1"` on inline HTML triggers markdown parsing of content
- Raw span elements (`<br>`, `<img>`) pass through

### Unit: Abbreviation parsing
- `*[HTML]: Hyper Text Markup Language` defines abbreviation
- All instances of `HTML` in text get wrapped in `<abbr title="...">`
- Abbreviations work inside footnotes and HTML blocks

### Integration: Full span rendering
- Parse each test `.text` file and compare output to expected `.html`
- Run `./scripts/cargo-safe test --lib kramdown_parser::tests::kramdown_span` and verify 0 failures

## Notes

- The span parser file is large (3282 lines) but most of the infrastructure is in place
- Focus on debugging existing code, not rewriting -- compare actual vs expected output for each failing test to find the specific parsing bug
- Footnotes are the largest category (11 tests) and may have the most complex interactions
- Some span tests may fail because they depend on block-level features not yet implemented (e.g., `abbreviations_in_footnote` may need footnote block support). If any tests cannot pass due to missing block-level features from 281c/281d, document them and create follow-up issues
