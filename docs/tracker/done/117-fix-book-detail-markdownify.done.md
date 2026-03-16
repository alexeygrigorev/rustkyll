# Issue 117: Fix book detail page markdownify pipeline

book-ml-bookcamp.html has 5% pixel diff. The newline_to_br | markdownify filter pipeline produces different output for Q&A threads with complex text.

## Acceptance criteria
- book-ml-bookcamp.html achieves 0% pixel diff
- No regressions on other book pages

## Log

### [SWE] 2026-03-16

**Root cause analysis:**

Two issues in the `newline_to_br | markdownify` pipeline:

1. `normalize_void_elements` in `kramdown::postprocess` was converting `<br />` to `<br>`.
   Jekyll/kramdown outputs XHTML-style self-closing tags (`<br />`), so this normalization
   was wrong. Affected all void elements (br, hr, img, meta, link, etc.).

2. `markdownify` filter used `markdown_to_html` which calls full `kramdown::postprocess`
   including `add_block_spacing`. This added an extra newline after `</p>`, making
   `</p>\n` become `</p>\n\n`. Combined with the template's own newline, this produced
   `</p>\n\n\n` (triple newline) vs Jekyll's `</p>\n\n` (double newline).

**Fixes applied:**

1. Removed `normalize_void_elements` from `kramdown::postprocess` and
   `kramdown::normalize_html_output`. Jekyll/kramdown preserves XHTML-style
   self-closing tags, so rustkyll should too. The function is kept as `#[cfg(test)]`
   for its own unit tests.

2. Created `markdown_to_html_for_filter` in frontmatter.rs and
   `postprocess_for_filter` in kramdown.rs. The markdownify filter now uses
   lighter postprocessing that skips `add_block_spacing` and other block-level
   transforms not needed for inline filter output.

**Remaining differences (out of scope for this issue):**
- Date: "18 Dec 2020" vs Jekyll's "19 Dec 2020" for `end: 2020-12-18 23:59:59` -- YAML timestamp timezone handling issue
- `<li>` indentation: pulldown-cmark outputs `<li>` without indent, kramdown uses 2-space indent -- cosmetic, no pixel difference
- JSON-LD structured data block present in rustkyll but not Jekyll -- separate feature

**Files modified:**
- `src/kramdown.rs` -- removed normalize_void_elements from postprocess/normalize_html_output, added postprocess_for_filter
- `src/frontmatter.rs` -- added markdown_to_html_for_filter
- `src/template/filters/markdownify.rs` -- uses markdown_to_html_for_filter, added 4 new tests
- `tests/integration_books.rs` -- updated test to expect `<br />` instead of `<br>`

**Test results:** 1,206 tests pass, 0 fail. Clippy clean, fmt clean.
