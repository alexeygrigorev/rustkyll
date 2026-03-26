# Issue 370: DTC thread comment ordering / rendering in book archive

## Parent

Follow-up from #363 (RC-I).

## Problem

Two distinct rendering bugs affect book archive thread/reply sections:

### Sub-problem A: Numbered list not recognized in `newline_to_br | markdownify` pipeline

On `books/20230807-driving-data-quality-with-data-contracts.html` (16 diffs), the Toxicafunk question uses numbered items (`1.` / `2.`) and Andrew Jones's reply contains numbered sub-answers (`1.\n...` / `2.\n...`). Jekyll's kramdown renders these as `<ol><li>` elements. Our engine is failing to produce the expected `<ol><li>` structure -- the DOM diff shows all 16 differences are `missing_text` and `missing_element` inside `ol > li` selectors.

The root cause is likely that `newline_to_br` inserts `<br />\n` before the numbered lines, and when `markdownify` then processes the result, it does not recognize the `1.` / `2.` patterns as ordered list items because the preceding `<br />` tag disrupts the markdown block structure. Jekyll's kramdown handles this differently because it processes the original markdown with newlines intact, producing `<ol>` with `<li>` items that contain `<br>` within each item.

Key test text (from `_books/20230807-driving-data-quality-with-data-contracts.md`, Toxicafunk thread, Andrew Jones reply):
```
"Hey Toxicafunk,\n1. \nYes, there's a section in chapter 2 titled _Data contracts and the data mesh_ that aims to answer that...\n2.\nThat's a great question!..."
```

Expected HTML (from Jekyll): an `<ol>` with two `<li>` elements, each containing text with `<br>` and `<em>` inline elements.

### Sub-problem B: Thread header elements leak outside `<ul><li>` container

On `books/20241017-build-large-language-model-from-scratch.html` (7 diffs), `<h3>` and `<p>` elements that belong inside the comment container are rendering as siblings of the thread's parent container. The DOM diff shows:
- `ul > li > p`: missing (expected inside `<li>`)
- `ul > li > h3`: missing (expected inside `<li>`)
- `h3`: extra element (leaked outside)
- `p`: extra element (leaked outside)
- `div`: extra element (leaked outside)

This suggests that when `markdownify` produces block-level elements (like `<h3>` from markdown `###`), the browser or our HTML serialization breaks them out of inline context, causing them to render outside their intended container.

### Related pages with similar `ol > li` issues

The DOM diff also shows related numbered-list rendering failures on:
- `books/20240715-ai-data-privacy-and-protection.html` (12 diffs, same `ol > li` pattern)
- `books/20231106-analytics-engineering-with-sql-and-dbt.html` (8 diffs, similar `ol`/`div` mismatch)

These share the same root cause as Sub-problem A and should be fixed by the same change.

## Affected Pages

- `books/20230807-driving-data-quality-with-data-contracts.html` (16 diffs, Sub-problem A)
- `books/20241017-build-large-language-model-from-scratch.html` (7 diffs, Sub-problem B)
- `books/20240715-ai-data-privacy-and-protection.html` (12 diffs, related to Sub-problem A)
- `books/20231106-analytics-engineering-with-sql-and-dbt.html` (8 diffs, related to Sub-problem A)

## Reference

- Book layout template: `datatalksclub.github.io/_layouts/book.html`
- Archive rendering: `{% for thread in page.archive %}` with `{{ thread.text | newline_to_br | markdownify }}` and `{{ reply.text | newline_to_br | markdownify }}`
- DTC DOM baseline: 778/790 (from commit 7a5b0ce)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes clean
- [ ] `cargo fmt --check` passes
- [ ] Text containing `\n1. \n...` patterns processed through `newline_to_br | markdownify` produces `<ol><li>` structure matching Jekyll's kramdown output
- [ ] On `books/20230807-driving-data-quality-with-data-contracts.html`, the Andrew Jones reply to Toxicafunk renders as an `<ol>` with `<li>` items containing `<em>`, `<br>`, and text nodes matching the Jekyll reference
- [ ] On `books/20241017-build-large-language-model-from-scratch.html`, `<h3>` and `<p>` elements render inside the correct `<ul><li>` container, not as siblings outside it
- [ ] DTC DOM match count does not drop below 778/790
- [ ] DTC DOM match count improves (target: fix at least the 16 diffs on driving-data-quality and the 7 diffs on build-large-language-model)
- [ ] No site-specific hardcoding -- fix must be in the generic `markdownify` filter or `newline_to_br` pipeline, not special-cased for these pages
- [ ] Existing tests continue to pass (no regressions)
- [ ] Build the DTC site and inspect the generated HTML for the affected book pages to verify correct structure

## Test Scenarios

### Unit: newline_to_br + markdownify pipeline for numbered lists

