# Issue 524: jekyll-docs history page massive structural mismatch (1497 diffs)

## Problem

The `docs/history/index.html` page has 1497 DOM differences -- the largest single
page diff in jekyll-docs. The page structure is completely wrong: elements that
should be `<h3>` appear as `<tbody>`, `<ul>` appears as `<p>`, etc. The content
structure is shifted/offset, causing cascading mismatches for the entire page.

### Root pattern

The history page uses a Liquid-generated changelog with version headings and
bullet lists. A structural error early in the page (likely a table that is
improperly parsed) causes all subsequent elements to be offset, producing the
cascading 1487+ differences.

### Specific diffs

```
body > article > ul > li: expected_text_got_element - expected: 'Document keys of global variable', actual: '<table>'
body > article > ul > li > child[1]: tag_name_differs - expected: 'code', actual: 'table'
body > article > child[15]: tag_name_differs - expected: 'h3', actual: 'tbody'
body > article > child[16]: tag_name_differs - expected: 'ul', actual: 'p'
body > article > child[17]: tag_name_differs - expected: 'h2', actual: 'h3'
... and 1487 more differences
```

A `<table>` element appears where `<code>` is expected in a list item. This
suggests a markdown table inside a list item is being parsed as a real table
instead of inline code, and the resulting structural shift cascades through
the rest of the page.

## Scope

Investigate the history page's markdown source and fix the structural parsing
error. This is likely related to how markdown tables or code blocks inside list
items are handled.

NOTE: Fixing this single page could resolve ~1500 of the ~10,481 total diffs
(~14% of all diffs).

## Dependencies

None.

## DTC DOM Baseline

- Current: 790/790
- Must not drop below: 790/790

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes clean
- [ ] `cargo fmt` produces no changes
- [ ] docs/history/index.html structure matches Jekyll output (h2, h3, ul, li hierarchy)
- [ ] No spurious `<table>` or `<tbody>` elements in the history page
- [ ] Version headings render as `<h2>` / `<h3>` (not shifted elements)
- [ ] Changelog bullet lists render as `<ul><li>` correctly
- [ ] DTC DOM match count must not drop below 790/790
- [ ] docs/history/index.html diff count drops from 1497 to under 100

## Test Scenarios

### Unit: History page markdown parsing

- Extract the problematic section of history.md that causes the table insertion
- Parse it in isolation, verify no spurious `<table>` is generated
- Verify list items with inline code render correctly

### Integration: jekyll-docs site

- Build jekyll-docs, compare docs/history/index.html DOM
- Verify diff count drops dramatically (from 1497)
- Run full DOM comparison, verify no regression on other pages
