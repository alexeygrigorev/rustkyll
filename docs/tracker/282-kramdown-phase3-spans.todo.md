# Issue 282: Kramdown parser Phase 3 - Span elements

## Problem

After block parsing is complete, we need inline/span element parsing within text content.

## Scope

Implement all span element types:
- **Emphasis** / **Strong** — `*`, `_`, `**`, `__` with kramdown nesting rules
- **Link** — inline `[text](url)` and reference `[text][ref]` links
- **Image** — `![alt](url)` and reference images
- **CodeSpan** — backtick `` `code` `` spans
- **LineBreak** — trailing spaces or backslash line breaks
- **SmartQuote** — `"`, `'` smart typography
- **TypedSymbol** — `---` em-dash, `--` en-dash, `...` ellipsis
- **HtmlSpan** — inline HTML tags
- **FootnoteRef** / **FootnoteMarker** — `[^name]` footnotes
- **Abbreviation** — `*[abbr]: definition` abbreviations
- **MathInline** — `$$...$$` inline math
- **SpanExtension** — inline extensions
- **EscapedChar** — `\X` escaped characters
- **Autolinks** — `<url>` automatic links

## Dependencies

Depends on Issue #281 (Phase 2b) being complete.

## Test cases to pass

All `.text`/`.html` pairs in:
- `span/01_link/`
- `span/02_emphasis/`
- `span/03_codespan/`
- `span/04_footnote/`
- `span/05_html/`
- `span/abbreviations/`
- `span/autolinks/`
- `span/escaped_chars/`
- `span/extension/`
- `span/ial/`
- `span/line_breaks/`
- `span/math/`
- `span/text_substitutions/`
- Root: `cjk-line-break`, `encoding`

## Acceptance Criteria

- [ ] `cargo build` compiles
- [ ] `cargo test` passes
- [ ] All 216 conformance test cases pass (or document specific intentional deviations)
- [ ] Emphasis handles kramdown's nesting rules (not CommonMark rules)
- [ ] Smart quotes produce correct Unicode characters
- [ ] Footnotes collected and rendered at document end
- [ ] No regressions
