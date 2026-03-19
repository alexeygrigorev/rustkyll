# Issue 250: Fix DTC/docs layout template `<hr />` whitespace causing DOM diffs

## Problem

9 of the remaining 11 DTC/docs DOM differences are caused by `<hr>` in the footer.html include being normalized to `<hr />` by rustkyll. Jekyll outputs bare `<hr>` (HTML5-style) from include files, but rustkyll converts it to `<hr />` (XHTML-style) at include-load time.

### Root Cause

In `src/template/engine.rs` line 1336, when loading include files from `_includes/`, rustkyll calls `crate::kramdown::normalize_html_output(&content)` on every include. This function calls `normalize_br_hr_only()`, which converts bare `<hr>` to `<hr />`.

The `normalize_html_output` function was designed for markdown-rendered content (where kramdown produces XHTML-style void elements), but it is incorrectly applied to raw HTML include files. Jekyll does NOT normalize void elements in includes -- they pass through as-is.

### Why This Causes DOM Nesting Differences

The affected 9 pages all have children navigation (from `inject_children_nav`), which produces unclosed `<li>` elements (valid HTML5 optional closing tags). After these unclosed `<li>` tags, the footer include renders `<hr />`. The combination of unclosed `<li>` tags plus `<hr />` (vs bare `<hr>`) causes BeautifulSoup (the DOM comparison tool) to construct a different DOM tree, nesting `<footer>` inside `<hr />` instead of making them siblings.

Pages without children navigation are unaffected because the parser state is clean when it encounters the footer's `<hr />`.

### Affected Pages (9)

1. activities/index.html
2. courses/course-management-platform/index.html
3. courses/data-engineering-zoomcamp/index.html
4. courses/data-engineering-zoomcamp/logistics/index.html
5. courses/data-engineering-zoomcamp/resources/index.html
6. courses/ml-zoomcamp/index.html
7. general/guidelines/index.html
8. general/index.html
9. general/jobs/index.html

All show the same DOM diff pattern:
```
hr > footer: extra_element (footer nested under hr in rustkyll)
footer: missing_element (footer missing as sibling in rustkyll)
```

### Remaining 2 Failures (NOT in scope)

- activities/open-source-spotlight/index.html -- smart quotes (issue 211)
- general/guidelines/promotion/index.html -- emphasis parsing (issue 247)

## Scope

Stop normalizing void elements (`<hr>`, `<br>`, etc.) in include files loaded from `_includes/`. Include files contain raw HTML that should pass through the template engine without modification. The `normalize_html_output` call should only apply to markdown-rendered content.

## Fix Location

`src/template/engine.rs`, function `load_includes_recursive` (around line 1336). Remove or skip the `normalize_html_output` call on include file content.

Note: the comment at lines 1332-1335 says "Pre-normalize void elements... so the final normalize_html_output() can exit early". This was a performance optimization, but it causes incorrect output. The fix should remove this pre-normalization. If the final `normalize_html_output` call on the full page is needed for markdown content, it should still work correctly -- the markdown-rendered `<hr>` elements inside `{{ content }}` will still be in XHTML-style from kramdown rendering.

## Dependencies

- Issue 246 (done) -- provides the baseline of 46/57

## Acceptance Criteria

- [ ] AC1: `<hr>` elements in include files (e.g., `_includes/components/footer.html`) are NOT converted to `<hr />` during include loading
- [ ] AC2: Include files are loaded as-is without void element normalization
- [ ] AC3: Markdown-rendered `<hr />` elements (from kramdown) remain in XHTML-style (no regression)
- [ ] AC4: DTC/docs DOM comparison shows at least 55/57 matches (up from 46/57)
- [ ] AC5: The 9 pages listed above no longer show the `<hr /> > <footer>` nesting artifact
- [ ] AC6: `cargo test` passes (no regressions)
- [ ] AC7: `cargo clippy -- -D warnings` passes
- [ ] AC8: `cargo fmt` shows no changes

## Test Scenarios

### Unit: Include loading preserves bare void elements

