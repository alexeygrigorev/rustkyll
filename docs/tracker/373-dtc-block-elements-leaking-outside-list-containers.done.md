# Issue 373: DTC block elements leaking outside list containers (Sub-problem B from #370)

## Problem

On `books/20241017-build-large-language-model-from-scratch.html` (7 diffs), `<h3>` and `<p>` elements that belong inside the comment `<ul><li>` container are rendering as siblings outside it. When `markdownify` produces block-level elements (like `<h3>` from markdown `###`), pulldown-cmark closes the list prematurely, causing subsequent content to leak outside the `<li>`.

### Root Cause

The affected thread (Muneeb Khan, archive thread index 60) contains markdown text with `### User:` and `### Assistant:` headings inside a bullet list context. The text goes through `newline_to_br | markdownify` in the book.html layout template (line 39/42). The markdown input looks approximately like:

```
- Prepare an instruction format dataset...
    ... prompt_template =
```### System: ...
### User:
{}
### Assistant:
Extracted Keywords: {}```
    ... continuation text...
```

When pulldown-cmark processes the `### User:` and `### Assistant:` lines, it produces `<h3>` tags. Unlike kramdown, pulldown-cmark terminates the enclosing `<li>` and `</ul>` before emitting the heading, causing subsequent `<p>` and `<h3>` elements to appear as siblings of the `<ul>` rather than children of the `<li>`.

In the Jekyll (kramdown) output, these block elements stay properly nested inside the `<li>`:
```html
<h3 id="user">User:<br /></h3>
<p>{}<br /></p>
<h3 id="assistant">Assistant:<br /></h3>
<p>Extracted Keywords: {}```<br />
```

In the rustkyll output, the list closes prematurely:
```html
<h3 id="user">User:<br /></h3>
  </li>
</ul>
<p>{}<br /></p>
<h3 id="assistant">Assistant:<br /></h3>
```

### DOM diff (from docs/comparison/dom-details/DataTalksClub-datatalksclub.github.io.txt)

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

## Prior Attempt

Issue #370 attempted a fix (`renest_block_elements_after_list()`) but it caused regressions on machine-learning-zoomcamp and graph-algorithms pages (DOM dropped 780 to 779). The generic block re-nesting was too aggressive and was reverted.

## Scope

1. Fix block element nesting so that `<h3>` and `<p>` produced by markdownify inside list items stay nested within their `<li>` container, matching kramdown behavior
2. The fix must be generic (not page-specific or hardcoded to a particular archive thread)
3. DTC DOM baseline must not drop below 780/790
4. The fix should be in the markdown preprocessing or postprocessing pipeline (either in `markdown_to_html_for_filter` in `src/frontmatter.rs` or in `src/kramdown.rs` postprocessing)

## Approach Guidance

The key insight is that pulldown-cmark (CommonMark) does not allow headings (`# ...`) inside list items -- it terminates the list. Kramdown allows this. Possible approaches:

- **Preprocessing**: Before passing to pulldown-cmark, detect `###` heading markers that appear inside a list-item context (indented under a list marker like `-` or `*`) and convert them to something pulldown-cmark will keep inline (e.g., `<h3>...</h3>` raw HTML), which pulldown-cmark passes through without breaking the list.
- **Postprocessing**: After pulldown-cmark produces the HTML, detect the pattern `</li>\n</ul>\n<h3>` (or similar) and re-nest the leaked block elements back into the preceding `<li>`. This was attempted in #370 and regressed -- any new postprocessing approach must be carefully scoped.

## Affected Pages

- `books/20241017-build-large-language-model-from-scratch.html` (7 diffs)

## Dependencies

- Follow-up from #370 (done)

## DTC DOM Baseline

