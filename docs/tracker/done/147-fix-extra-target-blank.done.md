# Issue 147: Fix extra target='_blank' on links

## Problem

Rustkyll adds `target='_blank'` to some anchor elements where Jekyll does not. 3 instances across 2 files.

Discovered in issue #119 DOM diff audit.

## Acceptance criteria

- Links only have `target='_blank'` when Jekyll also adds it
- No regressions

## Log

### [SWE] 2026-03-16
- Root cause: IAL processing (`apply_attributes_to_last_tag`) was applying `{:target="_blank"}` to the last closing tag in the output, regardless of tag type. In 3 cases, the markdown link wasn't parsed by pulldown-cmark (due to parentheses in URL or HTML block context), so the IAL attached to `<figure>`, `<strong>`, or `<em>` instead of `<a>`.
  - Case 1: `</figure>Photo by [link](url){:target="_blank"}` - link on same line as closing HTML block, not parsed as markdown
  - Case 2 & 3: `[Wikipedia](https://en.wikipedia.org/wiki/Docker_(software){:target="_blank"})` - URL contains parentheses, confusing link parser
- Fix: Added guard in `apply_attributes_to_last_tag`: if the IAL contains a `target` attribute and the last closing tag is NOT `<a>`, skip the IAL. The `target` attribute is only meaningful on `<a>` elements in kramdown.
- Tests added: 2 new tests in kramdown.rs (negative test for figure/strong/em, positive test for a tags)
- Build: 1257 tests pass, 0 fail, clippy clean, fmt clean
- Files modified: src/kramdown.rs
