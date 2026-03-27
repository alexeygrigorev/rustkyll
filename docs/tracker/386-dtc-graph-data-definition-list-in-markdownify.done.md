# Issue 386: DTC graph-data definition list rendering in markdownify

## Problem

`books/20210405-the-practitioners-guide-to-graph-data.html` has 4 remaining
DOM diffs (confirmed by `./scripts/recount-all-dom.sh` at 783/790 baseline).
Jekyll's markdownify filter (which uses kramdown) renders a definition list
pattern inside an ordered list item as `<dl><dt><dd>` elements. Rustkyll's
markdownify filter uses pulldown-cmark, which has no definition list support,
so it emits the `: ` as literal text.

### Exact input (after `newline_to_br`)

The YAML `archive[].replies[].text` field from Denise Gosnell's reply contains:

```
3. Or, this GitHub<br />
: [https://github.com/awesomedata/awesome-public-datasets](https://github.com/awesomedata/awesome-public-datasets)
```

### Expected output (from Jekyll cached reference)

```html
<li>
  <dl>
    <dt>Or, this GitHub<br /></dt>
    <dd><a href="https://github.com/awesomedata/awesome-public-datasets">https://github.com/awesomedata/awesome-public-datasets</a></dd>
  </dl>
</li>
```

### Current rustkyll output

```html
<li>Or, this GitHub<br />
: <a href="https://github.com/awesomedata/awesome-public-datasets">https://github.com/awesomedata/awesome-public-datasets</a></li>
```

The 4 DOM diffs are:

```
body > div > ... > ol > li: expected_element_got_text - expected: '<dl>', actual: 'Or, this GitHub'
body > div > ... > ol > li > child[1]: tag_name_differs - expected: 'dl', actual: 'br'
body > div > ... > ol > li: extra_text - expected: '(none)', actual: ':'
body > div > ... > ol > li > a: extra_element - expected: '(none)', actual: '<a>'
```

## Prior Art and Regression Warnings

### Issue #368 (REVERTED)

Attempted `break_mixed_list_nesting()` -- a broad heuristic that inserted
HTML block markers to force pulldown-cmark to close lists on type transitions.
Caused a **net regression of 3 pages** (781/790 -> 778/790). Specifically
broke:
- `books/20210927-effective-data-science-infrastructure.html`
- `books/20241104-llm-engineer-s-handbook.html`

The lesson: broad list-nesting heuristics are dangerous. This fix must be
narrowly targeted.

### Issue #382 (INCORRECTLY CLOSED)

Concluded that rustkyll already matches Jekyll, but the SWE compared against
the wrong reference output (raw `_site/` instead of `_site_jekyll_cached/`
used by `recount-all-dom.sh`). The 4 definition list diffs are confirmed real
by the recount script.

### Kramdown parser already supports definition lists

`src/kramdown_parser/parser.rs` has `try_parse_definition_list()` (line ~4244)
with full support for kramdown definition list syntax, including 12 test cases
in `testcases/block/13_definition_list/`. The problem is that the markdownify
filter (`src/template/filters/markdownify.rs`) calls
`markdown_to_html_for_filter()` in `src/frontmatter.rs`, which uses
pulldown-cmark -- NOT the kramdown parser.

## Scope

1. Detect kramdown definition list patterns in the markdownify pipeline input
   and convert them to `<dl><dt><dd>` HTML
2. The conversion must handle the specific case where the pattern appears
   inside an ordered list item context (after `newline_to_br` inserts `<br />`)
3. Must not regress DTC DOM (783/790)
4. No site-specific hardcoding

## Recommended Approach

The safest approach is a **post-processing step** in `markdown_to_html_for_filter()`
(or a preprocessing step before pulldown-cmark) that detects the kramdown
definition list pattern and converts it. Two options:

**Option A: Preprocessing** -- Before pulldown-cmark runs, detect
`term<br />\n: definition` patterns and replace them with pre-rendered
`<dl><dt>term<br /></dt><dd>definition</dd></dl>` HTML blocks. This avoids
changing pulldown-cmark behavior but requires careful handling of markdown
inside the definition (e.g., links in the definition text must still be
rendered by pulldown-cmark).

**Option B: Post-processing** -- After pulldown-cmark produces HTML, scan for
the pattern where a list item contains text followed by `<br />\n: ` and
rewrite it into `<dl><dt><dd>` structure. This is simpler because links and
other inline markup are already resolved, but requires parsing HTML fragments.

Either approach must be **narrow** -- only trigger on the exact kramdown
definition list syntax (`term\n: definition`), not on colons in general text.

## Relevant Code Locations

- `src/frontmatter.rs` -- `markdown_to_html_for_filter()` (line ~747): the
  markdownify pipeline where preprocessing or postprocessing should be added
