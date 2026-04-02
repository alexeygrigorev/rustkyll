# Issue 476: muan-blog details element wrapping

## Problem

Text content inside single-line `<details><summary>...</summary>text</details>` blocks gets incorrectly wrapped in `<p>` tags by rustkyll, while Jekyll/kramdown preserves them as raw HTML.

### Jekyll output (correct)
```html
<details><summary>Content warning</summary>Some text here.</details>
```

### Rustkyll output (incorrect)
```html
<details><summary>Content warning</summary>
<p>Some text here.</details></p>
```

The `</details>` ends up inside the `<p>` tag, which is malformed HTML and produces DOM differences.

## Root Cause

The `split_text_after_html_block_close` function in `src/kramdown.rs` lists `</summary>` in `BLOCK_CLOSE_SPLIT_TAGS`. When it encounters `<details><summary>Text</summary>content</details>` on a single line, it inserts a blank line after `</summary>`:

```
<details><summary>Text</summary>

content</details>
```

Pulldown-cmark then treats the blank line as a markdown paragraph boundary, wrapping `content</details>` in `<p>` tags.

Multi-line `<details>` blocks (where `</details>` is on its own line, separated by blank lines) work correctly because pulldown-cmark correctly wraps the inner paragraphs in `<p>` -- matching what Jekyll does.

## Affected Files

6 notes in `websites/muan-blog/_notes/` have single-line `<details>` patterns:
- `2023-09-25-ee.md`
- `2024-10-28-oo.md`
- `2024-11-05-uu.md`
- `2024-11-06-uu.md`
- `2025-05-04-aa.md`
- `2025-07-24-aa.md`

These appear in `notes.html` (the aggregate page) and individual note pages. The DOM comparison currently shows 4 diffs in `notes.html` related to this issue (2 notes with extra `<p>` wrapping x 2 diffs each).

## Scope

Modify `split_text_after_html_block_close` in `src/kramdown.rs` so that it does NOT split after `</summary>` when the remainder of the line contains `</details>` (i.e., the entire `<details>...<summary>...</summary>content</details>` is on a single line or the `</details>` close is on the same line as `</summary>`).

The fix must be generic (not site-specific) -- it should handle any single-line `<details>` block, not just muan-blog content.

## Dependencies

None.

## Baseline

- DTC DOM: 790/790 pages, 596 matched (194 with diffs, 255 total diffs). Must not regress.
- muan-blog DOM: 39 common files, 36 matched, 3 with diffs, 1819 total diffs.
  - notes.html: 5 diffs (4 are `<details>` `<p>` wrapping, 1 is unrelated pre/div)
  - This issue targets fixing the 4 `<details>`-related diffs in notes.html

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes all existing tests plus new tests
- [ ] Single-line `<details><summary>X</summary>text</details>` is preserved as raw HTML (no `<p>` wrapping)
- [ ] Multi-line `<details>` blocks with blank lines still get correct `<p>` wrapping (no regression)
- [ ] `<details>` blocks where `</summary>` and `</details>` are on different lines still work correctly
- [ ] DTC DOM match count does not drop below 596/790 matched pages (255 total diffs or fewer)
- [ ] muan-blog notes.html `<details>`-related diffs reduced from 4 to 0
- [ ] Individual note pages for the 6 affected notes render `<details>` content without extra `<p>` tags

## Test Scenarios

### Unit: split_text_after_html_block_close

- Single-line `<details><summary>CW</summary>text</details>` -- must NOT be split; output equals input
- Single-line with Unicode: `<details><summary>Warning</summary>Chinese text here.</details>` -- preserved verbatim
- Multi-line with closing `</details>` on separate line -- `</summary>` still gets split (existing behavior preserved)
- `</summary>` followed by text WITHOUT `</details>` on the same line -- still gets split (existing behavior)
- `<details><summary>X</summary>text</details>` followed by more markdown text on next line -- details preserved, following text still parsed as markdown

### Unit: end-to-end kramdown rendering

- Render single-line `<details><summary>CW</summary>text</details>` through full pipeline -- output must NOT contain `<p>` inside `<details>`
- Render multi-line `<details>` with blank lines -- output must still have `<p>` wrapping inside (matching Jekyll)
- Render `<details>` followed by a regular paragraph -- both the details and paragraph must render correctly

### Integration: muan-blog output verification

- Build muan-blog site and check `notes.html` for `<details>` elements -- no spurious `<p>` wrapping
- Verify individual note pages (e.g., notes/2025-07-24-aa) render correctly
- DTC site build still produces same or better DOM match count

