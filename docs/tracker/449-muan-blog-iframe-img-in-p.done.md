# Issue 449: muan-blog iframe/img wrapped in <p> tags (9 diffs, 7 files)

## Problem
Block-level HTML elements (iframe, img) are wrapped in <p> tags by
the markdown parser instead of being passed through as-is.

## Scope
Fix markdown HTML block passthrough for iframe and img elements.

## Baseline
DTC 790/790. muan-blog 2194/2218.

## Log

### [SWE] 2026-03-28
- TDD: Wrote 5 failing tests (test_unwrap_iframe_from_p_tag, test_unwrap_img_with_style_from_p_tag, test_unwrap_img_with_width_from_p_tag, test_markdown_img_stays_in_p_tag, test_unwrap_does_not_affect_img_inside_link)
- TDD: Tests FAILED as expected -- pulldown-cmark wraps iframe/img in p tags
- Root cause: `wrap_bare_text_in_paragraphs()` in kramdown.rs did not list `iframe` or `img` in its BLOCK_TAGS/CONTAINER_TAGS constants, so standalone iframe/img lines were treated as bare text and wrapped in `<p>` tags during postprocessing
- Fix: Added `iframe` to both CONTAINER_TAGS and BLOCK_TAGS, added `img` to BLOCK_TAGS in `wrap_bare_text_in_paragraphs()`
- TDD: All 5 tests PASS after fix
- Full test suite: 3055 tests pass, 0 fail
- Clippy: clean (no warnings from our code)
- Fmt: clean
- DOM: DTC 790/790 (no regression), muan-blog 2199/2218 (was 2195, +4 pages fixed)
- Fixed pages: posts/acceptance.html, posts/details-on-details.html, posts/mission-focused.html, posts/presence.html fully fixed; posts/leaving-github.html and posts/noise.html iframe diffs fixed (remaining diffs are unrelated)
- Files modified: src/kramdown.rs
