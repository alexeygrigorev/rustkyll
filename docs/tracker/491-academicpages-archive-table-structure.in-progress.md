# Issue 491: academicpages archive table structure
## Problem
Missing tbody/tfoot, wrong dl/h2 tags. 1 page.
## Affected Sites
- academicpages
## Baseline
DTC 790/790. academicpages 27/45. Must not regress.

## Sub-problems

(a) kramdown table tbody/tfoot separators -- already fixed by issue #515
(b) kramdown definition lists (Term + `:   Definition`) rendering as `<p>` instead of `<dl>/<dt>/<dd>`

## Acceptance Criteria

1. Definition list syntax in kramdown mode produces `<dl>/<dt>/<dd>` HTML
2. Table tfoot/tbody separators produce correct HTML (already working)
3. Inline markdown (links, emphasis) inside definition text is rendered
4. Code blocks are not affected (no false positives)
5. CommonMark mode does not convert definition lists
6. DTC DOM baseline maintained at 787/787

## Log

### [SWE] 2026-03-30
- Verified sub-problem (a) is already resolved by issue #515 (tfoot/tbody working)
- Wrote 7 unit tests for definition list conversion in kramdown.rs (test_491_*)
- Wrote 3 integration tests in frontmatter.rs (test_issue491_*)
- Ran tests: all FAIL as expected (function not found)
- Implemented `convert_kramdown_definition_lists()` in src/kramdown.rs
  - Pre-processes kramdown definition list syntax to HTML before pulldown-cmark
  - Renders inline markdown (links, emphasis) within dt/dd elements
  - Skips code blocks, ATX headings, and other non-term lines
- Added `render_dl_inline_markdown()` helper to process inline markdown
- Added `is_potential_dl_term_line()` and `is_definition_marker_line()` helpers
- Wired into 3 pipeline functions in src/frontmatter.rs:
  - `markdown_to_html` (kramdown mode)
  - `markdown_to_html_with_options` (only when add_code_classes=true)
  - excerpt variant (kramdown mode)
- Ran tests: all 10 new tests PASS
- Full test suite: 3431 lib + all integration tests pass, 0 failures
- Clippy: clean (no warnings from our code)
- Fmt: clean
- DTC DOM baseline: 787/787 match, 0 differences
- Verified academicpages archive-layout-with-content output:
  - Definition lists render as `<dl>/<dt>/<dd>`
  - Links inside definitions render as `<a>` tags
  - Table tfoot/tbody structure correct
- Files modified: src/kramdown.rs, src/frontmatter.rs
- Files renamed: docs/tracker/491-*.todo.md -> .in-progress.md
