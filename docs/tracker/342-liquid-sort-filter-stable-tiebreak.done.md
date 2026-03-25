# Issue 342: Liquid sort filter stable tiebreak (match Jekyll)

## Problem

The Liquid `sort` filter in rustkyll does not match Jekyll's stable sort behavior for equal values. When a Liquid template does `site.posts | sort: "date" | reverse`, posts with the same date appear in a different order than Jekyll produces.

This was discovered in issue 337 sub-issue A: the `free-machine-learning-courses.html` page uses a custom `related-posts.html` include that sorts via Liquid `sort: "date" | reverse`. The 6 DOM diffs on this page are caused by the Liquid sort filter's tiebreak behavior, not by `site.related_posts`.

Jekyll's Liquid sort filter uses a stable sort, so equal-key items retain their original order from `site.posts` (which is date descending, path ascending for same-date). Rustkyll's Liquid sort filter does not preserve this stability.

## Affected pages

- `blog/free-machine-learning-courses.html` (6 DOM diffs from this issue)

## Origin

Descoped from issue 337 sub-issue A. Original acceptance criteria required 0 DOM diffs on this page, but the root cause was misdiagnosed as FAQ accordion whitespace / `site.related_posts` tiebreak.

## Scope

1. Reproduce the sort-order mismatch on `blog/free-machine-learning-courses.html`.
2. Fix the Liquid `sort` filter so equal-key values preserve Jekyll's stable ordering behavior.
3. Verify the related-post ordering on `blog/free-machine-learning-courses.html` matches the cached Jekyll output.
4. Keep the repo-wide DTC DOM baseline from regressing while this page-specific diff is removed.

## Baseline

- DTC DOM baseline: `766/790`

## Acceptance Criteria

- [ ] The Liquid `sort` filter preserves stable ordering for equal-key values so that `sort: "date" | reverse` matches Jekyll on equal dates.
- [ ] `blog/free-machine-learning-courses.html` reaches `0` DOM diffs for the previously reported related-post ordering mismatch, or any residual diff is explicitly split into a new follow-up issue before acceptance.
- [ ] A regression test proves the stable-tiebreak behavior: it fails before the fix and passes after the fix.
- [ ] The issue log records the before/after diff count for `blog/free-machine-learning-courses.html` and the specific ordering behavior that changed.
- [ ] The repo-wide DTC DOM match count does not drop below `766/790`.

## Test Scenarios

### Unit: stable sort behavior
- Add a targeted test for Liquid `sort` stability with equal values and verify the original order is preserved before `reverse` is applied.
- Confirm the test fails before the fix and passes after the fix.

### Integration: page comparison
- Build the DTC site and compare `blog/free-machine-learning-courses.html` against the cached Jekyll output.
- Verify the related-post links, titles, and descriptions appear in the same order as Jekyll.

### Integration: regression check
- Re-run the full DTC DOM comparison after the fix and confirm the repo-wide baseline stays at or above `766/790`.
- If any page-level diff remains, create a follow-up issue that references `#342` rather than silently dropping it.

## Dependencies

- None

## Log

### [PM] 2026-03-25
- Groomed the issue into a single-page DTC DOM parity target for `blog/free-machine-learning-courses.html`.
- Recorded the repo-wide DTC baseline at `766/790`.
- Added explicit acceptance criteria for stable Liquid `sort` behavior, page-level zero-diff verification, and fail-then-pass regression coverage.

### [SWE] 2026-03-25 09:32
- Picked up the issue and renamed it to `.in-progress.md` with `git mv`.
- Investigated `src/template/filters/sort.rs`, `src/generator.rs`, and the rendered `related-posts.html` output for `blog/free-machine-learning-courses.html`.
- Wrote targeted regression tests first for equal-key ordering in `src/template/filters/sort.rs` and verified they FAIL under the alternate implementation hypothesis:
  - `test_sort_tiebreaks_equal_property_values_by_slug_descending`
  - `test_sort_tiebreaks_equal_property_values_by_path_descending`
  - Failure output with the alternate implementation showed equal-key items ordered as `["earlier", "data-science-interview", "data-translator"]` instead of `["earlier", "data-translator", "data-science-interview"]`, and `["_podcast/aaa.md", "_podcast/zzz.md"]` instead of `["_podcast/zzz.md", "_podcast/aaa.md"]`.
- Implemented the alternate descending slug/path tiebreak in `sort.rs` and reran the targeted tests: PASSES (`2` passed, `0` failed).
- Rebuilt DTC with the alternate implementation and re-ran DOM comparison:
  - `blog/free-machine-learning-courses.html` disappeared from the diff output, confirming the page-level hypothesis.
  - But the repo-wide DTC result regressed to `764 files matched, 26 files with differences, 581 total differences`, below the `766/790` baseline.
