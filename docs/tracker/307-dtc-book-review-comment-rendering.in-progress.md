# Issue 307: DTC book review comment rendering -- newline_to_br + markdownify interaction

## Problem

DTC matches 662/790 (84%). Of the 128 remaining diff pages, 26 are book
review pages (`books/*.html`) with body content diffs. All 26 share the same
root cause: the book layout template renders comment text with
`{{ thread.text | newline_to_br | markdownify }}`, and the interaction between
`newline_to_br` and `markdownify` produces different HTML structure in rustkyll
vs Jekyll.

Total diff count across the 26 pages: ~270 individual DOM differences.

### Diff patterns observed

#### Pattern 1: Extra `<p>` wrapping before lists (20+ pages)

When comment text contains a line followed by a numbered or bulleted list,
Jekyll produces the text and list as siblings. Rustkyll wraps the text in
an extra `<p>` element, pushing the list to a sibling position.

Example (business-skills-for-data-scientists.html):
- Jekyll: `Here are a few tips<br />\n<ol><li>...</li></ol>`
- Rustkyll: `<p>Here are a few tips<br /></p>\n<ol><li>...</li></ol>`

The `<br />` from `newline_to_br` causes markdownify to wrap the preceding
text in `<p>` when it encounters the list start.

#### Pattern 2: Nested list inside list item broken out (15+ pages)

When a comment contains a numbered list with sub-bullets, Jekyll nests the
`<ul>` inside the `<li>`. Rustkyll breaks the `<ul>` out as a sibling of
the `<ol>`.

Example (business-skills-for-data-scientists.html):
- Jekyll: `<ul><li>text<br /><ol><li>a</li><li>b</li></ol></li></ul>`
- Rustkyll: `<ul><li>text<br /></li></ul><p>...</p><ol><li>a</li>...</ol>`

#### Pattern 3: Missing `<br>` elements (15+ pages)

Some `<br>` elements present in Jekyll output are missing in rustkyll output.
This happens when `newline_to_br` inserts `<br />` but markdownify then
removes or restructures them during markdown-to-HTML conversion.

#### Pattern 4: `<code>` backtick content lost (5+ pages)

In comments with inline code backticks, the `<code>` element is missing
in rustkyll output, or its content is lost during the newline_to_br +
markdownify pipeline.

## Root Cause Analysis

The `newline_to_br` filter converts `\n` to `<br />\n` in the raw text.
Then `markdownify` (which runs `markdown_to_html_for_filter`) processes this
HTML-mixed-with-markdown content. The problem is that the `<br />` tags
injected by `newline_to_br` interact with markdown block-level parsing:

1. A `<br />\n` before a list marker (`1.`, `-`, `*`) causes pulldown-cmark
   to treat the preceding text as a paragraph, wrapping it in `<p>`.
2. The `<br />` tags inside list items break the list continuation rules,
   causing nested lists to be broken out.
3. Some `<br />` tags get consumed by the HTML-to-markdown boundary and
   disappear.

Jekyll's kramdown handles this differently because kramdown treats `<br />`
as an inline HTML element and does not let it affect block-level parsing.
pulldown-cmark (used in frontmatter.rs) treats `<br />` as raw HTML which
may interrupt paragraph/list continuation.

## Scope

All four diff patterns are in scope. The fix must ensure that the output of
`newline_to_br | markdownify` matches Jekyll for the book review comment
use case.

### Possible approaches

1. **Pre-process before markdownify**: In the markdownify filter, detect when
   input already contains `<br />` from newline_to_br, and handle the
   block-level parsing accordingly.
2. **Post-process after markdownify**: After markdown conversion, fix up the
   HTML structure to match the expected output.
3. **Change how newline_to_br interacts with markdownify**: Ensure the `<br />`
   tags do not interfere with markdown block parsing.

### Out of scope

- JSON-LD description diffs (19 pages) -- tracked by issue 305
- Transcript timestamp diffs (54 pages) -- known acceptable
- Syntax highlighting diffs (7 pages) -- tracked by rouge issues
- Emphasis nesting diffs (3 pages) -- tracked by issue 275
- Zoomcamp embed structure diffs (6 pages) -- separate HTML passthrough issue
- `how-do-data-professionals` (170 diffs) -- front matter date/slug mismatch
- `ml-deployment-lambda` (276 diffs) -- Jekyll YAML bug, not our issue

## Dependencies

- None

## Key Files to Modify

- `src/template/filters/markdownify.rs` -- the markdownify filter implementation
- `src/frontmatter.rs` -- `markdown_to_html_for_filter` function
- `src/kramdown.rs` -- `postprocess_for_filter` function

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new tests below
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes
- [ ] Book review comments rendered with `newline_to_br | markdownify` match
      Jekyll output structure: text followed by list does NOT get extra `<p>`
      wrapping
- [ ] `<br />` elements from newline_to_br are preserved in the markdownify
      output (not consumed or lost)
- [ ] Nested lists inside list items remain nested (not broken out as siblings)
- [ ] Inline `<code>` elements within comments are preserved
- [ ] DTC DOM match improves to 682+/790 (from 662, fixing 20+ of the 26
      book review pages)
