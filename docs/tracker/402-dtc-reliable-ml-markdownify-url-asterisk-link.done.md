# Issue 402: DTC reliable-machine-learning markdownify URL asterisk link fix

## Problem

The `books/20221121-reliable-machine-learning.html` page had 7 DOM diffs caused by an
O'Reilly URL with Google Analytics tracking parameters containing asterisks:

```
[https://.../?_gl=1*95hemv*_ga*MTA2...](https://.../?_gl=1*95hemv*_ga*MTA2...)
```

This URL goes through `{{ reply.text | newline_to_br | markdownify }}` in the book layout.
pulldown-cmark treated the `*` characters as emphasis markers, breaking the link entirely
and producing `<em>` tags instead of an `<a>` link.

Jekyll/kramdown correctly parses `[url](url)` as a link even when the URL contains asterisks,
producing `<a href="url">text</a>`.

### Root cause

Issue 378 incorrectly removed `protect_url_link_text_emphasis()` from the
`markdown_to_html_for_filter` pipeline, based on the wrong assumption that Jekyll produces
`<em>` tags. The actual `_site_jekyll_cached` output shows Jekyll produces a proper `<a>` link.

## Fix

Re-added `protect_url_link_text_emphasis()` to the `markdown_to_html_for_filter` pipeline
(the markdownify Liquid filter path). This escapes `*` inside `[url://...](url)` markdown
link patterns before pulldown-cmark parses them, so pulldown-cmark correctly produces `<a>` links.

## Results

- reliable-machine-learning page: 7 diffs -> 2 diffs (5 fixed)
- Remaining 2 diffs: Jekyll strips `?_gl=...` query params from the URL, rustkyll preserves them.
  Both produce `<a>` tags (correct structure). Query param stripping is a separate issue.
- transfer-learning-in-action: 5 -> 3 diffs (improvement from emphasis handling)
- natural-language-processing-with-transformers: 3 -> 1 diff (improvement)
- DTC DOM: 785/790 (no regression -- baseline was 785/790 before this change)
- Total diff count: 339 -> 330

## Files modified

- `src/frontmatter.rs` -- re-added `protect_url_link_text_emphasis()` to `markdown_to_html_for_filter`
- `tests/test_issue_378.rs` -- updated tests to expect `<a>` links (matching actual Jekyll behavior)
- `tests/test_issue_367.rs` -- updated comment on markdownify test
- `tests/test_issue_390_url_emphasis.rs` -- updated test to expect `<a>` link instead of documenting known bug

## Log

### [SWE] 2026-03-28
- **TDD cycle:**
  - Wrote 6 tests in `tests/test_issue_378.rs` asserting markdownify produces `<a>` links from URL asterisks
  - Ran tests: 3 FAILED as expected (markdownify produced `<em>` instead of `<a>`)
  - Implemented fix: re-added `protect_url_link_text_emphasis()` to `markdown_to_html_for_filter`
  - Ran tests: all 6 PASS
- Updated `tests/test_issue_367.rs` comment for clarity
- Updated `tests/test_issue_390_url_emphasis.rs` to assert `<a>` link (no longer documenting known bug)
- **Build:** all tests pass (2922 lib + integration), clippy clean, fmt clean
- **DOM verification:**
  - Without change: 785/790, reliable-ML has 7 diffs
  - With change: 785/790, reliable-ML has 2 diffs
  - No regression. Total diffs reduced from 339 to 330.