- Reverted the experimental `sort.rs` change after verification so the worktree does not keep a regressive implementation.
- Current conclusion: the page-level mismatch is tied to equal-key ordering, but fixing it in the global Liquid `sort` filter causes broader regressions across other DTC pages. This issue remains blocked pending a narrower fix or a revised root-cause analysis.

### [Orchestrator] 2026-03-25
- Confirmed the exact page-level mismatch on `blog/free-machine-learning-courses.html` by comparing the rendered related-posts grid in the cached Jekyll output vs rustkyll output.
- `_includes/related-posts.html` builds `related_posts` by iterating `site.posts limit: 20`, collecting tag matches, then applying `sort: "date" | reverse` before slicing the first 3 posts.
- Jekyll related-post order:
  1. `/blog/data-engineering-zoomcamp.html`
  2. `/blog/building-discipline-in-machine-learning-with-ml-zoomcamp.html`
  3. `/blog/how-to-build-blood-cell-classifier-for-cancer-prediction-case-study-from-ml-zoomcamp.html`
- rustkyll related-post order:
  1. `/blog/data-engineering-zoomcamp.html`
  2. `/blog/how-to-build-blood-cell-classifier-for-cancer-prediction-case-study-from-ml-zoomcamp.html`
  3. `/blog/building-discipline-in-machine-learning-with-ml-zoomcamp.html`
- The reproduction is now narrowed to a swap of items 2 and 3 only. This suggests the remaining mismatch is in the relative order of two equal-date posts within the `site.posts -> sort: "date" -> reverse` pipeline, not in the broader include structure.

### [SWE] 2026-03-25 11:24
- Continued from the blocked state with the narrower hypothesis that the mismatch might come from stable sort semantics, collection input order, or both.
- Wrote fail-first unit tests in `src/template/filters/sort.rs` proving that equal property values must preserve input order:
  - `test_sort_preserves_input_order_for_equal_property_values`
  - `test_sort_preserves_input_order_for_equal_property_values_with_paths`
  - Verified FAIL before the change, with equal-key items reordered as `["earlier", "data-science-interview", "data-translator"]` and `["_podcast/aaa.md", "_podcast/zzz.md"]`.
- Implemented the narrow `sort` fix by removing the global slug/path tiebreak from Liquid `sort` and keeping only the stable primary-key comparison.
- Rebuilt DTC and verified the target page order now matched Jekyll, but the repo-wide DOM still regressed to `764 files matched, 26 files with differences, 581 total differences`.
- Investigated the regression pages and found the new failures clustered around podcast templates that do `site.podcast | sort: "episode"`. That isolated a second root cause: rustkyll was pre-sorting non-post collections by `date`, so stable Liquid sort was preserving the wrong input order for duplicate episode numbers.
- Added a second fail-first regression test in `src/collection.rs`:
  - `test_load_podcast_preserves_source_path_order_without_sort_by`
  - Verified FAIL before the change, with `["data-translator-role-and-data-strategy", "data-science-interview-and-cv-guide"]` loaded in the wrong order.
- Implemented the kept fix in `src/collection.rs`: only `posts` receive implicit date-based ordering; non-post collections now preserve source-path order unless the site explicitly configures `sort_by`.
- Re-ran the targeted regression tests:
  - `./scripts/cargo-safe test --lib template::filters::sort` -> PASS (`20` passed, `0` failed)
  - `./scripts/cargo-safe test --lib test_load_podcast_preserves_source_path_order_without_sort_by` -> PASS (`1` passed, `0` failed)
- Rebuilt rustkyll in release mode and re-ran the DTC checks:
  - Page-only DOM compare for `blog/free-machine-learning-courses.html`: `1 files matched, 0 files with differences, 0 total differences`
  - Repo-wide DTC DOM compare: `769 files matched, 21 files with differences, 563 total differences`
- Final conclusion: the safe, Jekyll-accurate fix required both parts together:
  1. Liquid `sort` must be stable for equal values.
  2. Non-post collections must keep their natural/source-path order unless `sort_by` is configured.
- This keeps the target page at `0` diffs and improves the repo-wide DTC baseline from `766/790` to `769/790`, so the issue is no longer blocked on a regressive implementation.

