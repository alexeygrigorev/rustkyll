# Issue 281c: Kramdown parser Phase 2b - HTML blocks, definition lists, math blocks

## Problem

Several block element types remain unimplemented: HTML blocks (raw HTML passed through or parsed), definition lists (kramdown-specific), and math blocks (`$$...$$`). These are medium-complexity features that share no deep dependencies with each other but are all needed for full kramdown compatibility.

## Scope

### HTML Blocks (category 09)

- **Raw HTML blocks**: `<div>`, `<p>`, `<table>`, etc. detected as block-level HTML
- **parse_block_html option**: when true, block HTML content is parsed as kramdown
- **markdown attribute**: `markdown="block"`, `markdown="span"`, `markdown="1"`, `markdown="0"` on HTML tags controls how content is parsed
- **HTML comments**: `<!-- ... -->` passed through
- **Script/style tags**: content not parsed as markdown
- **CDATA sections**: `<![CDATA[...]]>` handling
- **Processing instructions**: `<?...?>` treated as text
- **Invalid HTML**: closing tags without openers (`</div>`), self-closing tags (`<hr>`)
- **HTML after blocks**: HTML blocks following paragraphs and blockquotes
- **Nested block HTML**: `<div>` inside `<div>` with markdown parsing
- **XML namespaced tags**: `<some:url>` etc.
- **textarea**: block-level textarea elements
- **HTML5 boolean attributes**: `<p class>` (valueless attributes)
- **html_to_native option**: converts HTML elements (`<p>`, `<em>`, `<strong>`, `<code>`, `<h1>`-`<h6>`, `<ul>`, `<ol>`, `<dl>`, `<table>`, etc.) to native kramdown elements for markdown processing
- **Content model handling**: how block HTML interacts with definition lists and tables when nested

### Definition Lists (category 13)

- **Basic syntax**: term on one line, `: definition` on next
- **Multiple terms**: several terms before definitions
- **Multiple definitions**: several `: def` lines per term
- **Block content in definitions**: blockquotes, code blocks, headers, nested definition lists, lists
- **Para wrapping**: definitions separated by blank lines get `<p>` wrapping
- **Item IAL**: `{:.cls}` on definition items and terms
- **Definition list IAL**: `{:.cls}` on the whole `<dl>`
- **Auto IDs**: `{:auto_ids}` and `{:auto_ids-prefix-}` on definition lists
- **EOB separation**: `^` between definition lists creates separate `<dl>` elements
- **Edge cases**: `: ` at beginning of file, too much space between term and definition, escaped `\:`

### Math Blocks (category 15)

- **Display math**: `$$...$$` on its own line(s) becomes `\[...\]`
- **Inline math at block level**: `$$ expr $$` on a line with text before/after becomes `\(...\)` inline
- **Multi-line math**: `$$\begin{align*}...\end{align*}$$`
- **Math with IAL**: `{:.cls}` before math block wraps in `<div>`
- **No engine option**: `math_engine: ~` renders as `<div class="kdmath">$$...$$</div>`
- **XSS prevention**: HTML tags inside math are escaped
- **Indentation**: 4+ spaces means code block, not math

## Current test status

- **HTML blocks (29 total):** 0 pass, 29 fail (17 root + 12 html_to_native + 2 content_model -- note: 2 root tests `invalid_html_1` and `processing_instruction` may already pass)
- **Definition lists (12 total):** 3 pass (definition_at_beginning, no_def_list, too_much_space), 9 fail
- **Math blocks (3 total):** 0 pass, 3 fail
- **Combined target:** 44 tests total, currently ~3 passing, ~41 failing

## Dependencies

- Issue #280 (Phase 2a) must be `.done.md`
- Issue #281a (Lists) must be `.done.md` -- definition list `with_blocks` test has list content in definitions, and HTML `markdown_attr` test has `<dd markdown="1">`
- Issue #281b (Tables) must be `.done.md` -- content_model/tables test requires table support

## Test Cases to Pass

### HTML Blocks - Root tests (17 tests with .html files)

| Test file | What it tests | Options |
|-----------|---------------|---------|
| `simple` | Basic block HTML, nested divs, markdown parsing in blocks, inline elements, iframe | `parse_block_html: true` |
| `comment` | HTML comments passed through, comments in blockquotes | none |
| `html_after_block` | HTML div after paragraph and blockquote | none |
| `html_and_codeblocks` | Code blocks vs HTML blocks, indented HTML in code blocks | `parse_block_html: true` |
| `html_and_headers` | Setext header vs content inside div | none |
| `invalid_html_1` | Closing tag without opener (`</div>`) | none |
| `invalid_html_2` | Self-closing `<hr>` tag | none |
| `not_parsed` | Block HTML without parse_block_html -- content not parsed | none |
| `parse_as_raw` | Script/style tags, content preserved literally | `parse_block_html: true` |
| `parse_as_span` | `<p>` content parsed as span-level markdown | `parse_block_html: true` |
| `parse_block_html` | `<DIV>` parsed with markdown, nested divs, code blocks in divs | `parse_block_html: true` |
| `markdown_attr` | `markdown="block"`, `markdown="span"`, `markdown="1"`, `markdown="0"` | none |
| `cdata_section` | CDATA in block and inline contexts | none |
| `processing_instruction` | `<?...?>` processing instructions | none |
| `html5_attributes` | Boolean attributes, single/double quotes, unquoted values | none |
| `textarea` | Textarea as inline and block element | none |
| `xml` | XML namespaced tags | none |

