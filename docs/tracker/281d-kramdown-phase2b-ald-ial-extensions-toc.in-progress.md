# Issue 281d: Kramdown parser Phase 2b - ALD, IAL, block extensions, TOC

## Problem

The remaining Phase 2b features are the attribute and extension system: ALD (attribute list definitions), enhanced IAL (inline attribute lists) with ALD references, block extensions (comment, nomarkdown, options), and TOC (table of contents) generation. These features interact heavily with the rest of the parser since they modify attributes on other elements and control parser behavior.

## Scope

### ALD - Attribute List Definitions (category 10)

- **Syntax**: `{:ref: .class #id key="value"}` defines a reusable attribute set
- **Multiple definitions**: later definitions for the same ref merge/override
- **Reference in IAL**: `{:ref}` in an IAL expands to the ALD's attributes
- **Supported attribute types**: `.class`, `#id`, `key="value"`, `key='value'`, references to other ALDs
- **Escaping**: `key='dfsd\}'` with escaped braces
- **Invalid keys**: `k ey=value` (space in key) is ignored
- **ALD lines produce no output**: they are consumed and don't render

### Enhanced IAL (category 11)

Phase 2a implemented basic IAL. This issue enhances it with:

- **ALD references in IAL**: `{:ref}` expands the named ALD
- **Shorthand**: `{:.cls1#id.cls2}` without spaces
- **Auto IDs + IAL override**: `{:#myid .cls}` on a header overrides auto-generated ID
- **IAL on block HTML**: `{:.cls}` before/after a `<div>` applies to the div
- **IAL on blockquotes**: `{:#id}` after a blockquote
- **Nested IAL**: IAL before and after the same element (both apply, merged)
- **Multiple consecutive IAL**: `{:.cls1}` then `{:.cls2}` both apply to next element
- **IAL on lists, code blocks, headers**: all block types accept IAL
- **Class merging**: multiple `.class` values are space-separated in the `class` attribute
- **Complex ALD resolution**: `{:id: #id key="valo"}` then `{:id: #other .myclass other}` -- later values override for `#id`, classes merge, and `other` is resolved as ALD reference

### Block Extensions (category 12)

- **Comment**: `{::comment}...{:/comment}` renders as `<!-- ... -->` HTML comment
- **Comment self-closing**: `{::comment this='is' .ignore /}` produces no output
- **Nomarkdown**: `{::nomarkdown}...{:/nomarkdown}` outputs content literally without markdown processing
- **Nomarkdown with type**: `{::nomarkdown type="html"}` only outputs for HTML target
- **Nomarkdown self-closing**: `{::nomarkdown ... /}` produces no output
- **Options**: `{::options key="value" /}` modifies parser options mid-document
  - `parse_block_html`, `parse_span_html`: enable markdown parsing in HTML
  - `footnote_nr`: starting number for footnotes
  - `syntax_highlighter_opts`: syntax highlighting configuration
  - `template`: ignored (security risk)
- **Unknown extensions**: `{::something}...{:/something}` are ignored (rendered as text)
- **Unclosed extensions**: `{::comment}` without `{:/comment}` at end of document is rendered as text

### TOC - Table of Contents (category 16)

- **Syntax**: `* list marker` followed by `{:toc}` IAL on the list
- **Replaces the list**: the list element becomes a nested `<ul>` TOC
- **Auto IDs required**: TOC needs `auto_ids: true` to generate `#header-slug` links
- **TOC structure**: nested `<ul>` / `<li>` / `<a href="#id">` matching header hierarchy
- **TOC IDs**: each link gets `id="markdown-toc-slug"`
- **`.no_toc` exclusion**: headers with `{:.no_toc}` class are excluded from TOC but still rendered
- **`toc_levels` option**: `toc_levels: 2..3` limits which header levels appear in TOC
- **Footnotes in headers**: footnote markers in TOC links are stripped, but appear in the header itself
- **Links in headers**: `[Header]` link reference in header text -- TOC shows plain text, header shows link
- **Duplicate headers**: second occurrence gets `-1` suffix on ID (`header-1`)
- **`no_toc` marker**: `* TOC text` followed by `{:toc}` -- the list is replaced, its text content is discarded

## Current test status

