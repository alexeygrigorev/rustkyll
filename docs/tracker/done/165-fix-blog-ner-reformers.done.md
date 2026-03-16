# Issue 165: Fix blog/ner-reformers.html (200 diffs)

Sixth highest DOM diff blog post. Investigate and fix rendering differences. TDD per pattern.

## Log

### [SWE] 2026-03-16

- Investigated diffs between Jekyll and rustkyll output for blog/ner-reformers.html
- Found exactly 2 categories of differences (130 diff lines total):
  1. **Tight list `<li>` indentation**: Rustkyll added 2-space indent to `<li>` in tight lists; Jekyll does not indent them
  2. **Comment spans in syntax highlighting**: Rustkyll moved trailing newlines outside `</span>` tags; Jekyll keeps newlines inside the span
- Root cause #1: `indent_list_items()` in kramdown.rs had a tight-list branch that added 2-space indent to `<li>` elements -- already fixed by issue 166 changes in working tree
- Root cause #2: `wrap_fenced_code_blocks()` in kramdown.rs had `highlighted.replace("\n</span>", "</span>\n")` which moved newlines outside spans -- already fixed by issue 166 changes in working tree
- Added 3 tests specific to issue 165:
  - `test_issue165_python_comment_trailing_newline_inside_span` (syntax.rs) -- verifies Python comment spans include trailing newline
  - `test_issue165_tight_list_no_indent` (kramdown.rs) -- verifies tight list `<li>` is NOT indented
  - `test_issue165_python_comment_newline_in_code_block` (kramdown.rs) -- verifies full pipeline preserves newline inside comment spans
- After rebuild: 0 diffs remaining for blog/ner-reformers.html
- Build: 1551 tests pass, 0 fail, clippy clean, fmt clean
- Files modified: src/kramdown.rs, src/syntax.rs
