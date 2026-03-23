# Issue 316: DTC remaining 47 DOM diff pages -- book comments, embedded HTML, syntax highlighting

## Problem

DTC matches 743/790 (94%). After issues 275, 296, 305, 307, 308, 312 (emphasis,
JSONLD, book comments newline_to_br, sexagesimal filtering), 47 pages still have
DOM diffs. They cluster into clear categories.

### Category A: Book comment markdown rendering (21 pages)

The book review template renders user comments via:
```liquid
{{ thread.text | newline_to_br | markdownify }}
```

Issue 308 fixed 3 pages (backtick/fenced-code after `<br />`), but 21 book pages
still have diffs. The remaining issues are:

**A1: Nested list continuation after `<br />` (5 pages)**

When `newline_to_br` inserts `<br />\n` inside an ordered list item that continues
with a nested unordered sub-list, rustkyll breaks out of the `<ol>` and creates a
separate `<ul>`. Jekyll keeps the `<ul>` inside the `<li>`.

Example from `20210222-ml-algotrading-2ed.html`:
- Source: `1. On Aleix question...\n   - Finance has...\n   - Just as elsewhere...`
- After newline_to_br: `1. On Aleix question...<br />\n   - Finance has...<br />\n`
- Jekyll: `<ol><li>On Aleix...<br /><ul><li>Finance...</li></ul></li></ol>`
- Rustkyll: `<ol><li>On Aleix...<br /></li></ol><ul><li>Finance...</li></ul>`

Affected pages: `20210222-ml-algotrading-2ed`, `20210405-the-practitioners-guide`,
`20210927-effective-data-science`, `20240715-ai-data-privacy`, `20241104-llm-engineer`

**A2: Smart quote style in book comments (6+ pages)**

Comments contain typographic quotes that differ between Jekyll and rustkyll.
- Jekyll: curly/straight quotes depending on kramdown's smart quote rules
- Rustkyll: different curly/straight quote decisions

Example from `20210412-ai-and-machine-learning-for-coders.html`:
- Jekyll: `"TensorFlow Advanced Techniques"` (one quote style)
- Rustkyll: `"TensorFlow Advanced Techniques"` (different quote style)

**A3: Numbered list / `<br>` interaction with block elements (10+ pages)**

Comments with numbered lists where items contain `<br />` tags produce different
block-level structures. `<br />\n` before a numbered item like `3. text` causes
rustkyll to start a new `<ol>` instead of continuing the existing one. Similarly,
`<br />\n` before a blockquote `>` causes structural differences.

Examples:
- `20220912-skills-of-successful-software-engineer`: `<br />\n4.` starts new list
- `20221121-reliable-machine-learning`: `<br />\n3.` starts new list
- `20230807-driving-data-quality-with-data-contracts`: numbered items with `<em>`
- `20231106-analytics-engineering-with-sql-and-dbt`: blockquote after `<br />`

### Category B: Embedded HTML include rendering (6 pages)

Blog posts using `{% include %}` tags for structured content (FAQ accordions,
course structured data) produce shifted DOM elements. The `{% include %}` output
appears in a different position relative to surrounding content.

Pattern: `child[N]: tag_name_differs - expected: 'p', actual: 'div'` followed by
`div` -> `script` and `script: missing_element`.

This affects the 5 zoomcamp course pages (`ai-dev-tools`, `data-engineering`,
`llm`, `machine-learning`, `mlops`) plus `how-do-professionals-use-llm`.

Root cause investigation needed: these includes (`faq-accordion.html`,
`course-structured-data/*.html`, `related-posts.html`) render correctly in
Jekyll but their output is offset by one element in rustkyll. This may be
caused by a missing or extra `<p>` tag before or after the include output.

### Category C: Syntax highlighting class diffs (6 pages)

Rouge token classes differ for specific languages:
- SQL: `k` (Keyword) vs `n` (Name) for keywords like `SELECT`, `WHERE`
- Shell/Bash: various class diffs for command arguments
- Python: `k` vs `n` for `print`, decorators

Affected: `do-you-know-golden-rules`, `how-to-run-postgresql`, `important-sql-fact`,
`naming-variables`, `open-source-free-ai-agent-evaluation`, `practical-guide-better-code`

### Category D: JSONLD / miscellaneous (8 pages)

- 4 pages with JSONLD description diffs (remaining from issue 305)
- 2 pages with markdown link parsing issues (`{:target="_blank"}` attributes)
- 2 podcast pages with `<br>` / text split diffs

## Scope