**Note:** `standalone_image_in_div` and `table` have no `.html` reference files -- skip them.

### HTML Blocks - html_to_native subdirectory (12 tests)

These tests cover the `html_to_native` option which converts HTML elements to native kramdown elements for processing:

| Test file | What it tests | Options |
|-----------|---------------|---------|
| `code` | `<code>` HTML converted to native code span | `html_to_native: true` |
| `comment` | HTML comments with html_to_native | `html_to_native: true` |
| `emphasis` | `<em>`/`<strong>` HTML converted to native emphasis | `html_to_native: true` |
| `entity` | HTML entities with html_to_native | `html_to_native: true` |
| `header` | `<h1>`-`<h6>` HTML converted to native headers | `html_to_native: true` |
| `list_dl` | `<dl>` HTML converted to native definition list | `html_to_native: true` |
| `list_ol` | `<ol>` HTML converted to native ordered list | `html_to_native: true` |
| `list_ul` | `<ul>` HTML converted to native unordered list | `html_to_native: true` |
| `paragraph` | `<p>` HTML converted to native paragraph | `html_to_native: true` |
| `table_normal` | `<table>` HTML converted to native table | `html_to_native: true` |
| `table_simple` | Simple table HTML converted to native table | `html_to_native: true` |
| `typography` | Typographic HTML entities with html_to_native | `html_to_native: true` |

### HTML Blocks - content_model subdirectory (2 tests)

These tests cover HTML content model handling (how block HTML interacts with definition lists and tables):

| Test file | What it tests | Options |
|-----------|---------------|---------|
| `deflists` | Definition lists inside/around block HTML | none |
| `tables` | Tables inside/around block HTML | none |

### Definition Lists (12 tests)

| Test file | What it tests |
|-----------|---------------|
| `simple` | Basic term/definition, empty definition, continuation |
| `multiple_terms` | Multiple terms before definitions, styled terms |
| `definition_at_beginning` | `: ` at start of file is not a definition |
| `no_def_list` | Escaped `: ` does not create definition list |
| `too_much_space` | Two blank lines between term and definition breaks it |
| `para_wrapping` | Blank lines between definitions trigger `<p>` wrapping |
| `separated_by_eob` | `^` EOB creates separate `<dl>` elements |
| `with_blocks` | Block content in definitions: blockquotes, code, headers, nested deflists, lists |
| `styled_terms` | Inline markup in terms (`*kram*`) |
| `item_ial` | IAL on definitions and terms |
| `deflist_ial` | IAL on the whole definition list |
| `auto_ids` | Auto-generated IDs on terms, with prefix |

### Math Blocks (3 tests)

| Test file | What it tests | Options |
|-----------|---------------|---------|
| `normal` | Display math, inline math at block level, multi-line, indentation check, IAL | none |
| `no_engine` | `math_engine: ~` renders raw TeX in div | `math_engine: ~` |
| `gh_128` | Script tags inside math are HTML-escaped | none |

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes all existing tests (no regressions)
- [ ] At least 12 of 17 HTML block root conformance tests pass (the 5 that require `parse_block_html: true` with full markdown-in-HTML support may be deferred if too complex)
- [ ] At least 8 of 12 html_to_native conformance tests pass (complex ones like table_normal may be deferred)
- [ ] Both content_model tests pass (deflists, tables)
- [ ] All 9 currently-failing definition list conformance tests pass (simple, multiple_terms, para_wrapping, separated_by_eob, with_blocks, styled_terms, item_ial, deflist_ial, auto_ids)
- [ ] 3 definition list tests that already pass remain passing (definition_at_beginning, no_def_list, too_much_space)
- [ ] All 3 math block conformance tests pass
- [ ] HTML comments (`<!-- ... -->`) pass through unchanged
- [ ] Block-level `<div>` elements are detected and not parsed as markdown (default mode)
- [ ] `parse_block_html: true` enables markdown parsing inside block HTML
- [ ] `markdown="block|span|1|0"` attribute controls parsing per-element
- [ ] Script/style tag content is never parsed as markdown
- [ ] Invalid HTML (orphan closing tags) renders as escaped text
- [ ] Definition lists use `<dl>` / `<dt>` / `<dd>` structure
- [ ] Multiple terms and definitions per entry are supported
- [ ] Block content inside definitions (blockquotes, code, headers, lists) renders correctly
- [ ] Math blocks render as `\[...\]` for display math
- [ ] Inline math at block boundaries renders as `\(...\)`
- [ ] Math content is HTML-escaped (prevents XSS)
- [ ] If any HTML block tests requiring deep markdown-in-HTML parsing are deferred, follow-up issues are created
- [ ] If any html_to_native tests are deferred, follow-up issues are created

