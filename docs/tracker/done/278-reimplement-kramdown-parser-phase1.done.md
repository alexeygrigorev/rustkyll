# Issue 278: Reimplement kramdown parser in Rust -- Phase 1: Scaffold and Test Harness

## Problem

We're currently making pulldown-cmark (CommonMark parser) produce kramdown output through increasingly complex postprocessing in `src/kramdown.rs`. Every fix introduces edge cases because the two parsers have fundamentally different behavior:

- Pipe table detection rules differ
- Smart quote direction algorithms differ
- Emphasis delimiter handling differs (mixed `_`/`*`)
- Math handling differs
- HTML comment wrapping differs
- Typographic symbol conversion differs

DTC is stuck at 543/787 (69%) DOM match despite 15+ postprocessing fixes. The remaining 244 pages fail because of fundamental parser differences that can't be patched.

## Solution

Reimplement kramdown's parser and HTML converter natively in Rust. kramdown is MIT licensed (Thomas Leitner, 2009-2013), so this is legally straightforward.

This is a multi-phase effort. This issue covers **Phase 1 only**: scaffold the module, define core types, create stubs, and wire up all 198 conformance test cases as runnable (initially failing) tests.

## Phased Plan (Full Roadmap)

- **Phase 1 (this issue):** Scaffold -- `mod.rs`, element types, options, parser stub, HTML converter stub. Wire into `lib.rs`. Make all 198 kramdown test cases runnable (read `.text`, parse, compare to `.html`). Most tests will fail; that is expected.
- **Phase 2 (future issue):** Block elements -- paragraph, header, blockquote, code block, horizontal rule, lists, tables, HTML blocks. Each block type should pass its corresponding test cases.
- **Phase 3 (future issue):** Span elements -- emphasis, strong, links, images, code spans, line breaks, smart quotes, typographic symbols, HTML entities.
- **Phase 4 (future issue):** Integration -- wire into the main build pipeline as an alternative to pulldown-cmark. Run DTC site through it, measure DOM match.

## Scope (Phase 1)

### Must deliver

1. **Module structure:** `src/kramdown_parser/mod.rs` with submodules:
   - `element.rs` -- AST node types (`Element` enum, `ElementType` enum, `Document` struct)
   - `options.rs` -- Parser options struct (fields for all kramdown options found in `.options` test files; defaults matching kramdown 2.5.2)
   - `parser.rs` -- `KramdownParser` struct with `pub fn parse(input: &str, options: &Options) -> Document` that returns a minimal document (e.g., single root element wrapping input as raw text)
   - `html.rs` -- `HtmlConverter` struct with `pub fn convert(doc: &Document, options: &Options) -> String` that produces placeholder HTML (e.g., wraps raw text in `<p>` tags or returns empty string)
   - `entities.rs` -- HTML entity lookup (can use a crate like `htmlize` or a static table; must resolve numeric, named, and symbolic entities)

2. **Wire into `lib.rs`:** Add `pub mod kramdown_parser;` so the module compiles as part of the crate.

3. **Test harness:** A test module (in `src/kramdown_parser/mod.rs` or a separate `tests.rs`) that:
   - Discovers all 198 test case pairs (`.text` + `.html`) under `src/kramdown_parser/testcases/`
   - For each pair: reads the `.text` file, reads the corresponding `.options` file if present, parses it through `KramdownParser::parse`, converts through `HtmlConverter::convert`, compares output to the `.html` expected output
   - Each test case is a separate `#[test]` function (generated via macro or explicit listing) so failures are individually identifiable
   - Skips the 18 `.text` files that have no corresponding `.html` (man pages, minted, LaTeX-only, etc.) -- these are not HTML conformance tests
   - Tests that fail due to unimplemented features should be marked with `#[ignore]` and a comment indicating which phase will address them -- BUT there must be a tracking mechanism: a summary test that counts ignored vs passing vs total

4. **Options parsing:** The `.options` files are YAML with Ruby-style symbols (`:key: value`). The options parser must handle at least these keys found in the test suite:
   - `auto_ids`, `auto_id_prefix`, `auto_id_stripping`, `transliterated_header_ids`
   - `entity_output` (values: `:as_char`, `:as_input`, `:symbolic`, `:numeric`)
   - `footnote_nr`, `footnote_prefix`, `footnote_backlink`, `footnote_backlink_inline`, `footnote_link_text`
   - `header_offset`, `header_links`
   - `math_engine`
   - `parse_block_html`, `html_to_native`
   - `smart_quotes`, `typographic_symbols`
   - `syntax_highlighter`, `syntax_highlighter_opts`, `enable_coderay`, `coderay_*`
   - `link_defs`
   - `toc_levels`, `remove_line_breaks_for_cjk`
   Fields can start as defaults; they just need to exist in the struct so tests can set them.

