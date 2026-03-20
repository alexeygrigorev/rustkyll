# Issue 273: DTC `<br>` element handling in `newline_to_br | markdownify` pipeline

## Problem

23 `<br>`-related DOM diffs across ~12 DTC book pages. All diffs occur in the book archive Q&A sections rendered by `_layouts/book.html`, which uses the filter chain `{{ thread.text | newline_to_br | markdownify }}`. The `newline_to_br` filter converts `\n` to `<br />\n` BEFORE markdownify runs, which disrupts pulldown-cmark's markdown parsing in several ways.

No other DTC pages use `newline_to_br`, so this issue is scoped entirely to the book archive template.

## Root Cause Analysis

The `newline_to_br` filter inserts `<br />\n` for every `\n` in the YAML text values. When `markdownify` (pulldown-cmark) then processes this modified text, the injected `<br />` tags interfere with markdown structure recognition. Three distinct failure patterns exist:

### Pattern A: Missing `<br>` + missing `<code>` (4 diffs, 3 pages)

Affected pages: `street-coder`, `deep-learning-with-fastai-cookbook`, `driving-data-quality-with-data-contracts`

YAML text contains inline code (backticks) followed by `\n`. After `newline_to_br`, the input to markdownify looks like:

```
Use `code` here<br />\nWhat's next?
```

Pulldown-cmark misparses the combination of inline code + `<br />` in certain contexts, losing both the `<code>` element and the `<br>` element. Jekyll/kramdown handles this correctly because kramdown processes `<br>` as inline HTML within the paragraph and preserves the code spans.

### Pattern B: Missing `<br>` inside list items (7 diffs, 2 pages)

Affected pages: `mastering-spacy`, `build-large-language-model-from-scratch`

Multi-line content within a `<ul><li>` or `<ol><li>` that contains `<br />\n` between lines. Pulldown-cmark may be consuming the `<br />` tags during list item processing rather than preserving them as inline HTML. In the mastering-spacy case, the text contains literal code snippets (e.g., `>>> import spacy`) separated by `\n` within a list item -- Jekyll renders each line separated by `<br>`, but rustkyll loses them.

### Pattern C: Extra `<br>` where Jekyll renders `<ol>` (6 diffs, 2 pages)

Affected pages: `skills-of-successful-software-engineer`, `reliable-machine-learning`

YAML text contains numbered list patterns like:

```
great questions!\n4. Writing did the trick...\n3. I'm not sure...\n2. I would say...
```

After `newline_to_br`:

```
great questions!<br />\n4. Writing did the trick...<br />\n3. I'm not sure...<br />\n2. I would say...
```

Jekyll/kramdown recognizes the numbered items as an `<ol>` list. Pulldown-cmark does NOT recognize list structure because the `<br />` before `\n4.` breaks the list-start detection, so it renders everything as a `<p>` with `<br>` elements -- producing extra `<br>` tags and missing `<ol><li>` structure.

### Pattern D: Miscellaneous extra/missing `<br>` (6 diffs, 5 pages)

Affected pages: `transfer-learning-in-action`, `natural-language-processing-with-transformers`, `llm-engineer-s-handbook`, `driving-data-quality-with-data-contracts`, `street-coder`

Various cases where the interplay between `newline_to_br` and pulldown-cmark's paragraph/list handling produces wrong `<br>` placement -- either extra `<br>` elements in paragraphs or missing `<br>` in list items.

## Scope

Fix the `newline_to_br | markdownify` pipeline so that the output matches Jekyll/kramdown for all affected book pages.

### Possible approaches (engineer should evaluate)

1. **Pre-process approach**: Before passing to pulldown-cmark, detect markdown structures (lists, code blocks) in the `newline_to_br`-modified text and handle them specially -- e.g., remove `<br />` before list markers so pulldown-cmark sees the list structure.

2. **Post-process approach**: After markdownify, detect cases where `<br>` should exist but was stripped, or where list structure should exist but `<br>` was produced instead, and fix them.

3. **Dedicated markdownify-with-br mode**: Instead of chaining `newline_to_br | markdownify` as two independent Liquid filters, detect when both are used together (or add a combined filter) and handle newlines differently -- e.g., run pulldown-cmark first on the raw text, then insert `<br>` for newlines that are NOT part of markdown structural elements (lists, code blocks, headings).

