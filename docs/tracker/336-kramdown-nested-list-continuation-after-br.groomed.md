# Issue 336: Kramdown nested list continuation after `<br>` (DTC book comments)

## Problem

DTC book comment pages use a `{{ thread.text | newline_to_br | markdownify }}` pipeline. When a numbered list item contains a `<br />` tag followed by an indented sub-list, rustkyll closes the `<ol>` and creates a separate `<ul>` instead of keeping the `<ul>` nested inside the `<li>`.

This is the single biggest remaining blocker for DTC 100% DOM coverage, affecting **14 book comment pages** (765->779+/790).

## Example

The real data pattern from DTC book archives (e.g., `_books/20210222-ml-algotrading-2ed.md`) looks like this in the YAML front matter `thread.text`:

```
Alright, so here are a few points on your questions:
1. On Aleix question of how I would describe the use of ML for trading:
- Finance, of course, has very long history of using quantitative tools.
- Just as elsewhere, more data drives more demand for better techniques.
2. On the second question:
```

After `newline_to_br`, every `\n` becomes `<br />\n`, so markdownify receives:

```
Alright, so here are a few points on your questions:<br />
1. On Aleix question of how I would describe the use of ML for trading:<br />
- Finance, of course, has very long history of using quantitative tools.<br />
- Just as elsewhere, more data drives more demand for better techniques.<br />
2. On the second question:
```

Note: the sub-items (`- Finance...`, `- Just as...`) have NO indentation in the source -- they start at column 0 after a `<br />\n` that follows a numbered list item. Kramdown treats these as sub-list items of the preceding `<ol>/<li>` because the `<br />` acts as a soft break, not a block boundary.

Jekyll output (correct):
```html
<ol>
  <li>On Aleix question of how I would describe the use of ML for trading:<br />
    <ul>
      <li>Finance, of course, has very long history of using quantitative tools.<br /></li>
      <li>Just as elsewhere, more data drives more demand for better techniques.<br /></li>
    </ul>
  </li>
  <li>On the second question:</li>
</ol>
```

Rustkyll output (wrong):
```html
<ol>
  <li>On Aleix question...<br /></li>
</ol>
<ul>
  <li>Finance, of course, has very long history...</li>
  <li>Just as elsewhere, more data drives more demand...</li>
</ul>
<ol>
  <li>On the second question:</li>
</ol>
```

## Affected pages (14)

- books/20210222-ml-algotrading-2ed.html (11 diffs)
- books/20210405-the-practitioners-guide-to-graph-data.html (12 diffs)
- books/20210531-advanced-algorithms-and-data-structures.html (9 diffs)
- books/20210823-business-skills-for-data-scientists.html (10 diffs)
- books/20210927-effective-data-science-infrastructure.html (4 diffs)
- books/20211213-mastering-spacy.html (2 diffs)
- books/20220425-natural-language-processing-with-transformers.html (7 diffs)
- books/20220912-skills-of-successful-software-engineer.html (11 diffs)
- books/20220926-graph-algorithms-for-data-science.html (2 diffs)
- books/20221121-reliable-machine-learning.html (17 diffs)
- books/20230807-driving-data-quality-with-data-contracts.html (27 diffs)
- books/20231106-analytics-engineering-with-sql-and-dbt.html (15 diffs)
- books/20240715-ai-data-privacy-and-protection.html (20 diffs)
- books/20241017-build-large-language-model-from-scratch.html (17 diffs)
- books/20241104-llm-engineer-s-handbook.html (7 diffs)

## Root cause

The `markdownify` filter pipeline (`markdown_to_html_for_filter` in `src/frontmatter.rs`) passes content through pulldown-cmark. When pulldown-cmark encounters `<br />` at the end of a numbered list item, it treats the HTML tag as ending the list item's inline content. The subsequent `- ` lines are then parsed as a new top-level `<ul>` rather than a nested sub-list within the `<li>`.

The fix should be implemented as **preprocessing in `markdown_to_html_for_filter`** (in `src/frontmatter.rs`) or in a new kramdown.rs helper called from there. The approach:

1. **Detect the pattern**: After `newline_to_br`, look for sequences where a numbered list item line ends with `<br />` and is immediately followed by `- ` (unordered sub-list items) or another numbered list pattern that should nest.
2. **Preprocess to make pulldown-cmark nest correctly**: Either strip the `<br />` from the end of the parent list item line and properly indent the sub-items, or use a placeholder approach to protect the `<br />` from breaking the list structure, restoring it in postprocessing.

