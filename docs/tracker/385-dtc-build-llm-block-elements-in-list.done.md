# Issue 385: DTC build-large-language-model block elements leaking from list

## Problem

`books/20241017-build-large-language-model-from-scratch.html` has 7 DOM diffs.
Block elements (`<h3>`, `<p>`) produced by markdownify leak outside their
`<ul><li>` container.

Jekyll keeps `<h3>` and `<p>` nested inside `<li>`:

```html
<li>Prepare an instruction format dataset...
  <h3 id="user">User:<br /></h3>
  <p>{}<br /></p>
  <h3 id="assistant">Assistant:<br /></h3>
  <p>Extracted Keywords: {}```<br />
  ...continuation text...</p>
</li>
```

Rustkyll leaves `### User:` and `### Assistant:` as literal text instead of
converting them to `<h3>` tags, and the `<p>` wrapping around intervening
content is missing:

```html
<li>Prepare an instruction format dataset...
### User:<br />
{}<br />
### Assistant:<br />
Extracted Keywords: {}```<br />
...continuation text...</li>
```

Issue #373 added regression tests but found the page "already matched" -- this
was incorrect as the fresh DOM comparison shows 7 diffs remain.

### DOM Diffs (from docs/comparison/dom-details/DataTalksClub-datatalksclub.github.io.txt)

```
DIFF books/20241017-build-large-language-model-from-scratch.html (7 differences)
  body > div > div > div > div > div > ul > li > p: missing_element
  body > div > div > div > div > div > ul > li > h3: missing_element
  body > div > div > div > div > div > ul > li > p: missing_element
  body > div > div > div > div > div > child[4]: tag_name_differs - expected: 'div', actual: 'p'
  body > div > div > div > div > div > h3: extra_element
  body > div > div > div > div > div > p: extra_element
  body > div > div > div > div > div > div: extra_element
```

### Source Data

The affected archive thread is from Muneeb Khan (archive index ~60). The YAML
`text` field contains a long string with `\n` separators that, after YAML
parsing, looks like:

```
...The prompt template is structured somehow like this:
    [bullet] prompt_template =
```### System: You are an expert at extracting keywords...
### User:
{}
### Assistant:
Extracted Keywords: {}```
    [bullet] I do this for the training...
```

The text goes through the `newline_to_br | markdownify` pipeline in the
`book.html` layout template.

### Root Cause Analysis

After `newline_to_br`, each `\n` becomes `<br />\n`:

```
...like this:<br />
    [bullet] prompt_template = <br />
```### System: ...<br />
### User:<br />
{}<br />
### Assistant:<br />
Extracted Keywords: {}```<br />
```

The `escape_fenced_code_after_br` function in `src/frontmatter.rs` detects
that the content between the opening ` ``` ` and closing ` ``` ` contains
`\n### ` (heading markers), so it sets `is_inline_code = false` and
backslash-escapes the opening triple backticks to produce literal `` \`\`\` ``
text. This is correct for making the backticks literal.

However, after this escaping, the `### User:` and `### Assistant:` lines
should be recognized as headings by pulldown-cmark (they are preceded by
`<br />` lines, and `escape_headings_in_list_context` correctly does NOT
escape them). But in the actual output, they appear as literal `### User:`
text, not as `<h3>` tags.

The likely cause is one of:
1. The closing ` ``` ` may be interfering -- after the opening is escaped, the
   closing ` ``` ` is still present and may be parsed as a code fence opener
   by pulldown-cmark, swallowing the heading content
2. The `<br />` on the same line as the heading content (e.g., `### User:<br />`)
   may prevent pulldown-cmark from recognizing the ATX heading syntax
3. The list context interaction -- after `escape_headings_in_list_context`, the
   headings are left unescaped, but pulldown-cmark may still not produce them
   as headings if it considers them continuation text of the list item

The SWE should debug by printing the intermediate markdown just before
`Parser::new_ext()` for this specific content to identify exactly why
pulldown-cmark is not producing `<h3>` tags.

### REGRESSION WARNING

Issues #373 and #368 both failed on similar list/block-element patterns.
#373 was marked done with the claim the page "already matched" but the DOM
comparison proves it does not. #370 attempted a generic re-nesting fix that
regressed other pages and was reverted.

Any fix MUST:
- Be verified against the actual DOM comparison tool, not just `diff` on raw HTML
- Not rely on the SWE's claim that "it already works" -- the DOM comparison is
  the source of truth
- Be tested against the full DTC DOM baseline before reporting done

## Scope

1. Fix block element containment so that `### User:` and `### Assistant:` in
   the Muneeb Khan thread are rendered as `<h3>` tags inside `<li>`, with
   intervening text wrapped in `<p>` tags, matching Jekyll/kramdown output
2. The fix must be generic (handle any heading marker inside a code-fence-like
   context within the `newline_to_br | markdownify` pipeline), not hardcoded
   to this specific page or thread
3. Must not regress DTC DOM (782/790)

## Affected Pages

- `books/20241017-build-large-language-model-from-scratch.html` (7 diffs)

## DTC DOM Baseline

