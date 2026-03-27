# Issue 388: DTC natural-language-processing pipe-in-URI table in markdownify

## Problem

`books/20220425-natural-language-processing-with-transformers.html` has 3 DOM
diffs. Jekyll/kramdown treats `<tel:100-1000|100-1000>` as a table cell
delimiter, producing `<table><tbody><td>` structure. Rustkyll escapes the pipe
to `&#124;` which prevents table formation.

The archive text in book pages flows through `newline_to_br | markdownify`,
which calls `markdown_to_html_for_filter()`. The pipe in `<tel:100-1000|100-1000>`
must survive into the kramdown pipe-table conversion in that path so kramdown
produces the `<table>` structure matching Jekyll output.

## Root Cause

`escape_non_standard_autolink_schemes()` in frontmatter.rs (added in issue #364)
unconditionally escapes `|` to `&#124;` inside non-standard autolinks. This is
correct for the main markdown pipeline (`markdown_to_html`) because it prevents
unwanted tables in blog posts, but wrong for the markdownify pipeline
(`markdown_to_html_for_filter`) where kramdown DOES create tables from pipes in
angle brackets.

## Lesson from #366

Issue #366 attempted to fix this by removing pipe escaping globally. That
regressed blog posts (the main pipeline needs pipe escaping to prevent spurious
tables). This fix MUST be scoped to the markdownify path only. The main pipeline
(`markdown_to_html`) MUST continue escaping pipes exactly as it does today.

## Approach

Add a boolean parameter `escape_pipes` to `escape_non_standard_autolink_schemes`:
- `markdown_to_html()` calls: `escape_non_standard_autolink_schemes(markdown, true)` (current behavior, no change)
- `markdown_to_html_for_filter()` calls: `escape_non_standard_autolink_schemes(markdown, false)` (let pipes through)

The change is exactly two call sites and one function signature. No other code
should change.

## Scope

1. Only change pipe escaping behavior in the markdownify path
2. Must not regress DTC DOM (785/790)
3. Must not affect blog post rendering via the main pipeline

## Dependencies

None -- standalone rendering fix.

## Acceptance Criteria

- [ ] `escape_non_standard_autolink_schemes()` accepts an `escape_pipes: bool` parameter
- [ ] `markdown_to_html()` passes `escape_pipes: true` (preserving current behavior exactly)
- [ ] `markdown_to_html_for_filter()` passes `escape_pipes: false`
- [ ] In the markdownify path, `<tel:100-1000|100-1000>` produces output containing a literal `|` character (not `&#124;`), allowing kramdown pipe-table conversion to form a `<table>`
- [ ] In the main pipeline, `<tel:100-1000|100-1000>` still produces `&#124;` (pipe escaped, no table formed) -- regression guard
- [ ] All existing issue #364 tests continue to pass unchanged (they test the markdownify path via `markdown_to_html_for_filter`, so the pipe test now expects literal `|` not `&#124;`)
- [ ] DTC DOM match count improves from 785/790 (expected: 788/790, fixing 3 diffs on the NLP transformers page)
- [ ] No blog post rendering regressions (blog posts use `markdown_to_html`, which still escapes pipes)
- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` shows no changes

## Test Scenarios

### Unit: markdownify path preserves pipes (escape_pipes=false)

- Call `markdown_to_html_for_filter("<tel:100-1000|100-1000>")`, verify output contains literal `|` (not `&#124;`)
- Verify the output still escapes angle brackets: contains `&lt;tel:` and `&gt;`
- Verify autolink suppression still works: output does NOT contain `<a href=`

### Unit: main pipeline still escapes pipes (escape_pipes=true) -- regression guard

- Call `markdown_to_html("some text with <tel:100-1000|100-1000> in it")`, verify output contains `&#124;` (pipe escaped)
- This is the critical regression guard against the #366 mistake

### Unit: both paths still suppress non-standard autolinks

- Both `markdown_to_html` and `markdown_to_html_for_filter` with `<ssh:user@host>` should produce `&lt;ssh:user@host&gt;` (no `<a href=`)
- Both paths with `<http://example.com>` should still produce `<a href="http://example.com">` (standard autolinks preserved)

### Unit: escape_non_standard_autolink_schemes function directly

- Call with `escape_pipes=true` on `<tel:a|b>`: output contains `&#124;`
- Call with `escape_pipes=false` on `<tel:a|b>`: output contains literal `|`, NOT `&#124;`
- Call with `escape_pipes=false` on `<tel:a|b>`: output still contains `&lt;` and `&gt;` (angle bracket escaping is independent of pipe escaping)

### Integration: DTC site output verification

- Build the DTC site and check `books/20220425-natural-language-processing-with-transformers.html`
- Verify the page contains a `<table>` element where the tel: pipe text appears (matching Jekyll DOM)
- Verify DTC DOM match count is at least 788/790 (785 baseline + 3 fixed diffs)
- Spot-check blog posts to confirm no new tables appeared from pipe characters

## Baseline

- DTC DOM: 785/790

## Priority

LOW

## Log

### [SWE] 2026-03-27

**Investigation: Issue premise is INCORRECT -- proposed change REGRESSES the NLP page**

1. Verified baseline: the NLP transformers page (`books/20220425-natural-language-processing-with-transformers.html`) already renders correctly. The `<tel:100-1000|100-1000>` text appears as literal escaped text `&lt;tel:100-1000|100-1000&gt;s of GPUs` in both Jekyll and rustkyll output -- they match exactly.

2. The issue claims Jekyll produces a `<table>` from the pipe in `<tel:100-1000|100-1000>`. This is FALSE. Checking the Jekyll _site output directly:
   ```
   &lt;tel:100-1000|100-1000&gt;s of GPUs
   ```
   No `<table>` in Jekyll output for this content.

3. The issue claims 3 DOM diffs on this page. Current DOM comparison shows ONLY 1 diff: the global template-level `href=''` vs `href='https://...'` attribute, which is the same across all 784 pages. No content diff.

4. Implemented the proposed change (escape_pipes=false in markdownify path). Result: the NLP page went from 1 diff to 10 diffs. The pipe now forms a SPURIOUS table via `convert_kramdown_pipe_tables`, breaking the list item content into `<table><tbody><tr><td>` elements that do not exist in the Jekyll output.

5. Key technical finding: `escape_non_standard_autolink_schemes` does escape `|` to `&#124;` in the pre-pulldown markdown, but pulldown-cmark's HTML renderer decodes `&#124;` back to literal `|` in the final HTML output. So the current behavior is correct: the pipe is escaped BEFORE `convert_kramdown_pipe_tables` runs (preventing spurious table formation), then pulldown-cmark decodes it back to `|` for the final HTML, producing output identical to Jekyll.

6. Reverted all changes. No code modifications remain.

**CONCLUSION: This issue should be CLOSED as invalid. The proposed change is harmful. The NLP transformers page already matches Jekyll output. No fix is needed.**

- DTC DOM baseline: 3/787 matched (784 diffs, all from the global template href issue)
- NLP page: only the global template href diff, no content diffs
- Files modified: none (all changes reverted)