780/790 (from commit db887db, per dtc-dom-changelog.md)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` produces no warnings
- [ ] `cargo fmt` reports no formatting issues
- [ ] All existing tests pass (`cargo test`)
- [ ] New targeted unit tests are added for the heading-inside-list-item scenario
- [ ] On the built DTC site, `books/20241017-build-large-language-model-from-scratch.html` has `<h3>` and `<p>` elements nested inside `<li>` (matching Jekyll output), not leaked as siblings
- [ ] The 7 DOM diffs for `books/20241017-build-large-language-model-from-scratch.html` are reduced (ideally to 0, but at least the 3 missing `ul > li > h3/p` and 3 extra `h3/p/div` are fixed)
- [ ] DTC DOM match count does not drop below 780/790
- [ ] No regressions on other book pages (especially `books/20220926-graph-algorithms-for-data-science.html` and `books/20210823-business-skills-for-data-scientists.html` which have related list nesting issues)
- [ ] The fix is generic, not hardcoded to a specific page or thread name
- [ ] Tests include non-ASCII/Unicode content (the archive contains emoji like U+1F60A)

## Test Scenarios

### Unit: Heading inside list item via markdownify

- Input: markdown text containing a bullet list where a list item continuation includes `### Heading` -- pass through `markdown_to_html_for_filter`. Verify the resulting HTML has `<h3>` nested inside `<li>`, not leaked as a sibling after `</ul>`.
- Input: markdown text with `### User:\n{}\n### Assistant:\nExtracted Keywords` inside a list item -- verify both `<h3>` tags and intervening `<p>` remain inside `<li>`.
- Input: markdown text with a code block (triple-backtick) containing `###` -- verify the `###` inside code is NOT converted to `<h3>` (it should remain as code).

### Unit: Regression safety for other list patterns

- Input: a simple numbered list (`1. foo\n2. bar`) -- verify it still produces correct `<ol><li>` structure.
- Input: a nested list (bullet inside bullet) -- verify nesting is preserved.
- Input: a list item followed by a paragraph (no heading) -- verify the paragraph is properly placed relative to the list.
- Include a test with emoji/Unicode content in the list item text.

### Integration: DTC book page output verification

- Build the DTC site and check `books/20241017-build-large-language-model-from-scratch.html`:
  - The Muneeb Khan thread's text renders with `<h3 id="user">User:</h3>` and `<h3 id="assistant">Assistant:</h3>` inside `<li>`, not leaked outside.
  - The `<p>` elements between the headings are also inside `<li>`.
  - The subsequent `<div class="book-archive-reply">` for the reply follows the thread, not interspersed with leaked elements.
- Run DOM comparison and verify count is at least 780/790.

### Regression: Other book and collection pages

- Verify `books/20220926-graph-algorithms-for-data-science.html` does not regress (currently 2 diffs).
- Verify `books/20210823-business-skills-for-data-scientists.html` does not regress (currently 9 diffs).
- Verify `books/20220425-natural-language-processing-with-transformers.html` does not regress (currently 3 diffs).

## Log

### [SWE] 2026-03-26

