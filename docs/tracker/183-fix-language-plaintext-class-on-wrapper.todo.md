# Issue 183: Remove extra language-plaintext class from code block wrapper div

## Problem

rustkyll adds `class='highlighter-rouge language-plaintext'` to the `<div>` wrapper of fenced code blocks without a language tag. Jekyll only uses `class='highlighter-rouge'` on the div (the `language-plaintext` class goes on the inner `<code>` element only).

Sample diff:
```
body > div > div > div > div > div: attribute_differs
  expected: "class='highlighter-rouge'"
  actual:   "class='highlighter-rouge language-plaintext'"
```

## Goal

Match Jekyll's class attribute on the code block wrapper div.

## Affected Sites

- alexeygrigorev/mlbookcamp-page: ~5 pages
- muan-blog: partial improvement

## Approach (TDD)

1. Write a test that renders a fenced code block without language and asserts the wrapper div has only `highlighter-rouge` class
2. Verify the test fails
3. Fix in `src/kramdown.rs` (fenced code block wrapping)
4. Verify the test passes

## Acceptance Criteria

- [ ] Wrapper `<div>` has `class='highlighter-rouge'` only (no `language-plaintext`)
- [ ] Inner `<code>` still has `language-plaintext` class
- [ ] Existing code block tests pass