- **ALD (1 test):** 0 pass, 1 fail
- **IAL (3 tests):** 1 pass (`auto_id_and_ial`), 2 fail (`simple`, `nested`)
- **Extensions (6 tests):** 0 pass, 6 fail
- **TOC (5 tests):** 0 pass, 5 fail
- **Combined target:** 15 tests total, currently 1 passing, 14 failing

## Dependencies

- Issue #280 (Phase 2a) must be `.done.md` -- provides basic IAL, header parsing
- Issue #281a (Lists) must be `.done.md` -- TOC replaces a list element; extensions can appear in list items
- Issue #281c (HTML blocks) should be done -- IAL applies to HTML blocks, `parse_block_html`/`parse_span_html` options affect HTML block parsing

## Test Cases to Pass

### ALD (1 test)

| Test file | What it tests |
|-----------|---------------|
| `simple` | ALD definitions with classes, IDs, key-value pairs, escaping, invalid keys |

### IAL (3 tests)

| Test file | What it tests | Options |
|-----------|---------------|---------|
| `simple` | IAL on paragraphs, blockquotes, lists, code blocks, headers; ALD reference resolution; class merging; shorthand syntax | none |
| `auto_id_and_ial` | IAL overriding auto-generated header ID | `auto_ids: true` |
| `nested` | IAL before and after HTML blocks and blockquotes | none |

### Block Extensions (4 tests with valid expected output)

| Test file | What it tests |
|-----------|---------------|
| `comment` | Comment extension rendering as HTML comment, self-closing, unclosed at EOF |
| `nomarkdown` | Nomarkdown passthrough, with `type="html"`, self-closing, unclosed at EOF |
| `ignored` | Unknown extensions rendered as text |
| `options` | `parse_block_html`, `parse_span_html`, `footnote_nr` options, template ignored |

**Note:** `options2` and `options3` require footnote support and syntax highlighting respectively. These should be deferred if those features are not yet available.

### TOC (5 tests)

| Test file | What it tests | Options |
|-----------|---------------|---------|
| `no_toc` | `{:toc}` on list, list is removed, no TOC rendered (because `auto_ids` not set? Actually the no_toc test shows headers WITHOUT auto IDs -- no TOC generated) | none |
| `toc_exclude` | Full TOC generation, `.no_toc` exclusion | `auto_ids: true` |
| `toc_levels` | `toc_levels: 2..3` restricts header levels in TOC | `toc_levels: 2..3`, `auto_ids: true` |
| `toc_with_footnotes` | Footnote refs in headers stripped from TOC links | `auto_ids: true` |
| `toc_with_links` | Link refs in headers, duplicate header ID suffixing | `auto_ids: true`, `auto_id_stripping: true` |

**Note:** `toc_with_footnotes` requires footnote support. Defer if not available.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes all existing tests (no regressions)
- [ ] ALD `simple` conformance test passes
- [ ] All 3 IAL conformance tests pass
- [ ] `comment`, `nomarkdown`, and `ignored` extension tests pass
- [ ] `options` extension test passes for `parse_block_html` and `parse_span_html` toggling (footnote_nr part may be deferred)
- [ ] `no_toc` and `toc_exclude` TOC tests pass
- [ ] `toc_levels` test passes
- [ ] ALD definitions are stored and resolved when referenced in IAL
- [ ] IAL classes merge (space-separated), IDs override, key-value pairs merge
- [ ] `{:.cls1#id.cls2}` shorthand parsed correctly
- [ ] `{::comment}...{:/comment}` renders as `<!-- ... -->`
- [ ] `{::nomarkdown}...{:/nomarkdown}` passes content through without markdown processing
- [ ] `{::nomarkdown type="html"}` only outputs content for HTML converter
- [ ] Unknown extensions (`{::something}`) render as literal text
- [ ] Unclosed extensions at EOF render as literal text
- [ ] `{::options parse_block_html="true" /}` changes parser behavior for subsequent content
- [ ] TOC generates nested `<ul>` with `<a href="#slug">` links
- [ ] TOC respects `.no_toc` exclusion
- [ ] TOC respects `toc_levels` range restriction
- [ ] If `toc_with_footnotes`, `toc_with_links`, `options2`, or `options3` tests cannot pass, follow-up issues are created

## Test Scenarios