- Input: `"Hey,\n1. \nYes, there's a section titled _Data contracts_.\n2.\nThat's a great question!"` processed through `newline_to_br` then `markdownify`. Verify output contains `<ol>` with two `<li>` elements.
- Input: text with `1.` at start after `<br />\n` -- verify `markdownify` still recognizes ordered list pattern.
- Input: text with no numbered items through `newline_to_br | markdownify` -- verify no spurious `<ol>` elements introduced.

### Unit: markdownify block elements inside inline context

- Input containing markdown heading (`### Heading`) processed through `markdownify` -- verify `<h3>` is produced and does not break out of container context.
- Input containing mixed inline and block markdown elements -- verify correct nesting.

### Integration: Book archive rendering

- Build the DTC site and check `books/20230807-driving-data-quality-with-data-contracts.html` for correct `<ol><li>` structure in the Toxicafunk/Andrew Jones thread.
- Build the DTC site and check `books/20241017-build-large-language-model-from-scratch.html` for correct `<h3>` and `<p>` nesting inside thread containers.
- Run DOM comparison and verify no regression below 778/790 baseline.

### Regression: Existing book pages

- Verify `books/20210222-ml-algotrading-2ed.html` still renders correctly (this was fixed by #362).
- Verify other book archive pages with threads do not regress.
- Include non-ASCII/Unicode content in at least one test (the archive text includes emoji like U+1F642 and smart quotes).

## Dependencies

- #363 (RC-I) must be done (or its relevant fixes already committed)

## Priority

MEDIUM

## Log

### [SWE] 2026-03-26

**TDD Cycle -- Sub-problem A:**

1. Wrote 5 failing tests in `src/template/filters/markdownify.rs`:
   - `test_issue370_numbered_list_continuation_inside_li` -- verifies "1. <br />\nYes, there's..." keeps content inside `<ol><li>`
   - `test_issue370_no_spurious_ol_without_numbers` -- ensures no false positives
   - `test_issue370_numbered_list_with_unicode_continuation` -- tests smart quotes + emoji in list items
   - `test_issue370_three_numbered_items_with_br` -- tests 1./2./3. consecutive pattern
   - `test_issue370_heading_inside_list_context` -- tests heading re-nesting (already passes from issue 341)

2. Ran tests: 2 FAILED as expected (continuation_inside_li, unicode_continuation) -- content leaked outside `<ol>` into `<p>`.

3. Implemented fix: `strip_br_from_empty_numbered_list_markers()` in `src/frontmatter.rs`.
   - When a line is ONLY `N. <br />` or `N.<br />` (numbered list marker + `<br />` with no other content), joins the next line onto the marker line: `1. <br />Yes, text...`.
   - This allows pulldown-cmark to keep the continuation text inside the `<li>` as a lazy continuation, matching kramdown's behavior.
   - Helper function `strip_leading_number_dot()` parses `N.` prefix.

4. Ran tests: All 5 PASS.

**Attempted fix for single-item numbered lists (analytics-engineering):**
- Changed `insert_paragraph_break_before_numbered_list` to insert paragraph break for single numbered items (not just 2+ consecutive).
- Result: DOM dropped from 780 to 776/790 -- widespread regressions on other book pages.
- Reverted. The single-item case requires a more targeted approach.

**Attempted fix for Sub-problem B (build-large-language-model):**
- Implemented `renest_block_elements_after_list()` to re-nest `<p>` and `<h3>` elements after `</li></ul>`.
- Result: DOM dropped from 780 to 779/790 -- caused regressions on machine-learning-zoomcamp and graph-algorithms pages.
- Reverted. The generic block re-nesting is too aggressive.

**Final state:**
- Sub-problem A (driving-data-quality): FIXED -- 16 diffs eliminated, page now matches Jekyll
- Sub-problem B (build-large-language-model): NOT FIXED -- requires more targeted approach without regressions
- analytics-engineering and ai-data-privacy: NOT FIXED -- single-item and interleaved list patterns need different approach

**Build results:**
- `cargo build`: OK
- `cargo clippy -- -D warnings`: clean
- `cargo fmt --check`: clean
- Tests: 3142 pass, 0 fail (2842 lib + 300 integration)
- DTC DOM: 780/790 (baseline was 778/790, net improvement of +2)

**Files modified:**
- `src/frontmatter.rs` -- added `strip_br_from_empty_numbered_list_markers()` and `strip_leading_number_dot()`, called from `markdown_to_html_for_filter()`
- `src/template/filters/markdownify.rs` -- added 5 tests for issue 370

**Known limitations:**
- Sub-problem B (build-large-language-model, 7 diffs) not fixed: generic block re-nesting causes regressions
- analytics-engineering (8 diffs) not fixed: single numbered item starting at 2 can't use paragraph break without regressions
- ai-data-privacy (12 diffs) not fixed: interleaved ol/ul nesting is a more complex problem

### [QA] 2026-03-26

**Verification of Sub-problem A fix and overall quality:**

1. Build: `cargo build --release` -- OK
2. Clippy: `cargo clippy -- -D warnings` -- clean (only upstream lint rename warnings in liquid-lib)
3. Formatting: `cargo fmt --check` -- clean
4. Tests: all pass, 0 failures (2842 lib + integration tests)
5. DTC DOM comparison: 780/790 (baseline 778/790, net +2 improvement, no regression)
6. `books/20230807-driving-data-quality-with-data-contracts.html`: 0 diffs -- confirmed not present in diff output

**Acceptance criteria review:**

- [PASS] `cargo build` compiles without errors
- [PASS] `cargo clippy -- -D warnings` passes clean
- [PASS] `cargo fmt --check` passes
- [PASS] `newline_to_br | markdownify` produces `<ol><li>` structure -- tested by `test_issue370_numbered_list_continuation_inside_li` and 4 other tests
- [PASS] `books/20230807-driving-data-quality-with-data-contracts.html` Andrew Jones reply renders correctly -- DOM shows 0 diffs for this page
- [NOT FIXED] `books/20241017-build-large-language-model-from-scratch.html` -- Sub-problem B not fixed (attempted, reverted due to regressions)
- [PASS] DTC DOM match count >= 778/790 -- confirmed 780/790
- [PARTIAL] DTC DOM match count improves -- 16 diffs on driving-data-quality eliminated (+2 net), but 7 diffs on build-large-language-model remain
- [PASS] No site-specific hardcoding -- fix is generic in `strip_br_from_empty_numbered_list_markers()` in frontmatter.rs
- [PASS] Existing tests continue to pass
- [PASS] DTC site built and inspected

**TDD verification:**
- SWE log shows: (1) wrote 5 tests first, (2) 2 failed as expected, (3) implemented fix, (4) all 5 pass. TDD cycle followed.
- Tests include Unicode content (smart quotes U+2019, emoji U+1F642) per project conventions.

**Code quality:**
- `strip_br_from_empty_numbered_list_markers()` is well-documented with doc comments explaining the problem and approach
- `strip_leading_number_dot()` is a clean helper with proper byte-level parsing
- No unwrap in library code; proper error handling
- No unnecessary dependencies
- Fix is in the generic markdown pipeline, not site-specific

**Follow-up issues needed:**
- Sub-problem B (build-large-language-model, 7 diffs): block elements leaking outside `<ul><li>` -- needs separate issue
- analytics-engineering (8 diffs): single numbered item recognition -- needs separate issue
- ai-data-privacy (12 diffs): interleaved ol/ul nesting -- needs separate issue

**VERDICT: PASS**

Sub-problem A is fully resolved with +2 net DOM improvement and no regressions. Sub-problems B and the related pages were attempted but caused regressions and were correctly reverted per the regression-safe investigation protocol. These need follow-up issues for the remaining 27 diffs across 3 pages.

### [PM] 2026-03-26

**Acceptance Review**

Criteria assessment:

- [PASS] `cargo build` compiles without errors
- [PASS] `cargo clippy -- -D warnings` passes clean
- [PASS] `cargo fmt --check` passes
- [PASS] `newline_to_br | markdownify` produces `<ol><li>` structure -- verified by 5 unit tests
- [PASS] driving-data-quality page: Andrew Jones reply renders correctly -- 16 diffs eliminated, 0 remaining
- [DESCOPED] build-large-language-model `<h3>`/`<p>` nesting -- attempted, reverted due to regressions. Tracked in new issue #373.
- [PASS] DTC DOM >= 778/790 -- confirmed 780/790 (+2 improvement)
- [PARTIAL] DOM improvement target -- 16 diffs fixed on driving-data-quality; 7 diffs on build-large-language-model remain (descoped to #373)
- [PASS] No site-specific hardcoding -- `strip_br_from_empty_numbered_list_markers()` is generic
- [PASS] Existing tests pass -- 3142 tests, 0 failures
- [PASS] DTC site built and inspected

Code quality:
- TDD cycle properly followed: 5 tests written first, 2 failed as expected, fix implemented, all pass
- Tests include Unicode content (smart quotes U+2019, emoji U+1F642) per project conventions
- Fix is clean and well-documented: `strip_br_from_empty_numbered_list_markers()` with helper `strip_leading_number_dot()`
- No unwrap in library code
- Reverted attempts were the right call -- regressions were caught and undone

Descoped items with follow-up issues:
- Sub-problem B (build-large-language-model, 7 diffs): tracked in **#373** (`373-dtc-block-elements-leaking-outside-list-containers.todo.md`)
- analytics-engineering (8 diffs): tracked in **#374** (`374-dtc-analytics-engineering-single-numbered-list.todo.md`)
- ai-data-privacy (12 diffs): already tracked in existing **#371** (`371-dtc-residual-numbered-list-rendering.todo.md`)

**VERDICT: ACCEPT**

Sub-problem A is fully resolved. The driving-data-quality page went from 16 diffs to 0, and overall DTC DOM improved from 778 to 780/790. The three unfixed items were properly attempted, correctly reverted when regressions appeared, and are now tracked in follow-up issues #371, #373, and #374. Engineer may commit.
