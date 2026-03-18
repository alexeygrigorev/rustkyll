# Issue 183: Remove extra language-plaintext class from code block wrapper div

## Checklist Category

This is a sub-issue of **Other attribute differences** (85 pages) and contributes to the **Inline code gets extra CSS class** (1 page) category. The wrapper div class issue is distinct from the inline code class issue (which was addressed in issues 157/176).

## Problem

rustkyll adds `class='highlighter-rouge language-plaintext'` to the `<div>` wrapper of fenced code blocks without a language tag. Jekyll only uses `class='highlighter-rouge'` on the div (the `language-plaintext` class goes on the inner `<code>` element only).

Sample diff:
```
body > div > div > div > div > div: attribute_differs
  expected: "class='highlighter-rouge'"
  actual:   "class='highlighter-rouge language-plaintext'"
```

Jekyll's wrapper structure for a no-language fenced code block:
```html
<div class="highlighter-rouge"><div class="highlight"><pre class="highlight"><code>...</code></pre></div></div>
```

rustkyll currently produces:
```html
<div class="highlighter-rouge language-plaintext"><div class="highlight"><pre class="highlight"><code>...</code></pre></div></div>
```

## Goal

Match Jekyll's class attribute on the code block wrapper div -- `highlighter-rouge` only, no `language-plaintext`.

## Affected Sites

- alexeygrigorev/mlbookcamp-page: ~5 pages
- muan-blog: partial improvement (contributes to attribute diffs)

## Dependencies

None.

## Approach (TDD)

1. Write a test that renders a fenced code block without language and asserts the outermost wrapper div has only `class="highlighter-rouge"` (no `language-plaintext`)
2. Verify the test fails
3. Fix in `src/kramdown.rs` (fenced code block wrapping logic around line 1073)
4. Verify the test passes

## Acceptance Criteria

- [ ] Wrapper `<div>` for no-language fenced code blocks has `class="highlighter-rouge"` only (no `language-plaintext`)
- [ ] Inner `<code>` element still has `language-plaintext` class where appropriate
- [ ] Wrapper `<div>` for language-specified code blocks (e.g., `python`) still has `class="language-python highlighter-rouge"` (both classes) -- this is correct Jekyll behavior
- [ ] Existing code block rendering tests still pass
- [ ] `cargo test` passes

## Test Scenarios

### Unit: Wrapper div class (write FIRST, must fail before fix)

- **Test `test_no_language_wrapper_div_class`**: Render a fenced code block with no language tag. Assert the outermost div has `class="highlighter-rouge"` and does NOT contain `language-plaintext`.
- **Test `test_language_wrapper_div_still_has_both_classes`**: Render a fenced code block with `python` language. Assert the outermost div has `class="language-python highlighter-rouge"`.
- **Test `test_no_language_inner_code_has_plaintext_class`**: Render a no-language fenced code block. Assert the inner `<code>` element still has `class="language-plaintext"` (or whatever Jekyll puts there).

### Regression: Other code block behavior preserved

- **Test `test_highlighted_code_block_structure_unchanged`**: Render a code block with a recognized language. Verify the full wrapper structure matches Jekyll output.

### Integration: Output verification

- Build mlbookcamp-page and inspect pages with no-language code blocks to verify the wrapper div class is correct.

## Log

### [SWE] 2026-03-18
- TDD approach: wrote 4 new tests first, verified `test_no_language_wrapper_div_class` fails (red phase)
- Root cause: `wrap_fenced_code_blocks()` in kramdown.rs line 1201 unconditionally used `language-{lang} highlighter-rouge` for all code blocks, including plaintext
- Fix: added conditional -- when lang is "plaintext", wrapper div gets only `class="highlighter-rouge"`; language-specified blocks keep both classes
- Updated 8 existing tests that asserted `language-plaintext highlighter-rouge` on wrapper divs
- Updated doc comment to reflect new behavior
- Tests added: 4 new tests (test_no_language_wrapper_div_class, test_language_wrapper_div_still_has_both_classes, test_no_language_inner_code_has_no_extra_class, test_highlighted_code_block_structure_unchanged)
- Build: 1684 tests pass, 0 fail, fmt clean
- Clippy: pre-existing errors in other files (context.rs, vendor/), no errors in kramdown.rs
- Files modified: src/kramdown.rs
