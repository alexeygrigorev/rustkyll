# Issue 503: Kramdown footnote syntax not rendered in just-the-docs

## Problem

Kramdown footnote syntax (`[^1]` inline references and `[^1]: footnote text` definitions) is rendered as literal text instead of being converted to `<sup>` footnote links and a footnotes section at the bottom of the page.

### Example

**Jekyll** (correct):
```html
<p>...templating language, and HTML.<sup><a href="#fn:1">1</a></sup></p>
...
<div class="footnotes"><ol><li id="fn:1"><p>footnote text</p></li></ol></div>
```

**Rustkyll** (broken):
```html
<p>...templating language, and HTML.[^1] Jekyll builds...</p>
```

### Affected Pages in just-the-docs

- index.html -- `[^1]`, `[^2]` footnotes
- docs/navigation/main/collections/index.html -- `[^1]` footnote
- docs/ui-components/callouts/index.html -- `[^postfix]` footnote

## Root Cause

The markdown parser (likely pulldown-cmark) does not support kramdown footnote syntax natively. Jekyll uses kramdown which has built-in footnote support. Rustkyll needs to either:
1. Enable footnote support in pulldown-cmark (it has a footnotes extension), or
2. Pre-process footnote syntax before markdown parsing

## Dependencies

None.

## Baseline

- just-the-docs: 1/47 (or higher if #501/#502 are fixed first)
- DTC: 790/790 (must not regress)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] `[^1]` in body text is converted to `<sup>` with link to footnote
- [ ] `[^1]: text` definitions are collected into a `<div class="footnotes">` section
- [ ] Named footnotes like `[^postfix]` work correctly
- [ ] DTC DOM baseline remains at 790/790
- [ ] No regression in other sites' markdown rendering

## Test Scenarios

### Unit: Footnote parsing
- Markdown with `text[^1]\n\n[^1]: footnote` -- verify `<sup>` generated
- Markdown with named footnote `[^note]` -- verify correct linking
- Markdown with multiple footnotes -- verify numbering and ordering
- Markdown with no footnotes -- verify no change in output

### Integration: just-the-docs pages
- Build just-the-docs, check index.html has `<sup>` elements for footnotes
- Verify footnote section appears at bottom of page content
