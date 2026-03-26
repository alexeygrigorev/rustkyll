# Issue 372: DTC blood cell classifier list paragraph wrapping

## Problem

`blog/how-to-build-blood-cell-classifier-for-cancer-prediction-case-study-from-ml-zoomcamp.html`
has 16 DOM differences against Jekyll. The diffs are all in nested sub-list items
where Jekyll wraps content in `<p>` tags but rustkyll renders as bare text.

Pattern: `expected_element_got_text` and `missing_element` -- Jekyll produces `<li><p>text</p></li>`
but rustkyll produces `<li>text</li>` (tight vs loose list rendering).

## Root Cause Analysis

The source markdown contains two nested sub-lists where items are separated by blank lines:

**List 1** (lines ~104-109): Sub-list under "Malignant" describing ALL subtypes:
```markdown
- **Malignant:** ALL-related cells, categorized into three subtypes of malignant lymphoblasts:
  - Early Pre-B

  - Pre-B

  - Pro-B
```

**List 2** (lines ~133-142): Sub-list under "Data augmentation" describing techniques:
```markdown
- **Data augmentation:** Various augmentation techniques were applied, including:
  - Rotations

  - Flips

  - Brightness variations

  - Contrast variations

  - Saturation level changes
```

In kramdown (Jekyll's markdown engine), blank lines between sub-list items make each sub-list item "loose", wrapping its content in `<p>`. Rustkyll currently has two interacting behaviors that prevent this:

1. `collapse_blank_lines_between_list_items()` in `src/kramdown.rs` removes blank lines between items in "partially loose" list regions, forcing them tight before pulldown-cmark sees them.
2. `mark_simple_partial_loose_list_items()` only marks items containing inline markdown links (`](`) for `<p>` wrapping -- plain text sub-list items like "Early Pre-B" or "Rotations" are never marked.

Issue #343 introduced the partial-loose mechanism but intentionally scoped it narrowly to items with inline links. This issue must broaden the scope to also handle plain-text sub-list items in partially-loose regions without regressing other pages.

## DOM Diffs (all 16)

From `docs/comparison/dom-details/DataTalksClub-datatalksclub.github.io.txt`:
```
body > div > div > div > div > ul > li > ul > li: expected_element_got_text - expected: '<p>', actual: 'Early Pre-B'
body > div > div > div > div > ul > li > ul > li > p: missing_element - expected: '<p>', actual: '(none)'
body > div > div > div > div > ul > li > ul > li: expected_element_got_text - expected: '<p>', actual: 'Pre-B'
body > div > div > div > div > ul > li > ul > li > p: missing_element - expected: '<p>', actual: '(none)'
body > div > div > div > div > ul > li > ul > li: expected_element_got_text - expected: '<p>', actual: 'Pro-B'
body > div > div > div > div > ul > li > ul > li > p: missing_element - expected: '<p>', actual: '(none)'
body > div > div > div > div > ul > li > ul > li: expected_element_got_text - expected: '<p>', actual: 'Rotations'
body > div > div > div > div > ul > li > ul > li > p: missing_element - expected: '<p>', actual: '(none)'
body > div > div > div > div > ul > li > ul > li: expected_element_got_text - expected: '<p>', actual: 'Flips'
body > div > div > div > div > ul > li > ul > li > p: missing_element - expected: '<p>', actual: '(none)'
... and 6 more differences (Brightness variations, Contrast variations, Saturation level changes)
```

Each sub-list item accounts for 2 diffs: one `expected_element_got_text` (text found where `<p>` was expected) and one `missing_element` (the `<p>` element itself is absent). 3 items in list 1 + 5 items in list 2 = 8 items x 2 diffs = 16 total.

## Scope

1. Reproduce the tight/loose sub-list mismatch on this specific page with a targeted unit test
2. Broaden the partial-loose marking in `mark_simple_partial_loose_list_items()` to also cover plain-text sub-list items (not just items with inline links), or use an alternative approach that produces the correct `<li><p>text</p></li>` output for these cases
3. The fix must be scoped carefully -- only sub-list items in partially-loose regions where blank lines separate the items should gain `<p>` wrapping
4. Must not regress DTC DOM baseline (776/790 from commit da6832a)

## Acceptance Criteria

- [ ] A targeted unit test reproduces the exact pattern: nested sub-list items separated by blank lines should render as `<li><p>text</p></li>`, not `<li>text</li>`. The test must fail before the fix and pass after.
- [ ] The generated HTML for `blog/how-to-build-blood-cell-classifier-for-cancer-prediction-case-study-from-ml-zoomcamp.html` eliminates all 16 DOM diffs (or any residual diffs are explicitly split into a follow-up issue with justification).
- [ ] The fix handles both plain-text sub-list items (like "Early Pre-B") and sub-list items with inline formatting (like bold/italic), not just items with inline links.
- [ ] Existing partial-loose tests from issue #343 (`test_issue343_partial_loose_first_item_wrapped_only`, `test_issue204_kramdown_per_item_loose_tight`) continue to pass.
- [ ] `cargo build` compiles without errors.
- [ ] `./scripts/cargo-safe test` passes with no new failures.
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes (ignoring pre-existing external warnings).
- [ ] The repo-wide DTC DOM match count does not drop below 776/790.
- [ ] The issue log records before/after diff count for the target page and the repo-wide DTC DOM baseline.

## Test Scenarios

### Unit: nested sub-list partial-loose wrapping

- Parse markdown with a parent list item containing a nested sub-list where sub-items are separated by blank lines (no inline links, just plain text). Verify each sub-item renders as `<li><p>text</p></li>`.
- Parse the same pattern but with inline bold formatting in sub-items (e.g., `  - **Bold item**`). Verify `<p>` wrapping still applies.
- Parse a nested sub-list where items are NOT separated by blank lines (tight). Verify they render as `<li>text</li>` without `<p>` wrapping (no false positives).

### Unit: regression guard for issue #343

- Re-run `test_issue343_partial_loose_first_item_wrapped_only` and verify it still passes (items with inline links in partial-loose regions must still get `<p>` wrapping).
- Re-run `test_issue204_kramdown_per_item_loose_tight` and verify it still passes.

### Integration: page comparison

- Build the DTC site and compare `blog/how-to-build-blood-cell-classifier-for-cancer-prediction-case-study-from-ml-zoomcamp.html` against the cached Jekyll output.
- Verify the nested sub-list structures (ALL subtypes and augmentation techniques) match Jekyll's HTML, with each sub-item wrapped in `<li><p>...</p></li>`.

### Integration: regression check

- Re-run the full DTC DOM comparison after the fix and confirm the repo-wide baseline remains at or above 776/790.
- Spot-check other pages with nested lists (e.g., `blog/guidelines-to-get-data-engineer-job-against-odds.html`) to confirm no regression.

## Dependencies

- Issue #343 (partial-loose list wrapping) -- DONE. This issue builds on the mechanism introduced there.

## DTC DOM Baseline

- 776/790 from commit da6832a

## Log

### [PM] 2026-03-26
- Groomed the issue with detailed root cause analysis tracing the problem to `mark_simple_partial_loose_list_items()` in `src/kramdown.rs` (line 2231: `has_inline_markdown_link` guard excludes plain-text sub-list items).
- Identified two affected lists in the source markdown: ALL subtypes (3 items) and augmentation techniques (5 items), totaling 8 sub-list items x 2 diffs = 16 DOM differences.
- Added acceptance criteria requiring fail-first TDD, full elimination of the 16 diffs, regression guards for #343 tests, and the DTC DOM baseline gate at 776/790.
- Recorded DTC DOM baseline: 776/790 from commit da6832a.

### [SWE] 2026-03-26
- Wrote 3 failing tests (TDD): `test_issue372_nested_sublist_plain_text_loose`, `test_issue372_nested_sublist_bold_text_loose`, `test_issue372_nested_sublist_tight_no_wrapping`
- Ran tests: 2 FAIL as expected (plain text and bold sub-list items not wrapped in `<p>`), 1 PASS (tight sub-list correctly not wrapped)
- Root cause confirmed: `collapse_blank_lines_between_list_items()` removes blank lines between sub-list items in partial-loose regions, making them tight. The `mark_simple_partial_loose_list_items()` only marks items with inline links.
- Implemented fix: added `find_locally_loose_subgroup_lines()` function to detect contiguous groups of indented sub-list items at the same indent level where ALL consecutive pairs have blank lines between them. Modified `collapse_blank_lines_between_list_items()` to preserve blank lines for these groups, allowing pulldown-cmark to render them as loose (with `<p>` wrapping).
- Key design decision: did NOT modify `mark_simple_partial_loose_list_items()` which handles case 1 (inline links in partial-loose regions, issue #343). Instead, implemented case 2 (locally fully-loose sub-groups) as blank-line preservation in the collapse function, letting pulldown-cmark handle `<p>` wrapping naturally.
- Also fixed `wrap_marked_partial_loose_list_items()` to strip trailing space before the marker during cleanup.
- Ran tests: all 3 issue 372 tests PASS
- Regression tests: `test_issue343_partial_loose_first_item_wrapped_only` PASS, `test_issue204_kramdown_per_item_loose_tight` PASS
- Full test suite: 2827 lib + all integration tests pass, 0 failures
- Clippy: clean (no warnings)
- Fmt: clean
- Build: release build successful
- DTC DOM baseline: 778/790 (up from 776/790, +2 pages fixed)
- Blood cell classifier page: 0 DOM diffs (all 16 eliminated)
- Files modified: `src/kramdown.rs`

### [QA] 2026-03-26
- Release build: success
- Full test suite: all 2827 lib tests pass, 0 failures
- Clippy: clean (only pre-existing liquid-lib warnings)
- Fmt: clean
- Issue 372 tests: 3/3 pass (plain text loose, bold text loose, tight no wrapping)
- Issue 343 regression: `test_issue343_partial_loose_first_item_wrapped_only` PASS
- Issue 204 regression: `test_issue204_kramdown_per_item_loose_tight` PASS
- DTC DOM comparison: 778/790 (baseline 776/790, +2 improvement, no regression)
- Blood cell classifier page: not in DOM diff details file (0 diffs confirmed)
- Verified generated HTML: `<li><p>Early Pre-B</p></li>`, `<li><p>Rotations</p></li>`, etc. all correct
- TDD log verified: SWE wrote tests first, confirmed 2 failures, then implemented fix
- Acceptance criteria 1-9: all PASS
- VERDICT: PASS

### [PM] 2026-03-26 -- Acceptance Review

**VERDICT: ACCEPT**

All 9 acceptance criteria verified:

1. TDD followed -- 3 tests written first, 2 failed before fix, all 3 pass after. Confirmed by SWE log and QA.
2. All 16 DOM diffs on the blood cell classifier page eliminated (0 remaining).
3. Fix covers both plain-text sub-list items and items with inline bold formatting, as demonstrated by the two test cases.
4. Regression tests for issues #343 and #204 pass -- independently verified by PM.
5. `cargo build` compiles without errors.
6. Full test suite: 2827 lib tests, 0 failures.
7. Clippy clean (no project warnings).
8. DTC DOM baseline improved from 776/790 to 778/790 (+2, no regression).
9. Before/after counts recorded in issue log.

Implementation review: The `find_locally_loose_subgroup_lines()` function is well-scoped -- it detects contiguous indented sub-list groups where all consecutive pairs have blank lines, and preserves those blank lines so pulldown-cmark handles `<p>` wrapping naturally. This avoids adding another post-processing workaround and integrates cleanly with the existing `collapse_blank_lines_between_list_items()` logic. The trailing space marker fix in `wrap_marked_partial_loose_list_items()` is a minor correctness improvement.

No descoped items. All criteria met. Ready to commit.
