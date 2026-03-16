# Issue 124: Fix kramdown loose list <p> wrapping

## Problem

mojombo-blog posts show pixel diffs (3.49%, 1.56%) from kramdown's loose list <p> wrapping. When a list item contains multiple paragraphs (separated by blank lines), kramdown wraps each paragraph in <p> tags. Pulldown-cmark may handle this differently.

Related to issue #114 (bare text wrapping) but specific to list items with multiple paragraphs.

## Acceptance criteria
- mojombo-blog post-readme-driven achieves 0% pixel diff
- mojombo-blog post-open-source achieves 0% pixel diff
- List items with blank-line-separated paragraphs match kramdown output
- No regressions

## Log

### [SWE] 2026-03-16
- Root cause: `strip_paragraphs_in_html_blocks` in kramdown.rs strips `<p>` tags from ALL `<li>` elements, including bare `<li>` from markdown loose lists where kramdown correctly wraps content in `<p>` tags.
- Fix: In `strip_p_in_tag`, when processing `<li>` elements, skip bare `<li>` (no attributes) since those come from markdown loose list syntax. Only strip `<p>` from `<li>` with attributes (e.g. `<li class="podcast">`), which come from raw HTML/Liquid includes where pulldown-cmark erroneously inserts `<p>` wrappers.
- TDD: Wrote 5 failing tests first, then implemented fix, all pass.
- Updated 1 existing test (`test_strip_p_in_nested_ul_li`) to use `<li>` with attributes (realistic scenario); added `test_preserve_p_in_bare_li` to document new behavior.
- Tests added: 7 new tests (5 for issue 124 + 2 updated/new for existing behavior)
- Build: 1222 unit tests pass, 0 fail; clippy clean; fmt clean
- Files modified: src/kramdown.rs