### [QA] 2026-03-25
- Tests: `./scripts/cargo-safe test --lib template::filters::sort` passed (`20` passed, `0` failed); `./scripts/cargo-safe test --lib test_load_podcast_preserves_source_path_order_without_sort_by` passed (`1` passed, `0` failed); full `./scripts/cargo-safe test` passed (`0` failures across lib, main, integration, and doc-test targets).
- Clippy: clean (`./scripts/cargo-safe clippy -- -D warnings` passed).
- Fmt: FAIL (`cargo fmt --check` reported formatting diffs in `src/template/filters/sort.rs`).
- DTC build/output: `./scripts/cargo-safe build --release` passed; `./target/release/rustkyll build --source websites/DataTalksClub/datatalksclub.github.io --destination /tmp/dtc_qa_issue342` passed.
- Page-level verification: `blog/free-machine-learning-courses.html` related-post order now matches Jekyll exactly:
  1. `/blog/data-engineering-zoomcamp.html`
  2. `/blog/building-discipline-in-machine-learning-with-ml-zoomcamp.html`
  3. `/blog/how-to-build-blood-cell-classifier-for-cancer-prediction-case-study-from-ml-zoomcamp.html`
- Page-only DOM compare: `1 files matched, 0 files with differences, 0 total differences`.
- Repo-wide DTC DOM comparison: `769 files matched, 21 files with differences, 563 total differences (3026 acceptable diffs filtered out)`; baseline `766/790` maintained and improved.
- TDD compliance: PASS — the SWE log shows fail-first tests for the kept `sort` and collection-order fixes before implementation, then passing tests after the fix.
- DTC build performance: FAIL — this QA build reported `Time: 2.95s` with `Generation: 1.819s`, above the process limit of `1.0s`.
- Acceptance criteria: stable ordering PASS; target page zero diffs PASS; regression test fail-then-pass PASS; before/after diff logging PASS; repo-wide baseline PASS.
- VERDICT: FAIL
- Specific issues:
  1. Run formatting on `src/template/filters/sort.rs` so `cargo fmt --check` passes.
  2. Investigate or explain the DTC performance regression before this issue can pass under the current process rules.

### [QA] 2026-03-25 19:28 CET
- Re-ran `342` in isolation after the `340` and `343` commits by creating a clean worktree at `28064ae` and applying only the current `src/template/filters/sort.rs` and `src/collection.rs` patch.
- TDD compliance: PASS. The kept fix still has explicit fail-first coverage for both parts of the bug:
  - stable Liquid sort behavior for equal property values
  - non-post collection load order preservation without `sort_by`
- Isolated verification:
  - `cargo fmt --check -- src/template/filters/sort.rs src/collection.rs` -> PASS
  - `./scripts/cargo-safe test --lib template::filters::sort` -> PASS (`20 passed`)
  - `./scripts/cargo-safe test --lib test_load_podcast_preserves_source_path_order_without_sort_by` -> PASS (`1 passed`)
  - `./scripts/cargo-safe build --release` -> PASS
- Target page verification:
  - page-only compare for `blog/free-machine-learning-courses.html` -> `1 files matched, 0 files with differences, 0 total differences`
  - related-post ordering matches Jekyll exactly under the current pushed baseline
- Repo-wide DTC DOM verification in isolation (`HEAD + 342` only):
  - `771 files matched`
  - `19 files with differences`
  - `489 total differences`
  - `3026 acceptable diffs filtered out`
  - this improves over both the issue baseline (`766/790`) and the current pushed baseline without `342` (`770/790`)
- Performance note:
  - the isolated DTC build reported `Time: 1.13s` with `Generation: 0.701s`
  - this is materially better than the earlier suspect `2.95s` QA result and no longer indicates a `342`-specific regression
- Acceptance criteria:
  - Criterion 1: PASS
  - Criterion 2: PASS
  - Criterion 3: PASS
  - Criterion 4: PASS
  - Criterion 5: PASS
- VERDICT: PASS

### [PM] 2026-03-25 19:30 CET
- Reviewed the refreshed isolated QA run against the new pushed baseline after `340` and `343`.
- Product outcome is correct and complete for this issue:
  - the Liquid `sort` filter now preserves input order for equal values
  - non-post collections no longer get an implicit date sort unless explicitly configured
  - `blog/free-machine-learning-courses.html` now compares clean against cached Jekyll output
- Repo-wide DTC impact is positive and non-regressive:
  - pushed baseline without `342`: `770/790`
  - isolated `HEAD + 342`: `771/790`
- The earlier QA fail is resolved in substance:
  - formatting now passes
  - the prior performance complaint does not reproduce in the refreshed isolated run
- Acceptance criteria review:
  - Criterion 1: PASS
  - Criterion 2: PASS
  - Criterion 3: PASS
  - Criterion 4: PASS
  - Criterion 5: PASS
- VERDICT: ACCEPT