This issue focuses on **Categories A and B** (27 pages combined). These are the
highest-impact categories and share related root causes (markdown rendering of
HTML-containing content, especially the `newline_to_br | markdownify` pipeline
and include rendering).

### In scope

1. **Fix nested list continuation in newline_to_br | markdownify pipeline** --
   when `<br />\n` appears inside a list item that has indented sub-list content,
   the sub-list must remain inside the parent `<li>`. The markdown parser must
   not treat `<br />\n` as a list item terminator.

2. **Fix numbered list restart after `<br />`** -- `<br />\n3. text` inside an
   existing `<ol>` should continue the list, not start a new one. The `<br />`
   before a numbered item must not cause list termination.

3. **Fix include rendering position** -- investigate why `{% include %}` output
   for FAQ accordions and structured data appears offset by one element.
   Determine if a `<p>` tag is being incorrectly inserted or omitted before
   the include output.

### Out of scope

- Syntax highlighting class mapping (Category C, 6 pages) -- separate rouge
  token mapping work, extend issue 310 or create new issue
- JSONLD / miscellaneous (Category D, 8 pages) -- various unrelated small issues

## Dependencies

- Issue 308 (book comments newline_to_br) -- DONE. This issue continues that work.
- Issue 305 (JSONLD description) -- DONE. Category D is partially residual.

## Key Files to Modify

- `src/frontmatter.rs` -- `markdown_to_html_for_filter()` and preprocessing:
  the `newline_to_br | markdownify` pipeline where `<br />` interacts with
  list parsing
- `src/kramdown.rs` -- `postprocess_for_filter_with_options()`: list item
  handling when `<br />` is present
- `src/kramdown_parser/parser.rs` -- list continuation logic: how `<br />`
  inside list items affects block-level parsing
- `src/template/include_tag.rs` -- include rendering: investigate whether
  whitespace around include output differs from Jekyll
- `src/generator.rs` -- page rendering pipeline for pages with includes

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new tests below
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes
- [ ] `newline_to_br | markdownify` on input `1. text\n   - sub\n   - sub2\n2. next`
      produces `<ol><li>text<ul><li>sub</li><li>sub2</li></ul></li><li>next</li></ol>`
      (nested list inside parent `<li>`)
- [ ] `newline_to_br | markdownify` on input with `<br />\n3. item` inside an
      existing numbered list continues the list (does not restart `<ol>`)
- [ ] Blog posts with `{% include faq-accordion.html %}` render the FAQ div
      in the same position as Jekyll (no element offset)
- [ ] Course structured data includes render `<script type="application/ld+json">`
      in the correct position
- [ ] DTC DOM match improves to 760+/790 (from 743, fixing 17+ pages)
- [ ] No regressions on muan-blog (must remain 2174+/2218)
- [ ] No regressions on mlwiki (must remain 559+/644)
- [ ] No regressions on any of the 13+ sites currently at 100%
- [ ] Tests include non-ASCII/Unicode content (comments with accented names,
      CJK text in list items, emoji in comments)

## Test Scenarios

### Unit: Nested list continuation with `<br />`

- Input through `newline_to_br | markdownify`:
  `1. Question about ML<br />\n   - Point A<br />\n   - Point B<br />\n2. Follow-up`
- Verify: `<ol><li>Question...<br /><ul><li>Point A</li><li>Point B</li></ul></li><li>Follow-up</li></ol>`
- Verify: `<ul>` is a CHILD of the first `<li>`, not a sibling

### Unit: Numbered list continuation after `<br />`

- Input through `newline_to_br | markdownify`:
  `1. First<br />\n2. Second<br />\n3. Third`
- Verify: single `<ol>` with 3 `<li>` items (not multiple `<ol>` elements)

### Unit: Blockquote after `<br />` in comment

- Input: `text<br />\n> quote`
- Verify: `<p>text<br /></p><blockquote><p>quote</p></blockquote>` (proper
  block separation, not text merged with blockquote)

### Unit: Comment with ordered list, nested unordered, and `<br />`

- Replicate the exact pattern from `20210222-ml-algotrading-2ed`:
  `1. On question:<br />\n   - Finance has...<br />\n   - More data...<br />\n2. Next`
- Verify nested structure matches Jekyll output

### Unit: Unicode in book comments

- Input: `1. Recommendation for Munstermann<br />\n   - See Recce at Neuberger Berman<br />\n2. Danke schon!`
- Verify accented characters preserved, nested list correct

### Integration: Include rendering position

- Build a minimal test page with `{% include %}` that outputs a `<div>` block
- Verify the include output appears at the correct DOM position relative to
  surrounding `<p>` and `<h2>` elements
