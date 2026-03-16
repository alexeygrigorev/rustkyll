# Issue 146: Fix ordered list start attribute

## Problem

Rustkyll adds `start='N'` attributes to `<ol>` elements where Jekyll does not. 33 instances across ~5 files.

This happens when markdown has ordered list items that don't start at 1, or when the list is split across HTML blocks. Kramdown may not emit `start` attributes in these cases.

Discovered in issue #119 DOM diff audit.

## Acceptance criteria

- `<ol>` elements match Jekyll's `start` attribute behavior
- No regressions

## Log

### [SWE] 2026-03-16

- Root cause: `postprocess_for_filter()` (used by the `markdownify` Liquid filter) did not call `remove_ol_start_attribute()`. The full `postprocess()` (used for page body content) already had this fix from issue #90, but the lighter filter path was missed.
- All 33 instances are in `books/` pages where book archive Q&A threads use `{{ thread.text | newline_to_br | markdownify }}` -- this goes through `markdown_to_html_for_filter` -> `postprocess_for_filter`, bypassing the full postprocess pipeline.
- Fix: Added `remove_ol_start_attribute` call to `postprocess_for_filter()` in `src/kramdown.rs`
- Tests added: 2 new tests
  - `test_d11_postprocess_for_filter_removes_ol_start` in kramdown.rs (unit test for the postprocess_for_filter path)
  - `test_markdownify_no_ol_start_attribute` in markdownify.rs (integration test through the filter)
- TDD: both tests confirmed failing before fix, passing after
- Build: 1470 tests pass, 0 fail, clippy clean, fmt clean
- Files modified: `src/kramdown.rs`, `src/template/filters/markdownify.rs`
