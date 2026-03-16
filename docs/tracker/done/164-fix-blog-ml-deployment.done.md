# Issue 164: Fix blog/ml-deployment-lambda.html (315 diffs)

Fifth highest DOM diff blog post. Investigate and fix rendering differences. TDD per pattern.

## Acceptance Criteria

1. blog/ml-deployment-lambda.html output matches Jekyll exactly (0 diffs)
2. Regression tests cover the diff patterns found
3. All existing tests still pass
4. Clippy clean, fmt clean

## Diff Patterns Found and Fixed

After rebuilding with current code plus uncommitted changes from issue 163, the following patterns were identified and fixed:

1. **Tight list `<li>` indentation**: Rustkyll added 2-space indent before `<li>` in tight lists. Jekyll does NOT indent tight list items. Fixed in `indent_list_items()`.
2. **Blockquote content indentation + missing blank line**: Rustkyll indented `<p>` inside `<blockquote>` by 2 spaces and skipped blank lines. Jekyll does not indent and preserves a blank line before `</blockquote>`. Fixed in `indent_blockquote_content()`.
3. **Syntax highlighting span trailing newlines**: Issue 163's uncommitted changes stripped trailing newlines from spans, but Rouge/Jekyll keeps them inside. Reverted `flush_pending()` to keep newlines inside spans.
4. **Author description trailing newline in JSON-LD**: `item.html_content.trim_end()` stripped trailing whitespace, but Jekyll preserves it. Fixed in `generator.rs`.

Additional patterns (already fixed by previous issues): code block closing divs, figcaption `<p>` preservation, blank lines after `</figure>`.

## Log

### [SWE] 2026-03-16

- Built both Jekyll and rustkyll sites and compared blog/ml-deployment-lambda.html
- Identified 4 active diff patterns after accounting for uncommitted issue 163 changes
- Wrote failing tests for each pattern, then fixed the code (TDD approach)
- Tests added (in kramdown.rs):
  - test_issue164_code_block_closing_divs_on_one_line
  - test_issue164_figcaption_p_preserved_end_to_end
  - test_issue164_blank_line_after_figure
  - test_issue164_code_block_then_figure_combo
  - test_issue164_tight_list_no_indent
  - test_issue164_blockquote_format_matches_jekyll
- Tests fixed (in syntax.rs):
  - test_issue163_yaml_comment_trailing_newline_inside_span (corrected assertion to match actual Jekyll behavior)
- Code fixes:
  - kramdown.rs: `indent_list_items()` -- tight lists no longer indented
  - kramdown.rs: `indent_blockquote_content()` -- no indentation, blank line before `</blockquote>`
  - syntax.rs: `flush_pending()` -- trailing newlines kept inside spans
  - syntax.rs: removed injected `fix_trailing_newlines_in_spans` dead function
  - generator.rs: removed `trim_end()` from collection item content
- Build: 1319 tests pass, 0 fail
- Clippy clean, fmt clean
- Files modified: src/kramdown.rs, src/syntax.rs, src/generator.rs, src/frontmatter.rs (pre-existing tests from issue 162)
- Final diff count: 0 (exact match with Jekyll output)