- `src/kramdown_parser/parser.rs` -- `try_parse_definition_list()` (line ~4244):
  reference implementation of kramdown definition list parsing
- `src/kramdown_parser/html.rs` -- `convert_definition_list()` (line ~1988):
  reference for `<dl><dt><dd>` HTML output format
- `src/template/filters/markdownify.rs` -- the markdownify Liquid filter
- `datatalksclub.github.io/_books/20210405-the-practitioners-guide-to-graph-data.md` --
  source YAML with the definition list pattern

## Dependencies

- None (all prerequisite issues are done)
- Related: #368 (reverted), #382 (incorrectly closed), #363 (parent)

## Baseline

- DTC DOM: 783/790
- Target page: 4 diffs remaining (all from this issue)
- Expected after fix: 784/790 (graph-data page fully matching)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with no failures
- [ ] `cargo clippy -- -D warnings` is clean
- [ ] `cargo fmt` produces no changes
- [ ] When kramdown definition list syntax (`term\n: definition`) appears in
  markdownify input, rustkyll produces `<dl><dt>term</dt><dd>definition</dd></dl>`
- [ ] The specific graph-data page pattern (`3. Or, this GitHub<br />\n: [link](url)`)
  renders as `<dl><dt>Or, this GitHub<br /></dt><dd><a href="...">...</a></dd></dl>`
  inside the `<ol><li>`
- [ ] DTC DOM match count >= 783/790 (must not drop below baseline)
- [ ] Graph-data page (`books/20210405-the-practitioners-guide-to-graph-data.html`)
  has 0 diffs in DOM comparison (was 4)
- [ ] All existing kramdown definition list tests continue to pass
  (`testcases/block/13_definition_list/*`)
- [ ] Regular colons in text (e.g., `I get my graph data from:`) do NOT trigger
  definition list conversion (regression guard)
- [ ] Regular list items without `: ` on the following line are not affected
  (regression guard)
