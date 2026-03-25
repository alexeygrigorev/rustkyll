# Issue 339: DTC blog canvas data attributes and LLM tools page (1 page)

## Problem

`blog/how-do-professionals-use-llm-tools-and-frameworks.html` (9 diffs)

The page uses `<canvas>` elements with custom `data-*` attributes (`data-type="bar"`, `data-orientation="horizontal"`, `data-title="..."`) that are stripped by rustkyll. Jekyll preserves them.

Also has `<figcaption>` ordering differences relative to `<canvas>` elements.

## Root cause

HTML sanitization or attribute handling strips `data-*` attributes from `<canvas>` and possibly other elements. Jekyll's kramdown passes all attributes through.

## Scope

1. Reproduce and isolate the remaining diffs on `blog/how-do-professionals-use-llm-tools-and-frameworks.html`.
2. Fix the rendering path so valid raw `<canvas>` blocks preserve their `data-*` attributes in rustkyll output.
3. Match Jekyll's `<figcaption>` ordering for the affected chart blocks.
4. Verify the resulting output against the Jekyll reference page and keep the repo-wide DTC baseline stable.

## Baseline

- DTC DOM baseline: `766/790`

## Acceptance Criteria

- [ ] `blog/how-do-professionals-use-llm-tools-and-frameworks.html` renders with the same canvas/chart structure as Jekyll, including the `class="ai-chart"` elements and their `data-type`, `data-orientation`, and `data-title` attributes.
- [ ] The affected `<figcaption>` elements appear in the same relative order as the Jekyll page.
- [ ] The page-level DOM comparison for `blog/how-do-professionals-use-llm-tools-and-frameworks.html` reaches `0` diffs or otherwise matches the Jekyll reference exactly for the previously reported canvas/figcaption discrepancies.
- [ ] Any regression tests added for this issue fail before the fix and pass after the fix.
- [ ] The repo-wide DTC DOM match count does not drop below `766/790`.

## Test Scenarios

### Integration: page comparison
- Build the DTC site with rustkyll and compare `blog/how-do-professionals-use-llm-tools-and-frameworks.html` against the cached Jekyll output.
- Verify the generated HTML contains the expected `<canvas>` tags and `data-*` attributes, and that the `<figcaption>` ordering matches the Jekyll page.

### Regression: raw HTML preservation
- Add a targeted regression test for the raw HTML / markdown interaction that preserves canvas attributes in this page.
- Confirm the test fails before the fix and passes after the fix.

## Log

### [PM] 2026-03-24 19:05
- Groomed the issue into a precise single-page DOM parity target for the LLM tools blog post.
- Recorded DTC baseline: `766/790`.
- Added explicit output verification for the canvas `data-*` attributes and `<figcaption>` ordering, with a page-level zero-diff target for the previously reported discrepancies.
- Current DTC compare summary: `766 files matched, 24 files with differences, 731 total differences (3026 acceptable diffs filtered out)`.

### [SWE] 2026-03-24 19:12
- Wrote regression tests first in `src/kramdown.rs`: `test_postprocess_escapes_malformed_single_quote_canvas`, `test_postprocess_preserves_wellformed_single_quote_canvas`, and `test_has_unbalanced_single_quotes_detects_canvas`.
- Ran the malformed-canvas regression test before the fix: FAILS as expected because the malformed `<canvas>` remained a live tag instead of being escaped.
- Implemented the fix in `src/kramdown.rs` by escaping malformed raw HTML tags with unbalanced single quotes before newline normalization, so the LLM tools page now matches Jekyll's escaped canvas handling.
- Verified the helper test passes, then reran the regression test and the broader postprocess slice: `29` postprocess tests passed, `0` failed.
- Verification also passed for `cargo fmt --check`, `./scripts/cargo-safe clippy -- -D warnings`, and `./scripts/cargo-safe build --release`.
- Rebuilt the DTC site to `/tmp/dtc_issue339` and reran DOM comparison against the cached Jekyll output; summary was `767 files matched, 23 files with differences, 722 total differences (3026 acceptable diffs filtered out)`.
- The target page `blog/how-do-professionals-use-llm-tools-and-frameworks.html` no longer appears in the diff list, so the canvas/figcaption discrepancy reported for issue 339 is resolved while keeping the repo-wide baseline above `766/790`.
- Files modified: `src/kramdown.rs`, `docs/tracker/339-dtc-blog-canvas-and-llm-tools.in-progress.md`.

### [QA] 2026-03-24 19:22
- Tests: `./scripts/cargo-safe test --lib -- test_postprocess_` passed (`29` passed, `0` failed); full `./scripts/cargo-safe test` passed (`0` failures across lib, main, integration, and doc-test targets).
- Clippy: clean (`./scripts/cargo-safe clippy -- -D warnings` passed; only renamed/removed lint warnings from dependency configuration output before crate check).
- Fmt: clean (`cargo fmt --check` passed).
- DTC build/output: `./scripts/cargo-safe build --release` passed; `./target/release/rustkyll build --source websites/DataTalksClub/datatalksclub.github.io --destination /tmp/dtc_qa_issue339` passed.
- DTC DOM comparison: `768 files matched, 22 files with differences, 569 total differences (3026 acceptable diffs filtered out)`; baseline `766/790` maintained and improved.
- Page-level verification: `blog/how-do-professionals-use-llm-tools-and-frameworks.html` no longer appeared in the DOM diff output; direct HTML spot-check confirmed the expected chart `<canvas>` / `<figcaption>` structure is present in `/tmp/dtc_qa_issue339/blog/how-do-professionals-use-llm-tools-and-frameworks.html`.
- TDD compliance: PASS — SWE log shows test written first, failure confirmed before the fix, implementation, then passing targeted tests.
- Acceptance criteria: canvas/chart structure PASS; figcaption ordering PASS; page-level zero-diff target PASS; regression tests fail-then-pass PASS; repo-wide DTC baseline PASS.
- Note: the shared-tree DTC build generation time in this QA run was `1.413s`, above the process target, but this worktree also contains unrelated live rendering changes for other issues; the `339`-specific acceptance criteria and DOM parity target were still met.
- VERDICT: PASS

### [PM] 2026-03-25 08:43
- Reviewed diff: 2 files changed (`src/kramdown.rs`, this issue file).
- Output verification: rebuilt DTC to `/tmp/dtc_pm_issue339`, confirmed `blog/how-do-professionals-use-llm-tools-and-frameworks.html` is absent from the DOM diff output, and inspected the generated HTML for the expected `<canvas>` / `<figcaption>` structure.
- Results verified: DTC DOM comparison returned `768 files matched, 22 files with differences, 569 total differences (3026 acceptable diffs filtered out)`; baseline `766/790` improved.
- Acceptance criteria: all met.
- Follow-up issues created: none.
- VERDICT: ACCEPT
