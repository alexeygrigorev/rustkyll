# Issue 281: Kramdown parser Phase 2b - Complex block elements

## Problem

After Phase 2a implements core blocks, we need the more complex block structures: lists, tables, HTML blocks, definition lists, and block-level extensions.

## Scope

Implement these block element types:
- **List** / **ListItem** — ordered and unordered lists, nesting, mixed content
- **Table** / **TableRow** / **TableCell** — kramdown pipe tables with alignment
- **HtmlBlock** — raw HTML blocks passed through
- **DefinitionList** / **DefinitionTerm** / **DefinitionDefinition** — kramdown definition lists
- **MathBlock** — `$$...$$` display math
- **Toc** — `{:toc}` table of contents generation
- **ALD** (attribute list definition) — `{:ref: .class #id}` definitions
- **IAL** (inline attribute list) — `{: .class #id}` on elements
- **BlockExtension** — `{::comment}...{:/comment}`, `{::nomarkdown}...{:/nomarkdown}`, `{::options}...{:/options}`

## Dependencies

Depends on Issue #280 (Phase 2a) being complete.

## Test cases to pass

All `.text`/`.html` pairs in:
- `block/08_list/`
- `block/09_html/`
- `block/10_ald/`
- `block/11_ial/`
- `block/12_extension/`
- `block/13_definition_list/`
- `block/14_table/`
- `block/15_math/`
- `block/16_toc/`
- `block/04_header/` deferred tests (auto-IDs, header links)

## Acceptance Criteria

- [ ] `cargo build` compiles
- [ ] `cargo test` passes
- [ ] Conformance tests for listed categories pass
- [ ] Lists handle nesting, lazy continuation, mixed ordered/unordered
- [ ] Tables handle alignment, header/footer, escaping
- [ ] IAL/ALD attributes applied correctly to elements
- [ ] No regressions
