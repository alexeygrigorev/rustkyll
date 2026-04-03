# Issue 547: `<details markdown="1">` content not rendered as markdown

## Problem

When a `<details>` block has the `markdown="1"` attribute, rustkyll strips the attribute but does NOT process the inner content as markdown. Jekyll/kramdown renders the inner content, wrapping text in `<p>` and converting markdown syntax like `**bold**` to `<strong>bold</strong>`.

**Source markdown:**
```markdown
<details markdown="1">
<summary>Click here!</summary>
Here you can see an **expandable** section
</details>
```

**Jekyll output (expected):**
```html
<details>
  <summary>Click here!</summary>
  <p>Here you can see an <strong>expandable</strong> section</p>
</details>
```

**Rustkyll output (actual):**
```html
<details>
<summary>Click here!</summary>
Here you can see an **expandable** section
</details>
```

## Root Cause

The `process_markdown_attribute()` function in `src/kramdown.rs` correctly handles `markdown="1"` on elements like `<div>`, `<aside>`, etc. However, `<details>` is special because it contains a `<summary>` child element. The current logic likely fails because:

1. The content between `<details>` and `</details>` includes `<summary>...</summary>`, and the markdown processing either chokes on this nested HTML or the `<summary>` block interferes with the closing tag search.
2. The `<details>` tag may be handled by the separate fenced-code-in-details logic (line ~2547 in kramdown.rs) which strips the content before `process_markdown_attribute` gets to it.

The fix needs to ensure that when `<details markdown="1">` is encountered, the content AFTER `<summary>...</summary>` is rendered as markdown while preserving the `<summary>` element as-is.

## Affected Sites

- beautiful-jekyll: 1 page (`2020-02-28-sample-markdown/index.html`) -- would push from 4/5 to 5/5 DOM match

## Key Files

- `src/kramdown.rs` -- `process_markdown_attribute()` function (~line 1795)
- `src/kramdown.rs` -- `render_fenced_code_in_details()` function (~line 2547) may interfere

## Dependencies

None.

## DTC DOM Baseline

596/790 (255 total diffs). Must not regress.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] `<details markdown="1"><summary>Click</summary>text with **bold**</details>` renders inner content as markdown (bold becomes `<strong>`)
- [ ] `<summary>` content is preserved as-is (not processed as markdown)
- [ ] `<details>` WITHOUT `markdown="1"` is left unchanged (no markdown processing)
- [ ] beautiful-jekyll DOM match improves from 4/5 to 5/5
- [ ] DTC DOM match count does not drop below 596/790
- [ ] No regressions in muan-blog (36/39 or better)

## Test Scenarios

### Unit: details markdown attribute processing
- Parse `<details markdown="1"><summary>Title</summary>\nText with **bold**</details>` -- verify output contains `<strong>bold</strong>` wrapped in `<p>`
- Parse `<details markdown="1"><summary>Title</summary>\n- item 1\n- item 2</details>` -- verify output contains `<ul><li>` elements
- Parse `<details><summary>Title</summary>\nText with **bold**</details>` (no markdown attr) -- verify `**bold**` is NOT converted
- Parse nested `<details markdown="1">` inside another element -- verify correct handling

### Integration: beautiful-jekyll site build
- Build beautiful-jekyll site with rustkyll
- Run DOM comparison: verify `2020-02-28-sample-markdown/index.html` now matches
- Verify overall DOM match is 5/5

### Regression: DTC site build
- Build DTC site, verify DOM count >= 596/790

## Log

### [SWE] 2026-04-02

**Root cause:** `process_markdown_attribute()` in `src/kramdown.rs` correctly finds `<details markdown="1">` and extracts the inner content, but when the inner content starts with `<summary>...</summary>`, pulldown-cmark treats the entire content as an HTML block (CommonMark type 6) and passes it through as-is, so no markdown rendering occurs.

**Fix: Extract `<summary>` before markdown rendering**

- Wrote test `test_547_details_markdown_attr_bold` (src/kramdown.rs)
- Ran test: FAILS -- got `<details>\n<summary>Click here!</summary>\nHere you can see an **expandable** section\n</details>`, expected `<strong>expandable</strong>`
- Wrote test `test_547_details_markdown_attr_list` (src/kramdown.rs)
- Ran test: FAILS -- got raw `- item 1\n- item 2`, expected `<ul><li>`
- Wrote test `test_547_details_markdown_attr_unicode` (src/kramdown.rs)
- Ran test: FAILS -- got raw `**粗体**`, expected `<strong>粗体</strong>`
- Wrote test `test_547_details_no_markdown_attr_unchanged` (src/kramdown.rs)
- Ran test: PASSES (correct -- no markdown attr means no processing)
- Implemented fix: added `extract_summary_from_details()` helper function that separates `<summary>...</summary>` from the body content. In `process_markdown_attribute()`, for `<details>` tags, the summary is extracted first, only the body is sent through pulldown-cmark rendering, then the summary is re-inserted before the rendered body.
- Ran all 5 tests: PASSES
- Wrote test `test_547_details_markdown_attr_exact_jekyll_output` to verify exact output format matches Jekyll