- Load an include file containing `<hr>` and verify the loaded content still contains `<hr>` (not `<hr />`)
- Load an include file containing `<br>` and verify the loaded content still contains `<br>` (not `<br />`)
- Load an include file with mixed HTML (`<hr>`, `<meta>`, `<div>`) and verify none are modified

### Unit: Markdown rendering still normalizes void elements

- Render markdown containing `---` (horizontal rule) and verify it produces `<hr />` (XHTML-style)
- Render markdown containing a line break and verify it produces `<br />` (XHTML-style)
- Verify `normalize_html_output` still works correctly on markdown-rendered content

### Integration: Full site footer rendering

- Build a page that uses a layout with a footer include containing `<hr>`, verify the output contains bare `<hr>` (not `<hr />`) from the footer
- Build a page with children navigation AND a footer include, verify the footer's `<hr>` is bare while the children nav's `<hr>` is also bare (both match Jekyll)

### Output Verification: DTC/docs DOM comparison

- Build the DTC/docs site with rustkyll
- Run the DOM comparison script
- Verify at least 55/57 pages match (the 2 remaining failures should be the smart quotes and emphasis issues only)
- Specifically verify that the 9 pages listed above no longer show hr/footer nesting differences

## Origin

Descoped from issue 246 AC12 (target was 50/57, achieved 46/57 due to this whitespace issue).

## Log

### [SWE] 2026-03-19
- TDD Step 1: Wrote 3 failing tests in src/template/engine.rs:
  - test_load_includes_preserves_bare_hr
  - test_load_includes_preserves_bare_br
  - test_load_includes_preserves_mixed_html_unchanged
- TDD Step 2: Ran tests, all 3 FAIL as expected:
  - "Include loading must preserve bare <hr>, got: <hr />"
  - "Include loading must preserve bare <br>, got: <br />"
  - "assertion left == right failed: Include files should pass through without any modification"
- TDD Step 3: Removed `crate::kramdown::normalize_html_output(&content)` call in
  `load_includes_recursive()` at src/template/engine.rs:1336. Include files now pass
  through as-is without void element normalization.
- TDD Step 4: Ran tests, all 3 PASS
- Full test suite: 2107 passed, 0 failed
- Clippy: clean (no warnings in rustkyll code)
- Fmt: clean
- Files modified: src/template/engine.rs (removed normalize call, added 3 tests)

### [SWE] 2026-03-19 (QA feedback fix)
- QA identified that the include-level fix was only half the solution. The
  `normalize_br_hr_only` function in src/kramdown.rs does a blanket
  `html.replace("<hr>", "<hr />")` on the FULL rendered page output (called
  from layout.rs at lines 342, 387, 468, 536, 679 via `normalize_html_output`).
  This converts ALL `<hr>` including those from includes and layouts to `<hr />`.
- TDD Step 1: Wrote 3 tests in src/kramdown.rs:
  - test_normalize_html_output_does_not_convert_bare_hr
  - test_normalize_html_output_still_converts_bare_br
  - test_postprocess_still_converts_hr_in_markdown_content
- TDD Step 2: Ran test_normalize_html_output_does_not_convert_bare_hr: FAILS as expected
  - "normalize_html_output must NOT convert bare <hr> to <hr />. Got: <hr />"
- TDD Step 3: Renamed `normalize_br_hr_only` to `normalize_br_only` and removed
  the `<hr>` replacement. Only `<br>` -> `<br />` conversion remains.
  pulldown-cmark already outputs `<hr />` for markdown `---`, and `postprocess()`
  already calls `normalize_bare_void_elements()` on markdown content.
- TDD Step 4: All 3 new tests PASS
- Full test suite: 2108 passed, 0 failed
- Clippy: clean
- Fmt: clean
- DOM comparison: 55/57 (up from 46/57, +9 improvement)
  - All 9 hr/footer nesting pages now match
  - Remaining 2 failures: smart quotes (issue 211), emphasis parsing (issue 247)
- Files modified: src/kramdown.rs (renamed normalize_br_hr_only -> normalize_br_only,
  removed <hr> replacement, added 3 tests, updated comments)