- [ ] No regressions on other sites (mlwiki, muan-blog, choosealicense,
      lanyon, and all 13+ sites at 100%)
- [ ] Tests include non-ASCII/Unicode content (e.g., comments with emoji,
      accented names)

## Test Scenarios

### Unit: newline_to_br + markdownify interaction

- Input: `"Here are a few tips\n1. First\n2. Second\n3. Third"` through
  `newline_to_br | markdownify`. Verify output: text with `<br />` followed
  by `<ol>` as siblings (no extra `<p>` wrapping the text)
- Input: `"Some text\n- bullet one\n- bullet two"` through the same pipeline.
  Verify `<br />` preserved and `<ul>` follows without extra `<p>`
- Input: `"Question?\n\nAnswer with code: \`example\`"` through pipeline.
  Verify `<code>example</code>` present in output
- Input: `"Text\n1. Item with subbullets\n   - sub a\n   - sub b\n2. Next"`
  through pipeline. Verify `<ul>` is nested inside `<li>` of `<ol>`
- Input: `"Emoji test \U0001F64F\nNext line"` through pipeline. Verify
  emoji preserved and `<br />` between lines

### Unit: markdownify filter standalone

- Verify `markdownify` filter with input containing `<br />\n` before a list
  marker preserves the `<br />` and does not add extra `<p>`
- Verify `markdownify` with input `"text<br />\n<ol>\n<li>a</li>\n</ol>"`
  preserves the structure (does not wrap text in `<p>`)

### Integration: DTC book review pages

- Build DTC site with rustkyll
- Run DOM comparison against Jekyll cached output
- Verify match count is 682+ out of 790
- Spot-check `books/20210823-business-skills-for-data-scientists.html`:
  verify "Here are a few tips" is NOT wrapped in extra `<p>`
- Spot-check `books/20230807-driving-data-quality-with-data-contracts.html`:
  verify `<br>` elements in comment text are preserved
- Spot-check `books/20211213-mastering-spacy.html`: verify nested lists
  are correctly nested
- Verify no new diffs on non-book pages

### Regression: Other sites

- Run `cargo test` full suite
- Verify mlwiki match count unchanged (311+/644)
- Verify muan-blog match count unchanged (2174+/2218)
- Verify all 13+ sites at 100% remain at 100%

## Output Verification

```bash
./scripts/cargo-safe build --release
./target/release/rustkyll build \
  --source websites/DataTalksClub/datatalksclub.github.io/ \
  --destination /tmp/dtc_test

python3 scripts/dom_compare.py \
  --jekyll-dir websites/DataTalksClub/datatalksclub.github.io/_site_jekyll_cached \
  --rustkyll-dir /tmp/dtc_test
```

Spot-checks:
- Compare comment section of `books/20210823-business-skills-for-data-scientists.html`
  between Jekyll and rustkyll outputs -- structure should match
- Summary line must show >= 682 files matched (up from 662)

## Log

### [SWE] 2026-03-23
- **TDD Step 1**: Wrote 7 failing tests in `src/template/filters/markdownify.rs`:
  - `test_issue307_text_br_then_ordered_list_no_extra_p` -- text + `<br />` before `<ol>` should not get `<p>` wrapping
  - `test_issue307_text_br_then_unordered_list_no_extra_p` -- same for `<ul>`
  - `test_issue307_nested_list_stays_nested` -- nested lists remain inside parent `<li>`
  - `test_issue307_br_elements_preserved` -- `<br />` elements are not lost
  - `test_issue307_code_backticks_after_br` -- inline code preserved after `<br />`
  - `test_issue307_text_br_before_html_ol_no_extra_p` -- pre-existing HTML list
  - `test_issue307_unicode_br_markdownify` -- non-ASCII/emoji content preserved
- **TDD Step 2**: Ran tests, 3 FAILED as expected:
  - `test_issue307_text_br_then_ordered_list_no_extra_p`: got `<p>Here are a few tips<br /></p>` (extra `<p>` wrapping)
  - `test_issue307_text_br_then_unordered_list_no_extra_p`: got `<p>Some text<br /></p>` (same)
  - `test_issue307_text_br_before_html_ol_no_extra_p`: got `<p>text<br /></p>` (same)
  - 4 other tests passed (patterns 2-4 already worked)
- **TDD Step 3**: Implemented fix -- added `unwrap_br_paragraphs_before_lists()` in `src/kramdown.rs`
  - Post-processes HTML to detect `<p>...<br /></p>` followed by `<ol>`/`<ul>` and removes the `<p>` wrapper
  - Called from `postprocess_for_filter()` after `remove_ol_start_attribute` and before `add_block_spacing`
  - Updated existing test `test_issue273_pattern_c_numbered_list_after_br` to match corrected behavior (text before list no longer requires `<p>` wrapping)
- **TDD Step 4**: Ran tests, all 7 new tests PASS. All 2509 library tests PASS (2 ignored).
- Clippy: clean (no warnings from my code)
- Format: clean for my files
- Files modified: `src/kramdown.rs`, `src/template/filters/markdownify.rs`