The key insight is that this ONLY applies in the `newline_to_br | markdownify` pipeline (the filter path), not in the main `markdown_to_html` page rendering path. The `escape_fenced_code_after_br` function in `frontmatter.rs` is an existing example of this kind of br-aware preprocessing.

## Scope

This issue covers ONLY the `newline_to_br | markdownify` filter pipeline (`markdown_to_html_for_filter`). It does NOT cover:
- Nested list continuation in the main `markdown_to_html` page path (that is issue 329's Category A)
- Any non-br-related list nesting issues

## Dependencies

- Issue 329 (kramdown list indentation fix via `fix_kramdown_list_indentation`) is in progress but independent -- it handles the main markdown path for mlwiki, not the filter/markdownify path. The two issues may share the indentation-fixing logic, but 336 can proceed independently.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes (no regressions)
- [ ] The `markdown_to_html_for_filter` function correctly nests `<ul>` sub-lists inside `<ol>/<li>` when the input comes from the `newline_to_br | markdownify` pipeline
- [ ] Numbered list item followed by `<br />\n` then `- sub item` lines produces a nested `<ul>` inside the `<li>`, not a separate sibling `<ul>`
- [ ] Multiple sub-items under one numbered list item are all nested inside the same `<li>`
- [ ] Numbered list items without sub-items are unaffected (no regression)
- [ ] Plain text with `<br />` (no list context) is unaffected (no regression)
- [ ] Existing `<br />`-related tests (issue 273, 308) continue to pass
- [ ] Unicode content in list items and sub-items is handled correctly
- [ ] Build the DTC site and verify that at least 10 of the 14 affected book pages show fewer DOM diffs than before (the fix may not resolve every single diff on every page if some diffs have a different root cause)

## Test Scenarios

### Unit: br-then-sublist preprocessing

- Parse `"1. First item<br />\n- sub a<br />\n- sub b<br />\n2. Second item"` through `markdown_to_html_for_filter`, verify the output contains `<ol>` with `<li>` containing a nested `<ul>` with 2 `<li>` items, followed by a second `<li>` for "Second item" -- all inside a single `<ol>`
- Parse `"1. Item one<br />\n- sub<br />\n2. Item two<br />\n- sub2<br />\n3. Item three"` -- verify each numbered item can independently have sub-items, producing two nested `<ul>` elements inside two separate `<li>` elements
- Parse `"1. No sub items<br />\n2. Also no sub items"` -- verify this produces a normal `<ol>` with 2 `<li>` items and no `<ul>` at all (regression check)

### Unit: mixed content patterns

- Parse `"intro text<br />\n1. First item<br />\n- sub a<br />\n2. Second item"` -- verify that intro text is in a `<p>` and the list structure is correct (paragraph before list)
- Parse `"1. **bold item**<br />\n- *italic sub*"` -- verify inline formatting is preserved in both the parent list item and sub-item
- Parse `"1. Item with \`code\`<br />\n- sub with [link](url)"` -- verify inline code and links work in the nested structure

### Unit: Unicode content

- Parse `"1. Universit\u00e9 Technologique<br />\n- R\u00e9sum\u00e9 du cours<br />\n2. \u4f60\u597d"` -- verify non-ASCII characters are preserved in both parent and sub-list items

### Unit: edge cases

- Parse `"- bullet one<br />\n- bullet two"` -- verify that unordered list items with `<br />` between them remain as siblings in a single `<ul>` (NOT nested)
- Parse `"text<br />\n- bullet"` -- verify that a bullet after plain text with `<br />` still creates a list (existing behavior, no regression from issue 273 Pattern C)
- Parse `"1. Item<br />\n\n- sub"` -- verify behavior when there is a blank line between the numbered item and sub-item (this may or may not nest -- the key is it does not crash or produce malformed HTML)

### Integration: DTC book page output verification

- Build the DTC site with `./scripts/cargo-safe run -- --source datatalksclub.github.io --destination _site`
- Compare output for `books/20211213-mastering-spacy.html` (2 diffs -- smallest, easiest to verify) against Jekyll output: the `<ul>` sub-lists should be nested inside `<ol>/<li>` elements, not as siblings
- Compare output for `books/20230807-driving-data-quality-with-data-contracts.html` (27 diffs -- largest, most complex) to verify bulk improvement
- Run the DOM comparison tool on all 14 affected pages and confirm diff counts decrease

## Priority

HIGH -- This is the single biggest remaining blocker for DTC 100%. Fixing this gets DTC from ~765 to ~779/790.