5. **Attribution:** `src/kramdown_parser/mod.rs` must have a module-level doc comment with kramdown MIT license attribution (Thomas Leitner) and MDTest attribution (Michel Fortin). `LICENSE-kramdown` already exists at `src/kramdown_parser/LICENSE-kramdown`.

### Must NOT do

- Implement actual parsing logic (that is Phase 2/3)
- Wire into the main site generation pipeline (that is Phase 4)
- Implement syntax highlighting (rouge/coderay/minted)
- Change any behavior of the existing `src/kramdown.rs` postprocessor

## Architecture

```
src/kramdown_parser/
    mod.rs          -- public API, re-exports, attribution doc comment
    element.rs      -- Element, ElementType, Document, Attr types
    options.rs      -- Options struct, defaults, .options file parser
    parser.rs       -- KramdownParser::parse() stub
    html.rs         -- HtmlConverter::convert() stub
    entities.rs     -- HTML entity resolution
    testcases/      -- (already exists) 216 .text, 199 .html, .options files
    LICENSE-kramdown -- (already exists) MIT license text
```

## Element Types

The `ElementType` enum must cover all kramdown element categories. Reference: kramdown 2.5.2 `element.rb`. At minimum:

### Block elements
`Root`, `Blank`, `Paragraph`, `Header`, `Blockquote`, `CodeBlock`, `HorizontalRule`, `List`, `ListItem`, `Table`, `TableRow`, `TableCell`, `HtmlBlock`, `DefinitionList`, `DefinitionTerm`, `DefinitionDefinition`, `MathBlock`, `Toc`, `Eob`, `BlockExtension`

### Span elements
`Text`, `Emphasis`, `Strong`, `Link`, `Image`, `CodeSpan`, `LineBreak`, `SmartQuote`, `TypedSymbol`, `HtmlSpan`, `FootnoteRef`, `FootnoteMarker`, `Abbreviation`, `MathInline`, `SpanExtension`, `EscapedChar`

### Attribute types
`Attr` -- a map of `String -> String` for HTML attributes (id, class, etc.), plus support for IAL (inline attribute lists).

## Dependencies

- No other issues need to be `.done.md` first. This is a new module with no dependencies on in-progress work.

## Acceptance Criteria

- [ ] `./scripts/cargo-safe build` compiles without errors with the new `kramdown_parser` module
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes clean
- [ ] `cargo fmt --check` passes clean
- [ ] `src/kramdown_parser/mod.rs` exists and is declared in `src/lib.rs` as `pub mod kramdown_parser;`
- [ ] `Element` and `ElementType` enums exist with all block and span variants listed above
- [ ] `Document` struct exists wrapping a root `Element` with children
- [ ] `Options` struct exists with fields for all option keys listed above, with sensible defaults
- [ ] `Options` can be loaded from a `.options` YAML file (handles Ruby-style `:symbol` keys)
- [ ] `KramdownParser::parse()` accepts `&str` + `&Options` and returns a `Document`
- [ ] `HtmlConverter::convert()` accepts `&Document` + `&Options` and returns a `String`
- [ ] All 198 test case pairs are runnable as individual `#[test]` functions
- [ ] Each test reads the `.text` input, parses it, converts to HTML, and compares against the `.html` expected output
- [ ] Tests that require `.options` files load and apply those options
- [ ] A summary test exists that prints/asserts the count of passing vs total test cases (e.g., "kramdown conformance: 5/198 passing")
- [ ] The summary test does NOT `#[ignore]` -- it always runs and reports the current pass rate
- [ ] No existing tests are broken (`./scripts/cargo-safe test` still passes all previously-passing tests)
- [ ] Attribution comment in `mod.rs` references Thomas Leitner (kramdown MIT) and Michel Fortin (MDTest)

## Test Scenarios

### Unit: Element types
- Create an `Element` of each variant, verify it can be constructed
- Create a `Document` with nested elements, verify tree structure
- Verify `ElementType` variants include all block and span types listed above

