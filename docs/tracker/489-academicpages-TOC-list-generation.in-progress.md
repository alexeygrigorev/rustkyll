# Issue 489: academicpages TOC list generation
## Problem
TOC nav not generating ul structure. 2 pages.
## Affected Sites
- academicpages
## Baseline
DTC 788/790. academicpages 27/45. Must not regress.

## Log

### [SWE] 2026-03-30
- Root cause: `{:toc}` kramdown IAL inside `markdown="1"` blocks was being processed by
  pulldown-cmark (which doesn't understand kramdown IALs), rendering it as literal text.
  The kramdown parser's `generate_toc()` was never invoked for the main markdown pipeline.
- TDD: Wrote `test_issue489_toc_placeholder_in_markdown1_block` -- FAILS as expected
  (output shows literal `{:toc .toc__menu}` text instead of `<ul id="markdown-toc">`)
- Fix: Implemented TOC placeholder mechanism:
  1. `replace_toc_pattern_with_placeholder()` detects `* text\n{:toc ...}` pattern and
     replaces with HTML comment placeholder `<!-- KRAMDOWN_TOC:classes -->`
  2. `generate_toc_from_headings()` scans HTML for `<hN id="...">` tags and builds
     nested `<ul>` with proper `id="markdown-toc"` and `id="markdown-toc-*"` attributes
  3. `replace_toc_placeholders()` runs in `postprocess_with_options` after `add_heading_ids`
     to replace placeholders with actual TOC HTML
  4. Public entry point `replace_toc_pattern_in_markdown()` called from both
     `markdown_to_html()` and `markdown_to_html_with_options()` for standalone `{:toc}` usage
- Ran tests: 3 new tests PASS, all 3419 existing tests PASS
- Verified academicpages: terms/index.html and markdown/index.html both generate correct TOC
- DTC DOM: 788/790 (no regression)
- Clippy clean, fmt clean
- Files modified: src/kramdown.rs, src/frontmatter.rs