## Log

### [PM] 2026-04-02 grooming
- Read issue, investigated source code in `src/kramdown.rs`
- Identified root cause: `split_text_after_html_block_close` splits after `</summary>` even for single-line `<details>` blocks
- Found 6 affected notes in muan-blog with single-line `<details>` pattern
- Confirmed 4 DOM diffs in notes.html are from this issue
- DTC baseline: 596/790 matched, 255 total diffs
- muan-blog baseline: 36/39 matched, 1819 total diffs (notes.html has 5 diffs, 4 from this issue)

### [SWE] 2026-04-02

**Fix 1: Skip split after </summary> when </details> is on same line**
- Wrote 6 tests: test_issue476_single_line_details_not_split, test_issue476_single_line_details_unicode, test_issue476_multiline_details_summary_still_splits, test_issue476_summary_without_details_still_splits, test_issue476_single_line_details_no_p_wrapping, test_issue476_details_followed_by_paragraph
- Ran tests: 4 FAIL (single_line_details_not_split, single_line_details_unicode, single_line_details_no_p_wrapping, details_followed_by_paragraph), 2 PASS (multiline and summary-without-details already worked)
- Failure example: got `"<details><summary>Content warning</summary>\n\nSome text here.</details>"`, expected input unchanged
- Implemented fix in src/kramdown.rs:2198 -- added guard in split_text_after_html_block_close: when matched tag is `</summary>`, check if `</details>` appears on the same line (before next newline); if so, skip the split
- Ran tests: all 6 PASS

**Summary:**
- Files modified: src/kramdown.rs
- Tests added: 6 unit tests for single-line details element handling
- Build results: 3556 pass, 1 pre-existing fail (test_link_tag_pretty_permalink_html_page -- unrelated), clippy clean, fmt clean
- DTC DOM: 596/790 matched, 194 with diffs, 255 total diffs (matches baseline exactly)
- DTC build time: 0.611s (under 1.0s threshold)
- Known limitations: none

### [QA] 2026-04-02
- Tests: 3557 passed, 0 failed, 2 ignored (main crate); all integration test crates also pass
- Clippy: clean (only upstream liquid-lib warnings)
- Fmt: clean
- DTC DOM: 596/790 matched, 194 with diffs, 255 total diffs -- matches baseline exactly, no regression
- DTC build time: 0.565s (under 1.0s threshold)
- muan-blog DOM: 36/39 matched, 3 with diffs, 1815 total diffs (was 1819 -- improved by 4, matching the targeted details diffs)
- notes.html: 0 details/summary-related diffs remaining (was 4)
- TDD compliance: SWE log shows 6 tests written first, 4 failed as expected, fix applied, all 6 pass -- COMPLIANT
- Acceptance criteria:
  - [PASS] cargo build compiles without errors
  - [PASS] cargo test passes all existing tests plus 6 new tests
  - [PASS] Single-line details preserved as raw HTML (no p wrapping) -- verified via test_issue476_single_line_details_no_p_wrapping
  - [PASS] Multi-line details blocks still get correct p wrapping -- verified via test_issue476_multiline_details_summary_still_splits
  - [PASS] details blocks where summary and details close on different lines work correctly
  - [PASS] DTC DOM match count: 596/790 matched, 255 total diffs -- matches baseline
  - [PASS] muan-blog notes.html details-related diffs reduced from 4 to 0
  - [PASS] Individual note pages render details content without extra p tags (verified via end-to-end test)
- VERDICT: PASS

### [PM] 2026-04-02 13:10
- Reviewed diff: 1 file changed (src/kramdown.rs), 98 insertions (10-line fix + 85 lines of tests)
- Output verification: built DTC site independently, ran dom_compare.py -- 596/790 matched, 255 total diffs (matches baseline exactly)
- Results verified: muan-blog improved from 1819 to 1815 total diffs (4 details-related diffs fixed), DTC unchanged
- Code review: fix is 10 lines in split_text_after_html_block_close, well-commented, generic (not site-specific), correctly scoped
- Tests: 6 new unit tests covering single-line preservation, Unicode, multi-line regression, end-to-end pipeline, paragraph interaction
- TDD compliance: confirmed from SWE log (4 tests failed before fix, all 6 pass after)
- Acceptance criteria: all 8/8 met
- Follow-up issues created: none needed
- VERDICT: ACCEPT