## Test Scenarios

### Unit: HTML block detection
- `<div>` at start of line detected as block HTML start
- `<script>` detected as raw (never parsed) block
- `<!-- comment -->` detected as HTML comment
- `</div>` without opener detected as invalid HTML
- `<p>` with `markdown="1"` triggers markdown parsing of content

### Unit: Definition list detection
- Line starting with `: ` after a term line starts a definition
- Line starting with `: ` at beginning of document is NOT a definition
- `\: ` escaped colon is NOT a definition start
- Two blank lines between term and `: ` breaks the definition list

### Unit: Math block detection
- `$$expr$$` on its own line is display math
- `$$expr$$` with text before it is inline math
- `    $$expr$$` (4+ spaces) is a code block, not math

### Integration: Full rendering
- Parse each test `.text` file and compare output to expected `.html`
- Parse `simple.text` (HTML blocks) with `parse_block_html: true`, verify nested markdown parsing
- Parse `with_blocks.text` (definition lists), verify blockquotes, code, nested deflists in definitions
- Parse `normal.text` (math), verify `\[...\]` and `\(...\)` output

## Log

### [SWE] 2026-03-20

**Implementation Summary:**

1. **HTML Blocks (parser.rs, html.rs)**:
   - Processing instructions (`<?...?>`) no longer treated as HTML blocks (paragraph text instead)
   - Closing tags without openers (`</div>`) no longer treated as HTML blocks
   - Void elements (`<hr>`, `<br>`) now output as self-closing (`<hr />`)
   - HTML block parsing with proper tag nesting (tracks open/close tags)
   - Attribute normalization: single quotes -> double quotes, boolean attrs -> `=""`, unquoted values -> quoted
   - XML namespaced tags (`<some:url>`, `<webgen:block>`) treated as block HTML with case preservation
   - Raw tags (`<script>`, `<style>`) content preserved literally
   - `<textarea>` blocks span across blank lines
   - HTML comments pass through correctly
   - Invalid HTML inside blocks gets escaped (`</p>` orphan -> `&lt;/p&gt;`)
   - HTML block tags interrupt paragraphs and blockquote lazy continuation
   - `parse_block_html: true` enables markdown parsing inside block HTML (block/span modes)
   - `markdown="block|span|1|0"` attribute controls per-element parsing

2. **Definition Lists (parser.rs, html.rs)**:
   - Complete rewrite of definition list parser
   - Multiple terms before definitions
   - Para wrapping when blank line precedes definition
   - Block content in definitions: blockquotes, code blocks, headers, nested deflists, lists
   - EOB (`^`) separates definition lists
   - Item IAL (`{:.cls}`) on definitions and terms
   - Definition list IAL (`{:.dl-horizontal}`) on the `<dl>` element
   - Auto IDs (`{:auto_ids}` and `{:auto_ids-prefix-}`)
   - Proper indent handling for continuation lines and nested content

3. **Math Blocks (parser.rs)**:
   - Inline math detection: `$$ expr $$` within paragraph text becomes `\(...\)`
   - Display math: standalone `$$...$$` becomes `\[...\]`
   - No-engine mode: `math_engine: ~` renders as `<div class="kdmath">$$...$$</div>`
   - XSS prevention: HTML tags inside math are escaped

**Test Results:**
- block_09 (HTML): 12/31 passing (up from 4/31) -- +8 new passes
- block_13 (definition lists): 12/12 passing (up from 3/12) -- ALL PASS
- block_15 (math): 3/3 passing (up from 0/3) -- ALL PASS
- Full suite: 2217 passed, 56 failed, 15 ignored (no regressions)
- Clippy: clean in modified files
- Fmt: clean

**Remaining block_09 failures (19 tests):**
- 5 parse_block_html tests: require deep markdown-in-HTML parsing with proper indentation
- 9 html_to_native tests: require converting HTML elements to native kramdown elements
- 2 content_model tests: require content model awareness
- 1 html5_attributes test: attribute normalization edge cases
- 1 cdata test: CDATA content stripping
- 1 xml test: case-sensitive tag matching for non-standard tags

**Files modified:**
- `src/kramdown_parser/parser.rs` - HTML block parsing, definition list parser rewrite, math block inline detection
- `src/kramdown_parser/html.rs` - HTML block converter with parse modes, definition list converter with para wrap/block content/auto_ids, invalid HTML escaping
- `docs/tracker/281c-kramdown-phase2b-html-blocks.in-progress.md` - this issue file
