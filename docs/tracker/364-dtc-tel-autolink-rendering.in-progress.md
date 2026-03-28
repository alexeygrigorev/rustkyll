# Issue 364: DTC tel: autolink rendering

## Parent

Follow-up from #363 (RC-C).

## Problem

Text like `<tel:100-1000|100-1000>` is being parsed as an autolink producing an `<a>` element. Jekyll/kramdown does not autolink `tel:` URIs -- it renders the pipe as a literal character and keeps the text inline with `<br>`.

## Affected Pages

- `books/20211004-transfer-learning-in-action.html` (5 diffs)

## Acceptance Criteria

- [ ] `tel:` URIs are not converted to `<a>` autolinks in markdownify output
- [ ] Pipe character inside angle-bracket `tel:` expression renders as literal text
- [ ] DTC DOM match count does not regress
- [ ] No site-specific hardcoding

## Priority

LOW

## Log

### [SWE] 2026-03-28

- Investigated: kramdown treats `|` in `<tel:100-1000|100-1000>` as a table cell
  delimiter in SINGLE-LINE list items, producing
  `<table><tbody><tr><td>...</td><td>...</td></tr></tbody></table>`.
  Multi-line list items (with continuation text after `<br />`) keep the pipe literal.
- Wrote failing test `test_issue364_pipe_in_tel_produces_table` -- FAILS (no table produced)
- Wrote test `test_issue364_pipe_multiline_no_table` for the multi-line case
- Changed `escape_non_standard_autolink_schemes` to use `RKPIPEMARK` marker instead of `&#124;`
- Added `convert_pipe_markers_to_tables` post-processing function that:
  - Finds `RKPIPEMARK` markers in HTML output
  - Checks if the enclosing `<li>` or `<p>` has continuation text (newline after marker)
  - Single-line: converts to `<table><tbody><tr><td>` matching kramdown
  - Multi-line: replaces marker with literal `|`
- Called `convert_pipe_markers_to_tables` in both `markdown_to_html` and `markdown_to_html_for_filter`
- Updated `test_issue364_markdown_to_html_also_escapes` to expect `<table>`
- All tests pass: 13 issue364 tests, full suite passes (0 failures across 30 suites)
- Clippy clean, fmt clean
- DTC DOM: 787/790 (was 786/790, target met)
- No regression on transfer-learning page (correctly stays literal)
- Reliable-machine-learning improved from 7 to 2 diffs (bonus)
- Files modified: `src/frontmatter.rs`