**Verification:**
- Full test suite: 3782+ tests pass, 0 failures
- Clippy: clean (no warnings)
- Fmt: clean
- DTC DOM: 596/790 (255 total diffs) -- matches baseline exactly, no regression
- DTC build time: 0.587s -- under 1.0s threshold
- beautiful-jekyll DOM: 4/5 (3 remaining diffs are all pre-existing `<img>` wrapping issues, NOT related to `<details>`)
- muan-blog DOM: 36/39 -- no regression

**Note on beautiful-jekyll 5/5 target:** The issue predicted this fix would push beautiful-jekyll from 4/5 to 5/5. The `<details>` block now renders correctly and causes no DOM diffs. However, the sample-markdown page still has 3 pre-existing `<img>` wrapping diffs (p vs img tag) that were there before this change and are unrelated to `<details>`. The `<details>` fix is complete and correct.

**Summary:**
- Files modified: `src/kramdown.rs`
- Tests added: 5 unit tests (bold, list, no-attr unchanged, unicode, exact Jekyll output)
- Build results: 3782+ tests pass, 0 fail, clippy clean, fmt clean
- Known limitation: beautiful-jekyll stays at 4/5 due to unrelated `<img>` wrapping diffs

### [QA] 2026-04-02
- Tests: 3782 passed, 0 failed, 2 ignored (pre-existing, not from this issue)
- Clippy: clean (0 warnings)
- Fmt: clean
- DTC DOM: 596/790, 255 total diffs -- matches baseline exactly, no regression
- DTC build time: 0.649s -- under 1.0s threshold
- beautiful-jekyll DOM: 4/5, 3 diffs (all pre-existing img-vs-p issues, not details-related)
- muan-blog DOM: 36/39 -- no regression
- Acceptance criteria:
  - `cargo build` compiles without errors: PASS
  - `cargo test` passes with all existing tests plus new tests: PASS (5 new tests)
  - `<details markdown="1">` renders inner content as markdown: PASS (bold, list tested)
  - `<summary>` content preserved as-is: PASS
  - `<details>` WITHOUT `markdown="1"` left unchanged: PASS (dedicated test)
  - beautiful-jekyll DOM improves from 4/5 to 5/5: FAIL -- stays at 4/5 (see note below)
  - DTC DOM count does not drop below 596/790: PASS (596/790 exact)
  - No regressions in muan-blog (36/39 or better): PASS (36/39)
- TDD compliance: PASS -- 3 core tests (bold, list, unicode) written and confirmed failing before implementation; no-attr test confirmed passing (correct existing behavior); exact-output test written after implementation
- Note on beautiful-jekyll 5/5: The `<details>` rendering is now correct and produces no DOM diffs. The 3 remaining diffs on `sample-markdown/index.html` are all `tag_name_differs: expected 'p', actual 'img'` -- a pre-existing `<img>` wrapping issue unrelated to this fix. The acceptance criterion of 5/5 was based on an incorrect assumption in the issue spec that the details fix alone would resolve the page.
- Minor code quality note: The doc comment on `extract_summary_from_details()` (line 2095) starts with "Extract the tag name from an opening tag..." which is the original doc comment for `extract_markdown_tag_name()` below it. The SWE inserted the new function between the existing doc comment and its function, stealing the doc comment. Not a blocker.
- VERDICT: PASS (with note on beautiful-jekyll 5/5 criterion -- the details fix is correct and complete; the remaining gap is a separate pre-existing issue)

### [PM] 2026-04-02 14:30
- Reviewed diff: 1 source file changed (src/kramdown.rs), +131 lines
- Code review: Clean implementation. `extract_summary_from_details()` correctly separates `<summary>` from body before markdown rendering. Fixed misplaced doc comment (QA-flagged) -- restored `extract_markdown_tag_name`'s stolen doc comment.
- Output verification: Built release binary, ran DTC DOM recount via `scripts/recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io` -- confirmed 596/790 (no regression).
- All 5 unit tests pass: bold rendering, list rendering, no-attr unchanged, unicode, exact Jekyll output match.
- Results verified: Real DTC DOM data present, real beautiful-jekyll comparison done.
- Acceptance criteria:
  - `cargo build` compiles: MET
  - `cargo test` passes with new tests: MET (5 new, 3784 total)
  - `<details markdown="1">` renders inner content as markdown: MET
  - `<summary>` content preserved as-is: MET
  - `<details>` without `markdown="1"` left unchanged: MET
  - beautiful-jekyll DOM improves from 4/5 to 5/5: NOT MET -- stays 4/5; remaining 3 diffs are pre-existing `<img>` wrapping issues (tag_name_differs: expected 'p', actual 'img') unrelated to `<details>`. Descoped to follow-up issue 548.
  - DTC DOM count >= 596/790: MET (596/790 exact)
  - No regressions in muan-blog (36/39 or better): MET (36/39)
- Follow-up issues created: docs/tracker/548-beautiful-jekyll-img-wrapping-diffs.todo.md (for the 3 remaining img wrapping diffs)
- VERDICT: ACCEPT
