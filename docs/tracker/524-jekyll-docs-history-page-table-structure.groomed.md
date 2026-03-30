# Issue 524: jekyll-docs history page massive structural mismatch (3357 diffs)

## Problem

The `docs/history/index.html` page has 3357 DOM differences -- the largest single
page diff in jekyll-docs. The page structure is completely wrong: elements that
should be `<h3>` appear as `<tbody>`, `<ul>` appears as `<p>`, etc. The content
structure is shifted/offset, causing cascading mismatches for the entire page.

### Root pattern

The history page uses a Liquid-generated changelog with version headings and
bullet lists. A structural error early in the page (likely a table that is
improperly parsed, or a markdown structure that is incorrectly interpreted as a
table) causes all subsequent elements to be offset, producing cascading
differences.

### Specific diffs (from original issue -- actual count is 3357)

```
body > article > ul > li: expected_text_got_element - expected: 'Document keys of global variable', actual: '<table>'
body > article > ul > li > child[1]: tag_name_differs - expected: 'code', actual: 'table'
body > article > child[15]: tag_name_differs - expected: 'h3', actual: 'tbody'
body > article > child[16]: tag_name_differs - expected: 'ul', actual: 'p'
body > article > child[17]: tag_name_differs - expected: 'h2', actual: 'h3'
... and 3347+ more differences
```

A `<table>` element appears where `<code>` is expected in a list item. This
suggests a markdown table-like pattern inside a list item is being parsed as a
real HTML table instead of inline code/text.

## Root Cause

The history page source is at `websites/jekyll-docs/docs/_docs/history.md`. It
contains hundreds of changelog entries with headings (`## 4.4.1 / 2025-01-29`)
and bullet lists. The markdown content uses kramdown IAL syntax like `{: #v4-4-1}`.

The root cause is likely one of:

1. A changelog entry contains a line that looks like a markdown table separator
   (e.g., `| --- |` or similar pattern) but is actually part of a list item.
   The kramdown parser incorrectly starts a table, which shifts all subsequent
   elements.

2. A Liquid template variable expansion (`{{ site.repository }}`) produces
   content that confuses the markdown table parser.

3. The kramdown parser's handling of `{: #id}` IAL on heading lines interacts
   badly with the content that follows.

Investigation steps for the engineer:
- Build jekyll-docs and inspect the generated `docs/history/index.html`
- Find where the first `<table>` element appears in rustkyll output that is NOT
  in the Jekyll reference
- Trace back to the corresponding markdown source to identify the triggering pattern
- Compare rustkyll's intermediate markdown AST for that section vs what kramdown produces

The fix likely goes in `src/kramdown_parser/parser.rs` (table detection logic)
or `src/kramdown.rs` (the high-level markdown processing).

## Scope

Investigate the history page's markdown source, identify the structural parsing
error, and fix it. This is likely related to how markdown tables or code blocks
inside list items are handled, or how certain patterns are incorrectly recognized
as table starts.

NOTE: Fixing this single page could resolve ~3357 of the ~15,254 total jekyll-docs
diffs (~22% of all diffs).

## Dependencies

None.

## DTC DOM Baseline

- Current: 790/790 (DTC), jekyll-docs 14/125 matched, 15254 total diffs
- Must not drop below: 790/790 (DTC)
- Target: docs/history/index.html diff count drops from 3357 to under 200

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes clean
- [ ] `cargo fmt` produces no changes
- [ ] docs/history/index.html structure matches Jekyll output (h2, h3, ul, li hierarchy)
- [ ] No spurious `<table>` or `<tbody>` elements in the history page where Jekyll has none
- [ ] Version headings render as `<h2>` / `<h3>` (not shifted elements)
- [ ] Changelog bullet lists render as `<ul><li>` correctly
- [ ] Kramdown IAL `{: #id}` on headings is processed correctly
- [ ] Liquid variables `{{ site.repository }}` in links render correctly
- [ ] DTC DOM match count stays at 790/790
- [ ] docs/history/index.html diff count drops from 3357 to under 200
- [ ] No regression on other jekyll-docs pages or any other site
- [ ] Tests include the specific triggering markdown pattern (isolated)

## Test Scenarios

### Unit: History page markdown parsing

- Extract the problematic section of history.md that causes the spurious table insertion
- Parse it in isolation, verify no spurious `<table>` is generated
- Verify list items with inline code render correctly
- Verify `{: #v4-4-1}` IAL on headings does not interfere with subsequent content
- Include a test with the specific pattern: a list item containing text that resembles a table row

### Unit: Table detection edge cases

- Markdown that looks like a table separator inside a list item must NOT start a table
- Content after `{: #id}` IAL should parse correctly as the next block element
- Verify `{{ site.repository }}` Liquid variables in link markdown render as links, not tables

### Integration: jekyll-docs site

- Build jekyll-docs, compare docs/history/index.html DOM
- Verify diff count drops dramatically from 3357
- Verify the page contains the expected `<h2>`, `<h3>`, `<ul>`, `<li>` structure
- Run full DOM comparison on jekyll-docs and DTC, verify no regression

### Output Verification

- Build jekyll-docs site with rustkyll
- Inspect generated `docs/history/index.html` -- compare first 100 elements against
  Jekyll reference at `websites/jekyll-docs/docs/_site_jekyll_cached/docs/history/index.html`
- Verify no `<table>` elements exist in rustkyll output where none exist in Jekyll reference
- DTC site must build and produce 790/790 DOM match