782/790

## Dependencies

- Related to #373 (regression tests -- marked done but bug persists), #377 (similar pattern), #341 (renest_heading_after_list)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` produces no warnings
- [ ] `cargo fmt` reports no formatting issues
- [ ] All existing tests pass (`cargo test`)
- [ ] New targeted unit tests verify that markdown content with `### Heading` inside inline triple-backtick-fenced text (where backticks are not code fences) produces `<h3>` tags when processed through `markdown_to_html_for_filter`
- [ ] On the built DTC site, `books/20241017-build-large-language-model-from-scratch.html` has `<h3 id="user">User:</h3>` and `<h3 id="assistant">Assistant:</h3>` nested inside `<li>`, matching the Jekyll output structure
- [ ] The `<p>` elements between the headings (e.g., `<p>{}<br /></p>`) are also present inside `<li>`, matching Jekyll
- [ ] The 7 DOM diffs for this page are eliminated or reduced (at minimum the 3 missing `ul > li > h3/p` and 3 extra `h3/p/div` are fixed)
- [ ] DTC DOM match count does not drop below 782/790
- [ ] No regressions on other book pages (especially `books/20210823-business-skills-for-data-scientists.html`, `books/20220425-natural-language-processing-with-transformers.html`, `books/20231106-analytics-engineering-with-sql-and-dbt.html`)
- [ ] The fix is generic, not hardcoded to a specific page or thread name
- [ ] Tests include non-ASCII/Unicode content (the archive text contains emoji like U+1F60A and U+25E6)
- [ ] SWE must verify the fix using the DOM comparison tool (`python3 scripts/compare_dom.py`), NOT by eyeballing raw HTML diff -- and report the exact DOM count in the log

## Test Scenarios

### Unit: Heading markers inside inline-backtick-fenced text in list context

- Input: markdown with `newline_to_br` output containing a bullet list item whose continuation includes `` ```### System:...<br />\n### User:<br />\n...\n### Assistant:<br />\n...``` `` -- pass through `markdown_to_html_for_filter`. Verify the resulting HTML has `<h3>` tags for `### User:` and `### Assistant:`, NOT literal `### User:` text.
- Verify the `<h3>` tags and intervening `<p>` elements are nested inside `<li>`, not leaked as siblings after `</ul>`.
- Input: same pattern but with the heading inside actual fenced code blocks (triple backticks on their own line) -- verify `###` inside code blocks is NOT converted to headings (stays as code text).

### Unit: Closing backtick handling

- Input: markdown where opening `` ``` `` is escaped by `escape_fenced_code_after_br` but closing `` ``` `` remains -- verify the closing backticks become literal text and do not open a code fence that swallows heading content.

### Unit: Regression safety for other list patterns

- Input: a simple numbered list (`1. foo\n2. bar`) -- verify correct `<ol><li>` structure preserved.
- Input: a nested bullet list -- verify nesting preserved.
- Input: a list item followed by a paragraph with no heading -- verify proper placement.
- Include a test with emoji/Unicode content (U+1F60A, U+25E6) in the list item text.

### Integration: DTC book page output verification

- Build the DTC site and check `books/20241017-build-large-language-model-from-scratch.html`:
  - The Muneeb Khan thread renders with `<h3 id="user">User:</h3>` and `<h3 id="assistant">Assistant:</h3>` inside `<li>`, not as plain text or leaked outside.
  - The `<p>` elements between headings are also inside `<li>`.
  - The subsequent `<div class="book-archive-reply">` for the reply follows the thread correctly.
- Run DOM comparison and verify count is at least 782/790.

### Regression: Other book and collection pages

- Verify `books/20210823-business-skills-for-data-scientists.html` does not regress (currently 6 diffs).
- Verify `books/20220425-natural-language-processing-with-transformers.html` does not regress (currently 3 diffs).
- Verify `books/20231106-analytics-engineering-with-sql-and-dbt.html` does not regress (currently 11 diffs).
- Verify `books/20221121-reliable-machine-learning.html` does not regress (currently 13 diffs).

## Log

### [SWE] 2026-03-27