### Unit: Options
- Load default options, verify `auto_ids` defaults to `false`, `entity_output` defaults to something reasonable
- Parse a `.options` file with `:auto_ids: true`, verify the field is set
- Parse a `.options` file with `:entity_output: :symbolic`, verify the enum variant is correct
- Parse an `.options` file with nested YAML (e.g., `syntax_highlighter_opts` with `block`/`span` subkeys)
- Handle missing `.options` file gracefully (use defaults)

### Unit: Parser stub
- Call `KramdownParser::parse("hello world", &Options::default())` -- returns a `Document` (content of Document is not important yet, just that it returns without panicking)
- Call `KramdownParser::parse("", &Options::default())` -- returns a `Document` with empty/root element

### Unit: HTML converter stub
- Call `HtmlConverter::convert(&doc, &Options::default())` on a stub document -- returns a `String` (content not important yet)

### Integration: Conformance test harness
- All 198 `.text`/`.html` pairs are discovered and have corresponding test functions
- Running `./scripts/cargo-safe test kramdown` finds and runs the test harness
- Each individual test case is identifiable by name in test output (e.g., `test block_03_paragraph_no_newline_at_end`)
- Tests that fail show the diff between expected and actual HTML output (at least first N characters)
- The summary test reports a number like "kramdown conformance: X/198 passing" and does not panic (it is informational at this phase)
- The 18 `.text` files without `.html` counterparts are excluded from the test harness (they are man/minted/LaTeX tests, not HTML conformance)

### Build: Compilation
- `./scripts/cargo-safe build` succeeds
- `./scripts/cargo-safe clippy -- -D warnings` is clean
- `cargo fmt --check` passes

## Notes for the SWE

- The test cases live at `src/kramdown_parser/testcases/`. The directory structure is: `block/NN_name/test_name.{text,html,options}` and `span/NN_name/test_name.{text,html,options}`, plus two top-level pairs (`cjk-line-break`, `encoding`).
- There are exactly 198 `.text` files that have matching `.html` files. 18 `.text` files do not have `.html` counterparts (man pages, minted tests, LaTeX-only tests, a kramdown round-trip test) -- skip those.
- The `.options` files use Ruby YAML syntax with colon-prefixed symbol keys (`:auto_ids: true`). Strip the leading `:` when parsing.
- For the test macro/generator: consider using a macro that generates one `#[test]` per file pair, or use `include!` with a build script. Either approach is fine as long as each test case is individually runnable.
- The parser stub can return a Document containing a single `Text` element with the raw input. The HTML converter stub can just output that text wrapped in `<p>` tags or return it raw. The point is that the pipeline compiles and runs end-to-end.
- Do NOT use `#[ignore]` on the individual conformance tests. Let them fail. The summary test counts passing vs failing so we can track progress across phases. Failed tests are fine -- they show what Phase 2/3 need to implement.
- Keep this module isolated. Do not modify `src/kramdown.rs` or any other existing module except adding the `pub mod` line to `lib.rs`.

## Log

### [SWE] 2026-03-20
- Created module scaffold: element.rs, options.rs, parser.rs, html.rs, entities.rs, mod.rs, tests.rs
- Wired into src/lib.rs as `pub mod kramdown_parser;`
- Wrote TDD-style: unit tests first, then fixed options parser bugs (Ruby-style `:key:` parsing, unquoting)
- Options parser handles Ruby-style `:symbol` keys, nested YAML blocks, entity_output, link_defs, etc.
- All 198 conformance test pairs discovered and wired as individual #[test] functions via macro
- Summary test: kramdown conformance: 0/198 passing (198 failing) -- expected for stub phase
- Discovery test verifies all 198 .text/.html pairs on disk match ALL_TEST_STEMS list
- 18 .text files without .html (man pages, minted, LaTeX) correctly excluded
- Build: 2101 passed, 198 failed (all conformance), 0 ignored. All existing tests still pass.
- cargo fmt --check: clean
- clippy: our code is clean; vendor/liquid-core has pre-existing warnings (not our issue)
- Files created: src/kramdown_parser/{mod.rs, element.rs, options.rs, parser.rs, html.rs, entities.rs, tests.rs}
- Files modified: src/lib.rs (added `pub mod kramdown_parser;`)
