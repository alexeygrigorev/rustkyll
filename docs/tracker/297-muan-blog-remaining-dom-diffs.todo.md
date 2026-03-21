# Issue 297: muan-blog remaining 103 DOM diff pages

## Problem

muan-blog matches 2115/2218 (95%). 103 pages have diffs: list item indentation in strip_html, details/summary newlines, text content diffs.

## Acceptance Criteria

- [ ] muan-blog DOM match improves (target: 2150+/2218)
- [ ] No regressions on other sites
- [ ] cargo test passes