- Read issue, traced root cause through `escape_fenced_code_after_br` and `renest_heading_after_list`
- Root cause: `renest_heading_after_list` only moved ONE heading after `</li></ul>` back into `<li>`. When multiple block elements (`<h3>`, `<p>`, `<h3>`, `<p>`) follow the list close, only the first heading was re-nested. The subsequent `<p>{}</p>`, `<h3 id="assistant">`, and final `<p>` leaked outside.
- TDD: wrote 9 unit tests in `tests/test_issue_385.rs` (headings become h3, nesting inside li, closing backtick handling, regression safety for other list patterns, Unicode/emoji content, real fenced code blocks)
- Tests initially passed (the simple test cases already worked because the unit test inputs were simple enough for the existing code). Verified with DOM comparison that the actual page still had 8 diffs.
- Implemented fix: modified `renest_heading_after_list` in `src/frontmatter.rs` to collect ALL consecutive `<h>` and `<p>` elements after `</li></ul>` (not just the first heading) and re-nest them all inside `<li>`. The collection stops at `<div>` or any non-heading/paragraph element.
- Updated 2 existing issue-373 tests that expected the old (incorrect) single-heading behavior to expect the new correct behavior (all headings and paragraphs re-nested, `<div>` stays outside).
- Build: 2960 tests pass (2886 lib + 41 + 4 + 12 + 17 integration), 0 fail, clippy clean, fmt clean
- DOM comparison before fix: `books/20241017-build-large-language-model-from-scratch.html` had 8 diffs
- DOM comparison after fix: `books/20241017-build-large-language-model-from-scratch.html` has 1 diff (unrelated `href` attribute)
- 7 content diffs eliminated (3 missing `ul>li>h3/p`, 3 extra `h3/p/div`, 1 tag_name_differs)
- Regression check: business-skills (7 diffs), natural-language-processing (4 diffs), analytics-engineering (12 diffs), reliable-ml (14 diffs) -- all unchanged from baseline
- DTC DOM summary: 5 matched, 782 with differences, 1214 total differences (was 1222 before fix = 8 fewer)
- Files modified: `src/frontmatter.rs`, `tests/test_issue_385.rs`

### [QA] 2026-03-27

- Build: `cargo build --release` -- PASS
- Tests: all pass (9 issue-385 tests, full suite green)
- Clippy: clean (no warnings from rustkyll crate)
- Formatting: `cargo fmt --check` -- clean
- DOM comparison: 783/790 (baseline 782/790) -- improved by 1
- `books/20241017-build-large-language-model-from-scratch.html`: no longer in DIFF list (was 7 diffs, now 0)
- Verified HTML output: `<h3 id="user">User:<br /></h3>` at line 1136 inside `<li>`, `<h3 id="assistant">Assistant:<br /></h3>` at line 1140 inside `<li>`, `<p>{}<br /></p>` at line 1138 between headings inside `<li>`
- Regression pages unchanged from baseline:
  - business-skills: 6 diffs (AC baseline: 6)
  - natural-language-processing: 3 diffs (AC baseline: 3)
  - analytics-engineering: 11 diffs (AC baseline: 11)
  - reliable-ml: 13 diffs (AC baseline: 13)
- Fix is generic (no hardcoded page/thread names)
- Tests include Unicode content (U+1F60A emoji, U+25E6 bullet)
- 9 new tests covering: heading markers become h3, nesting inside li, p elements between headings, closing backtick handling, simple numbered list regression, nested bullet list regression, list-then-paragraph regression, Unicode/emoji, real fenced code blocks
- Issue #373 tests updated correctly (2 tests now expect all block elements re-nested, not just first heading)
- Note: diff also includes unrelated issue #384 changes (mailto stripping, pipe decoding) -- these do not affect #385 functionality

Acceptance criteria:
- [x] `cargo build` compiles without errors
- [x] `cargo clippy -- -D warnings` produces no warnings
- [x] `cargo fmt` reports no formatting issues
- [x] All existing tests pass
- [x] New targeted unit tests verify heading markers become `<h3>` tags (9 tests)
- [x] `<h3 id="user">User:</h3>` and `<h3 id="assistant">Assistant:</h3>` nested inside `<li>` in built page
- [x] `<p>` elements between headings present inside `<li>`
- [x] 7 DOM diffs eliminated (page now matches perfectly)
- [x] DTC DOM 783/790 >= 782/790 baseline
- [x] No regressions on business-skills, natural-language-processing, analytics-engineering, reliable-ml
- [x] Fix is generic, not hardcoded
- [x] Tests include non-ASCII/Unicode content
- [x] DOM comparison tool used for verification (783/790 reported)

VERDICT: PASS

### [PM] 2026-03-27

Acceptance review completed. Verified all 13 acceptance criteria.

**Code review:**
- `renest_heading_after_list` in `src/frontmatter.rs` extended from collecting a single heading to collecting all consecutive `<h>` and `<p>` elements after `</li></ul>`, stopping at `<div>` or other non-block elements. The logic is clean: a loop with `consume_cursor`, collecting into a `Vec<String>`, then building the replacement string. Generic, no hardcoded content.
- 9 new tests in `tests/test_issue_385.rs` covering: heading conversion, li nesting, p elements between headings, closing backtick handling, regression safety (numbered list, nested bullet, list+paragraph), Unicode/emoji, real fenced code blocks.
- 2 existing issue-373 tests updated to expect the new correct behavior (all block elements re-nested, not just first heading).

**Output verification:**
- Built page `books/20241017-build-large-language-model-from-scratch.html` contains `<h3 id="user">User:<br /></h3>` at line 1136 inside `<li>`.
- DOM comparison confirms 0 diffs for this page (was 7).
- DTC DOM: 783/790 (baseline 782/790, improved by 1).
- Regression pages unchanged: business-skills (6), NLP-transformers (3), analytics-engineering (11), reliable-ml (13).

**Note:** The diff also contains issue #384 changes (mailto stripping, pipe decoding) which are unrelated to this issue but do not interfere.

VERDICT: **ACCEPT**
