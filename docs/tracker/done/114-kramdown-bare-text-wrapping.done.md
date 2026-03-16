# Issue 114: Fix kramdown bare text auto-wrapping between blocks

Affects courses/2021-winter-ml-zoomcamp.html (4.12% pixel diff). Kramdown auto-wraps loose inline text between block elements (h3, ul) in <p> tags. Pulldown-cmark does not.

## Acceptance criteria
- Bare text between block elements wrapped in <p> matching kramdown
- Course page achieves 0% pixel diff

## Log

### [SWE] 2026-03-16
- Root cause: pulldown-cmark does not wrap bare inline text between block-level HTML elements in `<p>` tags. Kramdown does this automatically. The affected text comes from Liquid template output (e.g., `{{ session.subtitle }}` between `<h3>` and `<ul>`).
- Added `wrap_bare_text_in_paragraphs()` function to `src/kramdown.rs` that detects bare text at the top level between block elements and wraps it in `<p>` tags.
- Key design: tracks nesting depth of container elements (ul, div, pre, etc.) so text inside containers is not wrapped. Only top-level bare text between block elements gets wrapped.
- Added to the `postprocess()` pipeline, runs before `add_block_spacing()`.
- Tests added: 10 unit tests for wrap_bare_text_in_paragraphs
- Build: 1169 lib tests pass, 0 fail; all integration tests pass; clippy clean; fmt clean
- Files modified: src/kramdown.rs
- Verified output: course page bare text now correctly wrapped in `<p>` tags matching kramdown behavior
