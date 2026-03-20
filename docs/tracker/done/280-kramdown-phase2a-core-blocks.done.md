# Issue 280: Kramdown parser Phase 2a - Core block elements

## Problem

The kramdown parser module (`src/kramdown_parser/`) has a scaffold with stubs. The parser returns raw text and the HTML converter returns an empty string. We need to implement parsing and rendering of core block elements so the conformance tests in categories 01-07 pass.

## Scope

Implement these block element types in the parser (`parser.rs`) and HTML converter (`html.rs`):

- **Blank** -- blank lines (whitespace-only lines that separate blocks)
- **EOB** (end of block) -- `^` on a line by itself forces block boundary
- **Paragraph** -- basic text blocks wrapped in `<p>` tags
- **Header** -- ATX (`# ...`) and Setext (underlined) headers, with inline `{#id}` support
- **Blockquote** -- `>` prefixed blocks, with nesting and lazy continuation
- **CodeBlock** -- indented code blocks (4-space) AND fenced code blocks (``` and ~~~), with language class
- **HorizontalRule** -- `---`, `***`, `___` etc.

**NOT in scope** (span-level parsing): emphasis, links, images, code spans, smart quotes, line breaks within paragraphs. Text within block elements should be passed through as raw text for now, except where specific test cases require otherwise.

## Dependencies

- Issue 279 (Phase 1 scaffold) must be `.done.md` -- the element types, options parser, test harness, and test case files must all exist.

## Architecture

- **`parser.rs`**: Replace the stub `KramdownParser::parse()` with an actual block-level parser. Process input line-by-line, detecting block boundaries. Build an `Element` tree under the `Document` root.
- **`html.rs`**: Replace the stub `HtmlConverter::convert()` with an actual HTML renderer. Walk the `Element` tree and produce an HTML string.
- The parser should use a line-by-line scanning approach: detect block type from the first line, consume lines until the block ends, then move to the next block.

## Conformance test cases -- IN SCOPE

These must all pass. The test harness and `conformance_test!` macros already exist in `tests.rs`.

### 01_blank_line (2 tests)
- `spaces` -- blank lines with spaces produce empty output
- `tabs` -- blank lines with tabs produce empty output

### 02_eob (3 tests)
- `beginning` -- `^` at start of document
- `end` -- `^` at end of document
- `middle` -- `^` in middle of document

### 03_paragraph (5 in-scope tests)
- `one_para` -- single paragraph wrapped in `<p>`
- `two_para` -- two paragraphs separated by blank line
- `indented` -- paragraphs with 1-3 space indent are still paragraphs; 4+ spaces become code blocks
- `no_newline_at_end` -- file not ending with newline still produces correct output
- `two_para` (multiline) -- continuation lines within a paragraph preserve original spacing

### 04_header (6 in-scope tests)
- `atx_header` -- `#` through `######`, optional closing `#`s, inline `{#id}` attributes, edge cases (no space after `#`, `#` alone is a paragraph, empty header `# ` is a paragraph)
- `atx_header_no_newline_at_end` -- ATX header with no trailing newline
- `setext_header` -- `===` for H1, `---` for H2, inline `{#id}`, multiline text before underline becomes paragraph not header, indented 4+ becomes code
- `setext_header_no_newline_at_end` -- setext header with no trailing newline
- `with_line_break` -- header containing `<br />` tag (pass through as raw text)
- `header_type_offset` -- `:header_offset: 1` option shifts all header levels by 1 (capped at h6)

### 05_blockquote (6 tests)
- `indented` -- blockquotes with 0-3 space indent are blockquotes; 4+ spaces become code; mixed indents stripped
- `lazy` -- lazy continuation (lines without `>` prefix continue the blockquote); nested `> >` lazy continuation; EOB marker `^` ends a blockquote; IAL `{: #id}` on blockquote
- `nested` -- `> >` nesting with blank lines inside blockquote, re-entering outer blockquote after nested
- `no_newline_at_end` -- blockquote with no trailing newline
- `very_long_line` -- single very long line in a blockquote
- `with_code_blocks` -- indented code blocks inside blockquotes (4 spaces beyond the `>` prefix)

### 06_codeblock (12 in-scope tests)
- `normal` -- basic 4-space-indented code blocks, trailing whitespace preserved, extra indent beyond 4 preserved
- `no_newline_at_end` -- indented code block with no trailing newline
- `no_newline_at_end_1` -- indented code block followed by lazy continuation text, no trailing newline
- `with_blank_line` -- blank lines within indented code blocks continue the block; trailing blank lines are stripped
- `with_eob_marker` -- `^` between code blocks splits them into two separate `<pre>` blocks
- `lazy` -- lazy continuation of indented code blocks (lines without 4-space indent continue the code)
- `tilde_syntax` -- `~~~~` fenced code blocks; closing fence must be at least as long as opening; nested shorter tildes are content
- `with_lang_in_fenced_block` -- `~~~ ruby` sets `class="language-ruby"` on `<code>` element; IAL can add class to `<pre>`; `:syntax_highlighter: null` disables highlighter wrapping
- `with_lang_in_fenced_block_name_with_dash` -- language names with dashes (e.g., `act-iii`)
- `with_lang_in_fenced_block_any_char` -- language names with dots, `#`, and non-ASCII (e.g., `asn.1`, `asn#w1`, unicode `русский`)
- `error` -- unclosed fenced code block becomes a paragraph
- `disable-highlighting` -- with `:enable_coderay: false`, code blocks render as plain `<pre><code>` with HTML entities escaped; IAL `{: lang="html"}` sets `lang` attr on `<pre>`

### 07_horizontal_rule (4 tests)
- `normal` -- `***`, `* * *`, `- - -`, `---`, `___` are horizontal rules; `d- -` is NOT; `para\n---` is a setext H2; 4-space-indented `- - -` is a code block; IAL `{:.test}` adds class
- `error` -- mixed marker types (`_ * _`, `--- * * *`) are NOT horizontal rules; they become paragraphs (with entity conversion for `---` to em-dash)
- `sepspaces` -- markers separated by varying spaces
- `septabs` -- markers separated by tabs

## Conformance test cases -- DEFERRED (out of scope for this issue)

These tests exist in `tests.rs` but should NOT be expected to pass in this issue. The SWE should mark them with `#[ignore]` if they are not already.

### 03_paragraph (deferred)
- `line_break_last_line` -- requires span-level parsing (line breaks `  \n` and `\\\n`, autolinks `<https://...>`)
- `standalone_image` -- requires span-level parsing (images, IAL on images, `<figure>` conversion)
- `with_html_to_native` -- requires `:html_to_native: true` HTML parsing

### 04_header (deferred to Phase 2b - IAL/auto-ID)
- `with_auto_id_prefix` -- requires `:auto_ids: true` plus `:auto_id_prefix`
- `with_auto_ids` -- requires `:auto_ids: true` auto-generated IDs
- `with_auto_id_stripping` -- requires `:auto_id_stripping: true`
- `with_header_links` -- requires `:header_links: true`

### 06_codeblock (deferred)
- `highlighting` -- requires syntax highlighting integration (rouge/coderay)
- `highlighting-opts` -- requires syntax highlighting integration
- `rouge/simple` -- requires rouge syntax highlighter
- `rouge/multiple` -- requires rouge syntax highlighter
- `rouge/disabled` -- requires rouge syntax highlighter (even though highlighting is disabled, it tests rouge-specific wrapper structure)
- `guess_lang_css_class` -- requires `:syntax_highlighter_opts: { guess_lang: true }` which produces `highlighter-rouge` wrapper divs
- `whitespace` -- requires `{:.show-whitespaces}` IAL with special whitespace span rendering
- `with_ial` -- requires IAL `{:.cls}` support on code blocks (can be included if IAL on code blocks is straightforward; defer if not)

## Key kramdown behaviors to match (from test case analysis)

### Paragraphs
1. Wrapped in `<p>` tags; content trimmed of leading/trailing whitespace per-paragraph, but internal spacing preserved
2. 1-3 space indent at start of line is still a paragraph (indent stripped for first line only in some cases, preserved in others -- see `indented` test)
3. Blank line separates paragraphs
4. File not ending with newline still produces correct `<p>` closure
5. Output has a trailing newline after each `</p>`

### ATX Headers
1. `#` through `######` for H1-H6; must have space after `#` (except `#header#` which is valid)
2. Optional closing `#`s are stripped: `# header #` -> `<h1>header</h1>`
3. `# ` (hash + space + nothing) and `#` alone (no space) are paragraphs, NOT headers
4. Inline `{#id}` at end sets `id` attribute: `### Header {#id}` -> `<h3 id="id">Header</h3>`
5. `{#id}` must have space before it and valid ID chars (`[A-Za-z][A-Za-z0-9_:.-]*`); IDs starting with digit are NOT valid
6. `{#noid}` without space before the `{` or after closing `##` is NOT parsed as an ID
7. `\#` in header text is an escaped hash, rendered as `#`
8. `:header_offset` option shifts header levels (e.g., offset 1: `# H1` becomes `<h2>`, capped at `<h6>`)

### Setext Headers
1. `===` (one or more `=`) under text = H1; `---` (one or more `-`) under text = H2
2. Only single-line text before the underline counts; multiline text + underline = paragraph
3. Text indented 4+ spaces before underline = code block, not header
4. Inline `{#id}` on the text line (with trailing spaces) sets ID attribute
5. `=` on its own (no text above) is a paragraph

### Blockquotes
1. `>` prefix with optional space after it; 0-3 space indent before `>` is valid
2. Lazy continuation: lines without `>` continue the blockquote paragraph
3. Nested: `> >` or `>>` for nested blockquotes
4. Lazy continuation applies to nested quotes too
5. EOB marker `^` terminates a blockquote
6. Code blocks inside blockquotes: 4 spaces beyond the `>` prefix
7. Blank `>` lines create paragraph breaks within blockquote
8. HTML output uses 2-space indentation for blockquote children

### Code Blocks (indented)
1. Lines with 4+ space indent are code blocks; the first 4 spaces are stripped
2. Blank lines within a code block continue it (if followed by more indented lines)
3. Trailing blank lines at end of code block are stripped
4. Code block always ends with a newline in `<code>` content
5. Lazy continuation: lines without 4-space indent can continue a code block (kramdown-specific)
6. EOB marker `^` between code blocks splits them
7. HTML entities in code are escaped (`<` -> `&lt;`, `>` -> `&gt;`, `&` -> `&amp;`)

### Code Blocks (fenced)
1. Opening fence: 3+ backticks or tildes; closing fence must use same char and be at least as long
2. Language specifier after opening fence: `~~~ ruby` -> `class="language-ruby"` on `<code>`
3. Language names can contain any characters including dots, hashes, dashes, and Unicode
4. Unclosed fenced block (no closing fence) -> treated as paragraph (see `error` test)
5. Content between fences is literal (not parsed for markdown)
6. When `:syntax_highlighter: null` (or `:enable_coderay: false` with no rouge), render as plain `<pre><code>` without highlighter wrapper divs

### Horizontal Rules
1. 3+ of the same marker (`-`, `*`, or `_`) with optional spaces/tabs between them
2. Must use only ONE type of marker per rule (no mixing)
3. No other characters allowed on the line (except spaces/tabs)
4. `---` under text is a setext H2, not a horizontal rule
5. 4-space-indented `- - -` is a code block, not a horizontal rule
6. Rendered as `<hr />`

### Entity conversion
1. The `error` test for horizontal rules shows `---` in paragraph context being converted to em-dash. This is a span-level typographic symbol feature. If this is too complex for Phase 2a, the `error` test may need to be deferred. However, try to implement basic `---`/`--` to em-dash/en-dash conversion in paragraph text first.

### IAL (Inline Attribute List) -- minimal support needed
Several in-scope tests use `{#id}` on headers and `{:.test}` / `{: #id}` on blockquotes and horizontal rules. The SWE must implement at least:
- `{#id}` parsing on ATX and setext headers (built into header syntax)
- `{: ...}` block-level IAL on the line after a block element (for blockquotes, horizontal rules, code blocks)
- Supported IAL attributes: `#id`, `.class`, `key="value"`, `key=value`

If full block IAL is too complex, the SWE may defer these specific IAL tests and create a follow-up issue, but must explicitly list which tests are deferred.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors or warnings
- [ ] `cargo test` passes -- all existing tests continue to pass (no regressions)
- [ ] All 2 blank_line conformance tests pass
- [ ] All 3 EOB conformance tests pass
- [ ] At least 3 of 5 in-scope paragraph conformance tests pass (`one_para`, `two_para`, `no_newline_at_end`; `indented` is required if code block detection works)
- [ ] At least 5 of 6 in-scope header conformance tests pass (atx_header, setext_header, no-newline variants, with_line_break)
- [ ] All 6 blockquote conformance tests pass
- [ ] At least 8 of 12 in-scope codeblock conformance tests pass (normal, no_newline variants, with_blank_line, with_eob_marker, tilde_syntax, error, at least one fenced-with-lang test)
- [ ] All 4 horizontal_rule conformance tests pass (note: `error` test requires `---` -> em-dash entity conversion)
- [ ] Deferred tests are marked with `#[ignore]` and documented in a comment explaining why
- [ ] Parser produces correct `Element` AST for each block type (verified by conformance test output matching)
- [ ] HTML converter renders correct output (verified by conformance test exact string match)
- [ ] Code blocks properly escape HTML entities (`<`, `>`, `&`)
- [ ] No `unwrap()` in library code (`parser.rs`, `html.rs`) -- use `Result`/`Option` properly
- [ ] Unicode content in fenced code block language names works (see `with_lang_in_fenced_block_any_char` test with `русский`)

## Test Scenarios

### Unit: Blank lines and EOB
- Blank lines (spaces only, tabs only) produce empty HTML output
- EOB marker `^` at beginning, middle, and end of document produce empty output
- EOB marker between code blocks splits them into separate `<pre>` elements

### Unit: Paragraph parsing
- Single paragraph text -> `<p>text</p>\n`
- Two paragraphs separated by blank line -> two `<p>` elements separated by blank line
- Lines with 1-3 space indent are paragraphs; 4+ space indent starts a code block
- No trailing newline in input still produces properly closed output

### Unit: ATX header parsing
- `# H1` through `###### H6` produce correct `<h1>` through `<h6>`
- Closing `#`s are stripped: `## header ##` -> `<h2>header</h2>`
- `# ` and `#` alone are paragraphs, not headers
- `{#id}` at end of header sets `id` attribute
- Invalid IDs (starting with digit, no space before `{`) are NOT parsed as IDs
- Escaped hash `\#` renders as literal `#`
- `:header_offset: 1` shifts `# H1` to `<h2>`

### Unit: Setext header parsing
- Text followed by `===` line -> `<h1>`; text followed by `---` -> `<h2>`
- Multiline text followed by `===` is a paragraph (not a header)
- Text indented 4+ spaces followed by `=` is a code block
- `{#id}` on text line sets `id` attribute
- Bare `=` with no text above is a paragraph

### Unit: Blockquote parsing
- `> text` -> `<blockquote><p>text</p></blockquote>` with 2-space indentation
- Lazy continuation: line without `>` continues the blockquote paragraph
- `> > text` creates nested blockquotes
- Code blocks inside blockquotes (4 spaces after `>`)
- EOB `^` terminates blockquote
- Blockquote with no trailing newline

### Unit: Code block parsing (indented)
- 4-space-indented text -> `<pre><code>text\n</code></pre>`
- Extra indent beyond 4 is preserved
- Blank lines within code block continue it
- Trailing blank lines stripped
- HTML entities escaped in code content

### Unit: Code block parsing (fenced)
- Backtick fence opens and closes code block
- Tilde fence opens and closes; closing must be >= opening length
- Language after fence -> `class="language-X"` on `<code>`
- Unclosed fence -> paragraph fallback
- Language names with special chars (dots, dashes, Unicode)

### Unit: Horizontal rule parsing
- `***`, `---`, `___` with 3+ markers -> `<hr />`
- Spaces and tabs between markers are allowed
- Mixed marker types are NOT horizontal rules
- 4-space-indented rule markers are code blocks

### Integration: Block interaction
- Header followed by paragraph, separated by blank line
- Blockquote containing code blocks
- Code block terminated by EOB then new code block
- Horizontal rule between paragraphs
- Setext header `---` vs horizontal rule `---` disambiguation (text above = header, no text = hr)

### Integration: Options handling
- `:header_offset: 1` shifts header levels correctly
- `:enable_coderay: false` renders plain `<pre><code>`
- `:syntax_highlighter: null` renders plain `<pre><code>`
- `.options` files are parsed and applied correctly by the test harness
