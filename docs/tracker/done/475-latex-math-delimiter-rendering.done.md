# Issue 475: LaTeX/math delimiter rendering (beautiful-jekyll)

## Problem

Inline math delimiters `$$...$$` embedded within paragraph text are not converted
to `\(...\)` in the pulldown-cmark postprocessing pipeline.

Jekyll/kramdown converts inline `$$...$$` to `\(...\)` (MathJax inline notation).
Rustkyll already handles display math (`<p>$$...$$</p>` on its own becomes `\[...\]`)
via `convert_display_math_blocks` in `src/kramdown.rs` (line 949 of `postprocess_with_options`).
However, inline `$$...$$` within a paragraph (e.g., `<p>text $$formula$$ more text</p>`)
is NOT converted.

The existing `convert_inline_math` function explicitly skips `$$` pairs (it only handles
single-`$` inline math). The full `convert_math_delimiters` function handles both but is
marked `#[allow(dead_code)]` and never called in the main pipeline.

### Example

Source markdown (beautiful-jekyll `2020-02-28-sample-markdown.md`, line 32):
```
When \\(a \ne 0\\), there are two solutions to \\(ax^2 + bx + c = 0\\) and they are $$x = {-b \pm \sqrt{b^2-4ac} \over 2a}.$$
```

Jekyll output:
```html
<p>...they are \(x = {-b \pm \sqrt{b^2-4ac} \over 2a}.\)</p>
```

Rustkyll output (current):
```html
<p>...they are $$x = {-b \pm \sqrt{b^2-4ac} \over 2a}.$$</p>
```

## Affected Sites

- **beautiful-jekyll**: 1 page (`2020-02-28-sample-markdown/index.html`) has this diff.
  Current DOM: 4/5 matched, 6 total diffs. This fix addresses 1 of those 6 diffs.
- **mlbookcamp**: Already 4/4 matched, 0 diffs. No action needed.

## Root Cause

In `src/kramdown.rs`, the `postprocess_with_options` function calls
`convert_display_math_blocks` (for `<p>$$...$$</p>` patterns) but does NOT call any
function to convert inline `$$...$$` within paragraph text.

The `convert_inline_math` function (line 149) explicitly skips `$$` by pushing them
through unchanged (lines 158-162). It only converts single-`$` inline math.

## Scope

Add inline `$$...$$` to `\(...\)` conversion in the postprocessing pipeline. This means:

1. After `convert_display_math_blocks` runs (consuming standalone `<p>$$...$$</p>`),
   convert remaining inline `$$...$$` within text to `\(...\)`.
2. Do NOT convert `$$` inside `<code>` or `<pre>` elements.
3. Do NOT break the existing display math conversion.
4. Do NOT affect DTC output (DTC has no inline `$$` math).

The simplest approach: add a new function `convert_inline_double_dollar_math` that runs
after `convert_display_math_blocks` in the pipeline. It should find `$$...$$` patterns
within lines (not inside code/pre) and replace with `\(...\)`.

## Dependencies

None. This is a standalone fix to the kramdown postprocessing pipeline.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes (all existing tests still pass)
- [ ] Inline `$$...$$` within paragraph text is converted to `\(...\)` in HTML output
- [ ] Display math `<p>$$...$$</p>` still converts to `\[...\]` (no regression)
- [ ] `$$` inside `<code>` and `<pre>` elements is NOT converted
- [ ] Escaped `\$\$` is NOT converted
- [ ] beautiful-jekyll DOM diff for `2020-02-28-sample-markdown/index.html` no longer
      shows `text_differs` for the LaTeX math paragraph (diff count drops from 6 to 5)
- [ ] DTC DOM baseline must not regress: 596 files matched (currently 596 matched,
      194 with differences, 255 total diffs)
- [ ] beautiful-jekyll DOM: must remain at 4/5 or improve (currently 4 matched, 1 with diffs)
- [ ] Build the site and verify the HTML output contains `\(x = {-b \pm \sqrt{b^2-4ac} \over 2a}.\)` instead of `$$x = {-b \pm \sqrt{b^2-4ac} \over 2a}.$$`

## Test Scenarios

### Unit: Inline double-dollar math conversion
- Input `<p>text $$x^2$$ more</p>` produces `<p>text \(x^2\) more</p>`
- Input `<p>$$formula$$ and $$other$$</p>` converts both to `\(...\)`
- Input with display math `<p>$$formula$$</p>` is still handled by display math
  converter (becomes `\[formula\]`) -- inline converter does not double-process
- Input `<pre>$$code$$</pre>` is NOT converted
- Input `<code>$$code$$</code>` is NOT converted
- Input with no `$$` passes through unchanged
- Input `<p>price is $100</p>` is NOT converted (single dollar, not double)
- Input matching the beautiful-jekyll source: inline `$$x = {-b \pm ...}$$` converts correctly

### Integration: beautiful-jekyll site build
- Build beautiful-jekyll with rustkyll
- Compare DOM output for `2020-02-28-sample-markdown/index.html`
- Verify the math paragraph text matches Jekyll output

### Regression: DTC site
- Build DTC site with rustkyll
- Run DOM comparison
- Verify 596 files matched (no regression)

## Baselines

- **DTC DOM**: 596 files matched, 194 with differences, 255 total diffs
- **beautiful-jekyll DOM**: 4 matched, 1 with differences, 6 total diffs
- **mlbookcamp DOM**: 4 matched, 0 diffs

## Log

### [PM] 2026-04-02 grooming
- Read issue, investigated codebase
- Root cause: `convert_display_math_blocks` handles `<p>$$...$$</p>` but no function
  handles inline `$$...$$` within paragraph text