- [ ] No site-specific hardcoding -- the fix must be generic kramdown behavior
- [ ] These pages (which regressed under #368) must remain at their current
  diff count or better:
  - `books/20210927-effective-data-science-infrastructure.html`
  - `books/20241104-llm-engineer-s-handbook.html`

## Test Scenarios

### Unit: Definition list pattern detection

- Input: `"term<br />\n: definition"` after markdownify produces
  `<dl><dt>term<br /></dt><dd>definition</dd></dl>` (not `<p>` wrapping)
- Input: `"1. Snap: http://snap.stanford.edu/\n2. Kaggle: ...\n3. Or, this GitHub<br />\n: [link](url)"`
  produces `<dl>` only for item 3, items 1-2 remain normal `<li>` text
- Input: `"Just some text: with a colon"` does NOT produce `<dl>` (colon
  mid-line is not a definition marker)
- Input: `"I get my graph data from:\n1. First item"` does NOT produce `<dl>`
  (colon at end of line followed by numbered list is not definition syntax)

### Unit: Definition list with inline markup in definition

- Input: `"Term<br />\n: [link text](http://example.com)"` produces `<dd>`
  containing `<a href="http://example.com">link text</a>` (markdown links
  in definition text must be rendered)
- Input: `"Term<br />\n: **bold** definition"` produces `<dd>` containing
  `<strong>bold</strong> definition`

### Unit: Unicode content in definition lists

- Input with non-ASCII term: `"Terme francais<br />\n: definition avec accent"` produces
  correct `<dl><dt><dd>` structure
- Input with non-ASCII definition: `"Term<br />\n: definition"` where
  definition contains Unicode characters

### Regression: Previously broken pages

- Parse patterns that appear in `effective-data-science-infrastructure` and
  `llm-engineer-s-handbook` book pages -- verify the output is unchanged
  from current behavior (no list nesting regressions)
- Parse a pattern with immediate ordered-to-unordered list transition
  (e.g., `1. Item\n- Subitem`) and verify it stays nested (issue #362 pattern)

### Integration: DOM verification

- Build the DTC site with rustkyll
- Run `./scripts/recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io`
- Verify DOM score >= 783/790
- Verify graph-data page has 0 diffs
- Verify `effective-data-science-infrastructure` and `llm-engineer-s-handbook`
  pages are not regressed

## Priority

MEDIUM -- This is the only remaining diff on the graph-data page (4 diffs).
Fixing it would move DTC DOM from 783/790 to 784/790.

## Log

### [SWE] 2026-03-27
- Wrote 8 tests in `tests/test_issue_386.rs` (TDD: tests first)
  - `test_issue386_definition_list_basic` - basic term/definition pattern
  - `test_issue386_definition_list_in_ordered_list` - exact graph-data page pattern
  - `test_issue386_no_definition_list_for_colon_mid_line` - negative: no false positive
  - `test_issue386_no_definition_list_for_colon_at_end_of_line` - negative: colon at EOL
  - `test_issue386_definition_list_with_link` - markdown link in definition
  - `test_issue386_definition_list_with_bold` - bold text in definition
  - `test_issue386_definition_list_unicode` - non-ASCII content
  - `test_issue386_no_regression_on_ordered_unordered_transition` - issue #362 guard
- Ran tests: 5 FAIL, 3 PASS (negative tests pass, positive tests fail as expected)
- Implemented `convert_definition_list_in_html()` in `src/frontmatter.rs`
  - Post-processing approach (Option B): scans HTML output for `<br />\n: ` pattern
  - Rewrites containing `<li>` or `<p>` element content to `<dl><dt><dd>` structure
  - Narrowly targeted: only triggers on exact kramdown definition list syntax
- Ran tests: 8 PASS, 0 FAIL
- Clippy clean, fmt clean
- Built release, ran DOM recount:
  - DTC DOM: 785/790 (baseline was 783/790, +2 improvement)
  - graph-data page: 0 diffs (was 4)
  - effective-data-science-infrastructure: not regressed (0 diffs)
  - llm-engineer-s-handbook: not regressed (0 diffs)
- Files modified: `src/frontmatter.rs`, `tests/test_issue_386.rs`

### [QA] 2026-03-27
- Build: `cargo build --release` -- OK
- Tests: 3243 passed, 0 failed across all test crates (including 8 issue-386 tests)
- Clippy: clean (no rustkyll warnings, only 2 upstream `liquid-lib` deprecation notices)
- Format: `cargo fmt --check` -- clean
- DOM recount: 785/790 (baseline 783, +2 improvement)
- Graph-data page: 0 diffs (was 4) -- confirmed absent from dom-details file
- effective-data-science-infrastructure: 0 diffs -- confirmed absent from dom-details
- llm-engineer-s-handbook: 0 diffs -- confirmed absent from dom-details
- Code review:
  - `convert_definition_list_in_html()` is narrowly scoped: only triggers on `<br />\n: ` pattern
  - No site-specific hardcoding
  - Proper short-circuit when pattern absent
  - Tests cover positive cases, negative guards, unicode, inline markup, and regression guards
- Note: diff also includes issue 387 changes (`renest_leaked_paragraph_and_ul_into_bullet_item` + 5 tests in frontmatter.rs). These are separate from 386 but co-exist in the working tree. The 386-specific changes are `convert_definition_list_in_html()` in frontmatter.rs and `tests/test_issue_386.rs`.
- Acceptance criteria:
  - [x] `cargo build` compiles without errors
  - [x] `cargo test` passes with no failures
  - [x] `cargo clippy -- -D warnings` is clean
  - [x] `cargo fmt` produces no changes
  - [x] Kramdown definition list syntax produces `<dl><dt><dd>` in markdownify
  - [x] Graph-data page pattern renders correctly with `<dl>` inside `<ol><li>`
  - [x] DTC DOM >= 783/790 (actual: 785/790)
  - [x] Graph-data page has 0 diffs (confirmed)
  - [x] Existing kramdown definition list tests pass
  - [x] Regular colons do NOT trigger definition list (test: colon_mid_line)
  - [x] Regular list items without `: ` unaffected (test: colon_at_end_of_line)
  - [x] No site-specific hardcoding
  - [x] effective-data-science-infrastructure not regressed (0 diffs)
  - [x] llm-engineer-s-handbook not regressed (0 diffs)
- VERDICT: **PASS**

### [PM] 2026-03-27 -- Acceptance Review

All 14 acceptance criteria verified:

- [x] Build, test, clippy, fmt -- all clean (QA confirmed)
- [x] Kramdown definition list syntax produces correct `<dl><dt><dd>` structure
- [x] Graph-data page specific pattern renders correctly with `<dl>` inside `<ol><li>`
- [x] DTC DOM 785/790 (baseline 783, +2 improvement)
- [x] Graph-data page: 0 diffs (was 4)
- [x] Existing kramdown definition list tests pass
- [x] Negative guards: regular colons and regular list items not affected (2 dedicated tests)
- [x] No site-specific hardcoding -- `convert_definition_list_in_html()` triggers only on `<br />\n: ` pattern
- [x] effective-data-science-infrastructure: 0 diffs (not regressed)
- [x] llm-engineer-s-handbook: 0 diffs (not regressed)

Test coverage: 8 tests covering positive cases (basic, ordered list, link, bold), negative guards (colon mid-line, colon at EOL), unicode, and cross-issue regression (#362).

Implementation is clean: post-processing approach (Option B from spec), narrowly targeted, proper short-circuit, no over-engineering.

**VERDICT: ACCEPT**