### Unit: ALD parsing
- `{:id: .class1}` creates ALD named "id" with class "class1"
- `{:id: #myid key="value"}` creates ALD with id and key-value
- `{:test: k ey=value}` -- space in key means the attribute is skipped/ignored
- `{:id: .class1}` then `{:id: .class2}` -- classes merge

### Unit: IAL resolution with ALD
- IAL `{:ref}` where ref has `.class #id` -- element gets both class and id
- IAL `{:.cls1 :ref .cls2}` -- all classes merge
- IAL `{:ref}` where ref references another ALD -- recursive resolution

### Unit: Extension parsing
- `{::comment}` starts comment block, `{:/comment}` or `{:/}` ends it
- `{::nomarkdown type="html"}` -- type attribute parsed
- `{::comment ... /}` self-closing (no body)
- `{::options key="value" /}` always self-closing

### Unit: TOC generation
- Given headers at levels 1, 2, 3, 4 -- TOC nests correctly
- Header with `.no_toc` class excluded from TOC
- `toc_levels: 2..3` -- only h2 and h3 in TOC
- Duplicate header text gets `-1` suffix on ID

### Integration: Full rendering
- Parse each test `.text` file and compare output to expected `.html`
- Parse `simple.text` (IAL) with ALD definitions, verify attributes on all element types
- Parse `toc_exclude.text` with `auto_ids: true`, verify full TOC output matches expected
- Parse `options.text`, verify `parse_block_html` toggle takes effect mid-document

## Log

### [SWE] 2026-03-21
- Implemented ALD (Attribute List Definitions) parsing and storage
  - `is_ald()` detects ALD lines `{:name: attrs}`
  - `parse_ald()` extracts name and attributes
  - ALDs stored in `AldMap` (HashMap), merged on redefinition
  - ALDs produce no visible output (test `block/10_ald/simple` passes)
- Enhanced IAL parsing with ALD reference resolution
  - Replaced parser.rs `parse_ial` with delegation to span_parser's version (handles shorthand, escapes)
  - Added `__ald_ref__` entries for bare words in IAL
  - `resolve_ald_refs()` recursively expands ALD references
  - Unresolved refs kept for special handling (e.g., `toc`)
  - `apply_attrs` skips `__ald_ref__` entries, detects `toc` reference
- Added paragraph attribute rendering in HTML converter
  - `convert_paragraph` now calls `write_attrs` on `<p>` tag
- Added HTML block IAL attribute injection
  - `inject_attrs_into_html()` merges IAL attributes into raw HTML opening tags
- Added TOC support
  - `collect_headers()` pre-scans document headers
  - `generate_toc()` produces nested `<ul>` with links and IDs
  - `{:toc}` IAL on list suppresses list, generates TOC (or removes if no auto_ids)
  - Header auto-ID generation via `generate_header_id()`
- Fixed trailing blank line preservation in `extract_definitions`
  - When definitions are removed from end of text, preceding blank lines are preserved
- Fixed list parser to break on IAL/ALD lines (not consume them as lazy continuation)
- Added `parse_span_html` to Options struct
- Tests: 592/651 kramdown tests passing (was 579), 0 regressions
- Clippy clean, fmt clean
- Files modified:
  - src/kramdown_parser/parser.rs (ALD, IAL, list parser fix)
  - src/kramdown_parser/html.rs (paragraph attrs, TOC, header auto-IDs, HTML attr injection)
  - src/kramdown_parser/span_parser.rs (ALD in extract_definitions, ald_ref in parse_ial, toc_headers)
  - src/kramdown_parser/options.rs (parse_span_html)
  - src/kramdown_parser/mod.rs (pass ALDs from extract_definitions to parser)

### Known limitations (need follow-up issues)
- IAL `simple` test: attribute ordering doesn't match kramdown (HashMap vs insertion order)
- IAL `nested`: pending IAL before HTML blocks needs attribute injection into raw HTML tags
- Extension `options`: `{::options}` doesn't modify parser behavior mid-document (needs mutable options threading)
- Extension `options2`, `options3`: require footnote support / syntax highlighting integration
- TOC `toc_exclude`: attribute ordering on headers, TOC nesting indentation off by spaces
- TOC `toc_levels`, `toc_with_footnotes`, `toc_with_links`: need refined TOC generation