4. **Kramdown-compatible newline handling**: Study how kramdown actually processes `newline_to_br` output and replicate its HTML-aware parsing behavior that preserves `<br>` as inline elements without disrupting block-level markdown parsing.

### What is NOT in scope

- Non-book DTC pages (no other templates use `newline_to_br`)
- Non-DTC sites (muan-blog's HARDBREAKS is handled by issue 223)
- Other DOM diffs on book pages that are NOT `<br>`-related (e.g., text_differs, attribute_differs)

## Dependencies

- None. The `newline_to_br` and `markdownify` filters already exist. This is a behavioral fix.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all new and existing tests
- [ ] `cargo clippy -- -D warnings` passes
- [ ] Pattern A fixed: On pages with inline code + `<br>` (e.g., `street-coder`), the output contains both `<code>` and `<br />` elements matching Jekyll
- [ ] Pattern B fixed: On pages with multi-line content in list items (e.g., `mastering-spacy`), `<br />` elements appear between lines within `<li>` elements
- [ ] Pattern C fixed: On pages with numbered list text (e.g., `skills-of-successful-software-engineer`), the output renders `<ol><li>` structure matching Jekyll, not `<p>` with `<br>` separators
- [ ] Pattern D fixed: Remaining `<br>` diffs on other affected book pages are resolved
- [ ] No regressions on non-book DTC pages
- [ ] No regressions on non-DTC sites (run existing test suite)
- [ ] Tests include non-ASCII/Unicode content to guard against encoding regressions
- [ ] The `<br />` output uses XHTML-style self-closing (matching Jekyll/kramdown's output for this site)

## Test Scenarios

All tests follow TDD: write the test FIRST, verify it FAILS, implement the fix, verify it PASSES.

### Unit: Pattern A -- inline code + br in markdownify pipeline

- **Test: code span followed by br preserved** -- Input: `` "Use `code` here<br />\nWhat's next?" `` passed through `markdown_to_html_for_filter`. Assert output contains both `<code>code</code>` and `<br />` within a `<p>` element.
  - Write test FIRST. Expect FAIL. Implement. Verify PASS.

- **Test: multiple code spans with br** -- Input: `` "`first`<br />\n`second`" ``. Assert both `<code>` elements and the `<br />` are preserved.
  - Write test FIRST. Expect FAIL. Implement. Verify PASS.

### Unit: Pattern B -- br inside list items

- **Test: br elements preserved within list item** -- Input: `"- line one<br />\nline two<br />\nline three"` passed through `markdown_to_html_for_filter`. Assert output has `<li>` containing text separated by `<br />` elements.
  - Write test FIRST. Expect FAIL. Implement. Verify PASS.

- **Test: br in nested list item** -- Similar test with `<ul><li>` containing multi-line code snippets.
  - Write test FIRST. Expect FAIL. Implement. Verify PASS.

### Unit: Pattern C -- numbered list recognition despite br

- **Test: numbered list after br-modified newlines** -- Input: `"great questions!<br />\n4. Writing did the trick<br />\n3. I'm not sure<br />\n2. Communication"` passed through `markdown_to_html_for_filter`. Assert output contains `<ol>` with `<li>` elements, NOT `<p>` with `<br>` separators.
  - Write test FIRST. Expect FAIL. Implement. Verify PASS.

- **Test: unordered list after br-modified newlines** -- Input: `"intro text<br />\n- first item<br />\n- second item"`. Assert output contains `<ul><li>` structure.
  - Write test FIRST. Expect FAIL. Implement. Verify PASS.

### Unit: Unicode content

- **Test: br handling with non-ASCII text** -- Input containing Unicode characters (e.g., German umlauts, emoji, CJK) around `<br />\n` boundaries. Assert `<br />` is preserved and Unicode content is intact.
  - Write test FIRST. Expect FAIL if affected by fix. Verify PASS.

### Unit: Regression guards

- **Test: simple newline_to_br then markdownify still works** -- Input: `"First line.<br />\nSecond line."`. Assert output preserves `<br />` in `<p>`. (Existing test `test_newline_to_br_then_markdownify_pipeline` covers this -- verify it still passes.)

- **Test: markdownify without br is unchanged** -- Plain markdown input without any `<br>` elements. Assert normal rendering.

### Integration: Full site build (DTC)

- **Test: DTC book page br element count** -- (Mark as `#[ignore]` for CI speed.) Build DTC site, read a known affected book page (e.g., `books/20210322-street-coder.html`), count `<br />` elements in the Q&A section. Assert the count matches Jekyll's output for that page.

## Output Verification

After implementation, the engineer must build the DTC site and inspect the following pages:

1. `_site/books/20210322-street-coder.html` -- Q&A section must contain `<br />` elements within paragraphs where the YAML text has `\n`. Inline `<code>` spans must also be present.
2. `_site/books/20211213-mastering-spacy.html` -- The spacy code snippet in a list item must have `<br />` between each line (`>>> import spacy`, `>>> nlp = spacy.load(...)`, etc.).
3. `_site/books/20220912-skills-of-successful-software-engineer.html` -- Numbered answers (4, 3, 2, 1) must render as an `<ol>` with `<li>` elements, NOT as a `<p>` with `<br>` separators.
4. Compare these pages against the Jekyll reference output in `datatalksclub.github.io/_site/` (or build Jekyll reference with `bundle exec jekyll build` if available) to confirm the DOM structure matches.

## Affected Pages (complete list from DOM comparison)

Pages with `<br>` missing_element or extra_element diffs:

| Page | missing_br | extra_br | Pattern |
|------|-----------|----------|---------|
| books/20210322-street-coder.html | 6 | 0 | A, B, D |
| books/20211004-transfer-learning-in-action.html | 0 | 1 | D |
| books/20211206-deep-learning-with-fastai-cookbook.html | 1 | 0 | A |
| books/20211213-mastering-spacy.html | 5 | 1 | B, D |
| books/20220425-natural-language-processing-with-transformers.html | 0 | 1 | D |
| books/20220912-skills-of-successful-software-engineer.html | 1 | 3 | C |
| books/20221121-reliable-machine-learning.html | 0 | 1 | D |
| books/20230807-driving-data-quality-with-data-contracts.html | 2 | 1 | A, D |
| books/20241017-build-large-language-model-from-scratch.html | 1 | 0 | B |
| books/20241104-llm-engineer-s-handbook.html | 0 | 1 | D |

Total: 16 missing + 9 extra = 25 br-related DOM diffs (some pages have diffs in both directions)

## Log

- 2026-03-20: Created from DTC DOM comparison analysis.
- 2026-03-20: Groomed by PM. Root cause identified: `newline_to_br | markdownify` pipeline interaction where `<br />` tags injected by `newline_to_br` disrupt pulldown-cmark's markdown structure recognition. Three distinct failure patterns (A: code+br lost, B: br lost in list items, C: list structure replaced by br). 25 total br-related diffs across 10 book pages.

### [SWE] 2026-03-20
- TDD: Wrote 9 markdownify tests + 7 kramdown tests covering all patterns (A, B, C, D, Unicode, regression)
- Patterns A, B already handled correctly by pulldown-cmark (7 tests pass immediately)
- Pattern C test with `1.` also passes (kramdown treats only `1.` as list start)
- Pattern D: Wrote test for pipe-in-angle-brackets (`<tel:100-1000|100-1000>`) -- FAILS as expected (produces `<table>` instead of list)
- Root cause: `is_kramdown_table_line()` in kramdown.rs counted `|` inside `<...>` angle brackets as pipe-table delimiters
- Implemented `has_pipe_outside_angle_brackets()` helper that skips `|` inside `<...>`
- Ran test: PASSES -- no more table for autolinks/Slack refs with pipes
- Fixed 3 issue-272 tests that incorrectly expected table output for `<#C01AXGTRESH|books>` and `<mailto:a@b.com|a@b.com>` -- Jekyll escapes these as plain text, not tables
- Full test suite: 2033 lib + 289 other = all pass, 0 fail
- Clippy: pre-existing failure in `liquid-core` dependency (not our code)
- Fmt: clean
- DOM comparison: 0 br-related diffs remain (was 2 missing + 1 extra before fix)
- NLP transformers page now matches Jekyll (was broken by table parsing of `<tel:...|...>`)
- Slack ref page (blueprints-for-text-analytics) now matches Jekyll (was broken by table parsing of `<#...|...>`)
- Files modified: src/kramdown.rs, src/template/filters/markdownify.rs