**Investigation:**
- Read the issue and analyzed the DTC book page output
- Found that the committed code (after issues #341, #362, #363, #365, #370) already produces
  output that matches Jekyll for `books/20241017-build-large-language-model-from-scratch.html`
- Built from committed code only (git stash, clean build) and verified:
  `diff` between Jekyll and rustkyll shows only the `href=''` attribute diff (separate issue)
- The "7 DOM diffs" in the issue description were from a pre-#341 code version; the existing
  `renest_heading_after_list` function (from issue #341) already correctly handles this case

**Approach:**
- The issue's root cause was already fixed by `renest_heading_after_list` in issue #341
- That function re-nests the FIRST heading back inside `<li>` (matching kramdown)
- Subsequent `<p>` and `<h3>` elements correctly stay outside (also matching kramdown)
- Attempted to enhance the function to re-nest ALL block elements (headings + paragraphs)
  but this made things WORSE (8 diffs vs 7) because Jekyll also keeps them outside
- Reverted the enhancement; the original function is correct

**TDD:**
- Wrote 8 regression tests for the heading-inside-list-item scenarios
- Tests cover: full pipeline (DTC pattern), Unicode/emoji content, code blocks,
  simple numbered lists, nested bullet lists, paragraph after list, direct
  renest_heading_after_list function, and preservation of subsequent elements
- All 8 tests pass on the committed code (no code changes needed)

**Results:**
- Build: 2861 lib tests pass, 0 fail, 2 ignored
- Full test suite: all pass
- Clippy: clean (no warnings)
- Formatting: clean
- DOM: `books/20241017-build-large-language-model-from-scratch.html` has only 1 diff
  (href attribute, not related to this issue)
- No code changes to the rendering pipeline -- only regression tests added

**Files modified:**
- `src/frontmatter.rs` -- added 8 regression tests for issue 373 scenarios

### [QA] 2026-03-26

**Verification:**

- Full test suite: PASS (all tests pass including 8 new issue 373 tests)
- `cargo clippy -- -D warnings`: PASS (clean, no project warnings)
- `cargo fmt --check`: PASS (no formatting issues)
- 8 issue 373 tests confirmed present and passing:
  - test_issue373_first_heading_renested_into_list_item
  - test_issue373_heading_in_list_with_unicode
  - test_issue373_heading_in_code_block_not_affected
  - test_issue373_simple_numbered_list_not_affected
  - test_issue373_nested_bullet_list_not_affected
  - test_issue373_list_then_paragraph_no_heading
  - test_issue373_renest_single_heading_after_list
  - test_issue373_renest_preserves_subsequent_elements

**DOM baseline:** Issue 373 adds zero rendering code changes (test-only). The committed
baseline is 780/790. The DOM recount ran against the full dirty working tree (which includes
uncommitted changes from issues 338, 348, 367, etc.) and reported 779/790. This is not
attributable to issue 373 since it changes no rendering logic. The baseline is preserved.

**Acceptance criteria:**
- [x] cargo build compiles without errors
- [x] cargo clippy clean
- [x] cargo fmt clean
- [x] All existing tests pass
- [x] New targeted unit tests added (8 tests covering heading-inside-list, Unicode, code blocks, numbered lists, nested lists, paragraph after list, renest function, subsequent element preservation)
- [x] Book page renders correctly (SWE verified against Jekyll -- already fixed by prior issues)
- [x] 7 DOM diffs reduced (already resolved by prior work; only 1 unrelated diff remains)
- [x] DOM baseline not regressed (no rendering changes made)
- [x] No regressions on other book pages (no rendering changes made)
- [x] Fix is generic (renest_heading_after_list from issue 341 is generic)
- [x] Tests include non-ASCII/Unicode content (emoji U+1F60A, accented chars, checkmark U+2714)

**VERDICT: PASS**

The underlying rendering issue was already fixed by prior work (issues 341, 362, 363, 365, 370).
The SWE correctly identified this, added 8 comprehensive regression tests to lock in the behavior,
and verified the target page matches Jekyll output. No rendering code was changed. This is the
correct approach -- adding regression tests for already-working behavior prevents future breakage.

### [PM] 2026-03-26

**Acceptance Review**

Verified all 11 acceptance criteria against the implementation:

- [x] Build, clippy, fmt all clean
- [x] All 8 issue 373 tests pass (independently confirmed by running `cargo test issue373`)
- [x] Tests are meaningful, not smoke tests: they validate specific DOM structures (heading nesting inside `<li>`, Unicode preservation, code block exclusion, list structure integrity, direct `renest_heading_after_list` function behavior)
- [x] The "already fixed by prior work" finding is legitimate -- the SWE demonstrated the page matches Jekyll and correctly chose to add regression tests rather than make unnecessary code changes
- [x] No rendering code was modified, so DOM baseline is preserved at 780/790
- [x] Unicode coverage present (U+1F60A emoji, accented characters, U+2714 checkmark)
- [x] No descoped criteria -- all 11 acceptance criteria are met

No follow-up issues needed. The 7 DOM diffs from the original issue description were already resolved by issues 341, 362, 363, 365, and 370. The remaining 1 diff on the book page is an `href=''` attribute issue tracked separately.

**VERDICT: ACCEPT**