- No extra or missing `<p>` tags around include output

### Integration: DTC site build

- Build DTC site with rustkyll
- Run DOM comparison against Jekyll cached output
- Verify match count is 760+ out of 790
- Spot-check pages:
  - `books/20210222-ml-algotrading-2ed.html` -- nested list in comments
  - `books/20220912-skills-of-successful-software-engineer.html` -- numbered
    list continuation
  - `blog/llm-zoomcamp.html` -- FAQ include position
  - `blog/data-engineering-zoomcamp.html` -- structured data include position
- Verify no new diffs introduced

### Regression: Other sites

- Run `cargo test` full suite
- Run DOM comparison on muan-blog to verify no regression
- Run DOM comparison on mlwiki to verify no regression
- Verify all 13+ sites at 100% remain at 100%

## Output Verification

```bash
./scripts/cargo-safe build --release
./target/release/rustkyll build \
  --source websites/DataTalksClub/datatalksclub.github.io/ \
  --destination /tmp/dtc_316

python3 scripts/dom_compare.py \
  --jekyll-dir websites/DataTalksClub/datatalksclub.github.io/_site_jekyll_cached \
  --rustkyll-dir /tmp/dtc_316
```

Spot-checks:
- `diff <(grep -A5 'nate8020\|Aleix' _site_jekyll_cached/books/20210222-ml-algotrading-2ed.html | head -20) <(grep -A5 'nate8020\|Aleix' /tmp/dtc_316/books/20210222-ml-algotrading-2ed.html | head -20)` -- must show no diff in list nesting
- `diff <(grep -n 'faq-accordion\|structured-data' _site_jekyll_cached/blog/llm-zoomcamp.html | head -5) <(grep -n 'faq-accordion\|structured-data' /tmp/dtc_316/blog/llm-zoomcamp.html | head -5)` -- line numbers should be close
- Summary line must show >= 760 files matched (up from 743)

## Follow-up Issues

After this issue, remaining DTC diffs will be:
- Syntax highlighting class mapping (6 pages) -- extend rouge token mapping
- JSONLD description edge cases (4 pages) -- specific author bio handling
- Markdown link parsing with `{:target="_blank"}` (2 pages) -- kramdown IAL
- Podcast transcript formatting (2 pages) -- `<br>` placement

## Log

### [SWE] 2026-03-23

**Focus: Category B (include rendering offset, 6 pages)**

- Root cause analysis: After Liquid include processing, HTML comments from
  `related-posts.html` include appear on consecutive lines without blank lines
  between them. The indented comment `<!-- Use manually specified posts -->`
  (2 spaces, from inside `{% if %}` block) should be wrapped in `<p>` to match
  kramdown behavior, but the existing `wrap_standalone_comments_in_paragraphs`
  only wrapped comments surrounded by blank lines.

- TDD cycle:
  1. Wrote test `test_316_indented_comment_among_adjacent_comments_wrapped` -- FAILS as expected
  2. Wrote test `test_316_indented_comment_unicode_content` -- FAILS as expected
  3. Wrote test `test_316_non_indented_adjacent_comments_not_wrapped` -- PASSES (baseline)
  4. Implemented fix: modified `wrap_standalone_comments_in_paragraphs` to wrap
     indented HTML comments (1-3 spaces) that appear adjacent to other comments
  5. First attempt too broad: caused regression on `index.html` (subscribe form
     comment inside HTML block was incorrectly wrapped)
  6. Refined fix: only wrap indented comments when at least one neighbor line is
     also an HTML comment (distinguishes include-output comments from comments
     inside HTML block elements)
  7. Added test `test_316_indented_comment_between_html_elements_not_wrapped` to
     prevent regression
  8. All 4 new tests pass, all 9 existing test_274_ tests pass

- DTC comparison: 742/790 -> 746/790 (+4 pages, 0 regressions)
- Fixed pages:
  - blog/ai-dev-tools-zoomcamp (3 diffs -> 0)
  - blog/data-engineering-zoomcamp.html (3 diffs -> 0)
  - blog/llm-zoomcamp.html (3 diffs -> 0)
  - blog/slack-communities.html (was in diff, now matches)
- Remaining zoomcamp pages reduced but not fully fixed:
  - blog/machine-learning-zoomcamp.html (4 diffs -> 1: alt attribute diff remains)
  - blog/mlops-zoomcamp.html (7 diffs -> 4: bold text parsing diffs remain)

- Build: 2543 tests pass, 0 fail, clippy clean, fmt clean
- Files modified: src/kramdown.rs (added 90 lines)