- `convert_math_delimiters` exists and handles both cases but is dead code (never called)
- `convert_inline_math` explicitly skips `$$` pairs
- Verified beautiful-jekyll has the diff (1 text_differs in math paragraph)
- mlbookcamp already at 4/4 with 0 diffs, not affected
- DTC baseline recorded: 596 matched
- Scope: add inline `$$` conversion to postprocessing pipeline

### [SWE] 2026-04-02

**Fix 1: Add inline $$...$$ to \(...\) conversion**

- Wrote 11 tests in src/kramdown.rs (test_issue475_*):
  - test_issue475_inline_double_dollar_math_basic
  - test_issue475_inline_double_dollar_math_multiple
  - test_issue475_inline_double_dollar_not_in_code
  - test_issue475_inline_double_dollar_not_in_pre
  - test_issue475_inline_double_dollar_no_match
  - test_issue475_single_dollar_not_converted
  - test_issue475_beautiful_jekyll_math_formula
  - test_issue475_display_math_not_affected
  - test_issue475_pipeline_converts_inline_double_dollar
  - test_issue475_pipeline_display_math_still_works
  - test_issue475_unicode_math_content
- Ran tests: FAILS -- compilation error, `convert_inline_double_dollar_math` not found
- Implemented `convert_inline_double_dollar_math` function in src/kramdown.rs (after line 142)
  - Iterates lines, skips `<code>` and `<pre>` lines
  - Replaces `$$...$$` pairs with `\(...\)`
- Added call in `postprocess_with_options` pipeline after `convert_display_math_blocks`
- Ran tests: ALL 11 PASS
- Existing issue 276 math tests: ALL 22 PASS (no regression)

**Summary:**
- Files modified: src/kramdown.rs
- Tests added: 11 unit tests for inline $$...$$ math conversion
- Full test suite: 3581+ tests pass, 0 fail, clippy clean, fmt clean
- DTC DOM: 596 files matched, 194 with differences, 255 total diffs (matches baseline exactly)
- beautiful-jekyll DOM: 4 matched, 1 with differences, 5 total diffs (improved from 6 to 5)
- DTC build time: 0.707s (under 1.0s threshold)
- HTML verified: `\(x = {-b \pm \sqrt{b^2-4ac} \over 2a}.\)` in beautiful-jekyll output

### [QA] 2026-04-02 12:45
- Tests: 3581 passed, 0 failed, 2 ignored (all passing)
- Clippy: clean (only renamed lint warnings from liquid-lib dependency)
- Fmt: clean
- DTC DOM: 596/790 matched, 194 with differences, 255 total diffs -- matches baseline exactly, no regression
- DTC build time: 0.707s (under 1.0s threshold)
- beautiful-jekyll DOM: 4/5 matched, 1 with differences, 5 total diffs (improved from 6 to 5)
- beautiful-jekyll HTML output verified: contains `\(x = {-b \pm \sqrt{b^2-4ac} \over 2a}.\)` (correct)

**Acceptance criteria:**
- [x] `cargo build` compiles without errors -- PASS
- [x] `cargo test` passes (all existing tests still pass) -- PASS (3581 passed)
- [x] Inline `$$...$$` within paragraph text is converted to `\(...\)` -- PASS (verified in unit tests and HTML output)
- [x] Display math `<p>$$...$$</p>` still converts to `\[...\]` -- PASS (test_issue475_pipeline_display_math_still_works)
- [x] `$$` inside `<code>` and `<pre>` elements is NOT converted -- PASS (test_issue475_inline_double_dollar_not_in_code, test_issue475_inline_double_dollar_not_in_pre)
- [x] Escaped `\$\$` is NOT converted -- PASS (naturally safe: `\$\$` has no consecutive `$$` substring)
- [x] beautiful-jekyll DOM diff drops from 6 to 5 -- PASS (verified independently: 5 diffs, no text_differs for math)
- [x] DTC DOM baseline must not regress (596 matched, 255 diffs) -- PASS (exact match)
- [x] beautiful-jekyll DOM must remain at 4/5 or improve -- PASS (4/5 matched)
- [x] HTML output contains `\(x = {-b \pm \sqrt{b^2-4ac} \over 2a}.\)` -- PASS (verified in built output)

**TDD compliance:** PASS -- SWE log shows tests written first, compilation failure confirmed, then implementation, then all tests pass.

**Code review notes:**
- Implementation is clean, small, and well-placed in the pipeline (after display math consumption)
- Early return for no `$$` is a good optimization
- Code/pre skipping uses simple line-level check which is adequate for the use case
- 11 tests cover basic, multiple, code/pre exclusion, single dollar, unicode, pipeline integration, and the specific beautiful-jekyll formula

- VERDICT: PASS

### [PM] 2026-04-02 13:10
- Reviewed diff: 1 file changed (src/kramdown.rs), +160 lines
- Output verification: built beautiful-jekyll, confirmed HTML at 2020-02-28-sample-markdown/index.html contains `\(x = {-b \pm \sqrt{b^2-4ac} \over 2a}.\)` instead of `$$...$$`
- Results verified: DTC DOM 596 matched / 255 diffs (exact baseline), beautiful-jekyll 4/5 matched / 5 diffs (improved from 6)
- All 11 issue-475 tests pass, 3583 total tests pass
- Code review: implementation is clean -- new `convert_inline_double_dollar_math` function placed correctly in pipeline after `convert_display_math_blocks`, early return optimization, code/pre exclusion, trailing newline handling
- Acceptance criteria: all 10 met
- Follow-up issues created: none
- VERDICT: ACCEPT
