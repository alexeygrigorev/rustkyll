# Issue 288: Kramdown footnotes, links, and abbreviations span fixes

## Problem

11 span-level conformance tests fail across three related categories: footnotes (5), links (3), and abbreviations (3). These are all span-level features in `span_parser.rs` that need debugging and completion.

## Scope

### Footnotes (5 failing tests)

Footnote markers (`[^name]`) and definitions (`[^name]: content`) need to produce correct HTML. The footnote renderer generates a `<div class="footnotes">` section with an ordered list, backlinks, and superscript references in the text.

| Test name | Testcase path | Options | What it tests |
|-----------|---------------|---------|---------------|
| `kramdown_span_04_footnote_backlink_inline` | `span/04_footnote/backlink_inline` | varies | Inline backlink style (backlink in same `<p>` as footnote text) |
| `kramdown_span_04_footnote_definitions` | `span/04_footnote/definitions` | none | Multiple footnote definitions, multi-paragraph footnotes, definition ordering |
| `kramdown_span_04_footnote_markers` | `span/04_footnote/markers` | none | `[^name]` produces superscript `<sup>` with `<a href="#fn:name">` link, correct numbering |
| `kramdown_span_04_footnote_placement` | `span/04_footnote/placement` | none | Footnote section placed at end of document, before closing tags |
| `kramdown_span_04_footnote_regexp_problem` | `span/04_footnote/regexp_problem` | none | Edge cases in footnote name regex matching (names with special chars) |

Note: 6 other footnote tests also exist (backlink_text, footnote_link_text, footnote_nr, footnote_prefix, inside_footnote, without_backlink) -- these are currently passing or are handled by other issues. This issue covers only the 5 that are currently failing.

### Links (3 failing tests)

Reference-style links and link definitions need fixes in the span parser.

| Test name | Testcase path | Options | What it tests |
|-----------|---------------|---------|---------------|
| `kramdown_span_01_link_link_defs` | `span/01_link/link_defs` | none | `[ref]: url "title"` definitions, multiple defs, defs with various URL formats |
| `kramdown_span_01_link_link_defs_with_ial` | `span/01_link/link_defs_with_ial` | none | Link definitions with IAL: `[ref]: url\n{:.class}` applies class to the link |
| `kramdown_span_01_link_reference` | `span/01_link/reference` | none | `[text][ref]` reference links, `[text][]` self-reference, case-insensitive matching |

Note: 5 other link tests (empty, image_in_a, imagelinks, inline, links_with_angle_brackets) are currently passing.

### Abbreviations (3 failing tests)

Abbreviation definitions (`*[ABBR]: Full Text`) cause all instances of `ABBR` in the document to be wrapped in `<abbr title="Full Text">ABBR</abbr>`.

| Test name | Testcase path | Options | What it tests |
|-----------|---------------|---------|---------------|
| `kramdown_span_abbreviations_abbrev` | `span/abbreviations/abbrev` | none | Basic abbreviation matching, multiple abbreviations, longest-match-first |
| `kramdown_span_abbreviations_abbrev_defs` | `span/abbreviations/abbrev_defs` | none | Abbreviation definition handling, definitions stripped from output |
| `kramdown_span_abbreviations_abbrev_in_html` | `span/abbreviations/abbrev_in_html` | none | Abbreviations inside HTML elements (not replaced inside tags, only in text content) |

Note: `in_footnote` abbreviation test exists but is not in the current 51 failing tests.

## Approach

1. For each failing test, run it and diff actual vs expected output
2. Fix the span parser (`span_parser.rs`) and HTML converter (`html.rs`) as needed
3. Reference the kramdown Ruby source for each feature:
   - Footnotes: `footnote.rb` (parser) and `html.rb` (converter, search for `convert_footnote`)
   - Links: `link.rb` (parser) and `html.rb` (converter, search for `convert_a`)
   - Abbreviations: `abbreviation.rb` (parser) and `html.rb` (converter, search for `convert_abbreviation`)

## Dependencies

- Issue 282 (Phase 3 spans) should be done or stable -- this issue fixes remaining bugs in the span parser
- No dependency on block-level issues (footnotes, links, and abbreviations are all span-level)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes all existing tests (no regressions)
- [ ] All 5 footnote tests pass:
  - [ ] `kramdown_span_04_footnote_backlink_inline`
  - [ ] `kramdown_span_04_footnote_definitions`
  - [ ] `kramdown_span_04_footnote_markers`
  - [ ] `kramdown_span_04_footnote_placement`
  - [ ] `kramdown_span_04_footnote_regexp_problem`
- [ ] All 3 link tests pass:
  - [ ] `kramdown_span_01_link_link_defs`
  - [ ] `kramdown_span_01_link_link_defs_with_ial`
  - [ ] `kramdown_span_01_link_reference`
- [ ] All 3 abbreviation tests pass:
  - [ ] `kramdown_span_abbreviations_abbrev`
  - [ ] `kramdown_span_abbreviations_abbrev_defs`
  - [ ] `kramdown_span_abbreviations_abbrev_in_html`
- [ ] Total: 11/11 failing tests fixed
- [ ] Footnote section renders as `<div class="footnotes">` with `<ol>` containing `<li id="fn:name">` entries
- [ ] Footnote backlinks render as `<a href="#fnref:name" class="reversefootnote">` with configurable text
- [ ] Footnote markers in text render as `<sup id="fnref:name"><a href="#fn:name" class="footnote">N</a></sup>`
- [ ] Link definitions are extracted from document and not rendered as visible text
- [ ] Reference links resolve case-insensitively
- [ ] Abbreviation definitions are extracted and not rendered as visible text
- [ ] Abbreviation matching does not replace text inside HTML tags or code spans
- [ ] No regressions in currently-passing span tests
- [ ] No regressions in block tests
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` is clean

## Test Scenarios

### Unit: Footnote rendering
- `[^1]` in text produces `<sup id="fnref:1"><a href="#fn:1" class="footnote">1</a></sup>`
- `[^1]: Definition text` produces `<li id="fn:1"><p>Definition text <a href="#fnref:1" class="reversefootnote">&#8617;</a></p></li>`
- Multiple footnotes numbered sequentially
- Multi-paragraph footnote definition wraps each paragraph in `<p>`
- Footnote section placed at end of document content

### Unit: Link definition resolution
- `[ref]: http://example.com "Title"` defines link ref
- `[text][ref]` resolves to `<a href="http://example.com" title="Title">text</a>`
- `[text][]` resolves using `text` as the ref key (case-insensitive)
- `[ref]: url\n{:.class}` applies IAL class to all links using that ref

### Unit: Abbreviation matching
- `*[HTML]: Hyper Text Markup Language` defines abbreviation
- `HTML` in text becomes `<abbr title="Hyper Text Markup Language">HTML</abbr>`
- `HTML` inside `<code>` or HTML tag attributes is NOT replaced
- Multiple abbreviations are matched longest-first to avoid partial matches

### Integration
- Parse each test `.text` file and compare output to expected `.html`
- Run `./scripts/cargo-safe test --lib kramdown_span_04_footnote kramdown_span_01_link_link_defs kramdown_span_01_link_reference kramdown_span_abbreviations` and verify 0 failures

## Ruby reference files

- `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/parser/kramdown/footnote.rb`
- `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/parser/kramdown/link.rb`
- `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/parser/kramdown/abbreviation.rb`
- `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/converter/html.rb`
