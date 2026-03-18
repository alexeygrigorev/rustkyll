# Issue 220: Fix muan-blog smart quotes in body text

## Problem

~350 muan-blog note pages show `text_differs` where Jekyll uses straight apostrophe (`'` U+0027) but rustkyll uses curly RIGHT SINGLE QUOTATION MARK (`'` U+2019). This is from pulldown-cmark's smart punctuation feature which is enabled by default. Jekyll's CommonMark processor does not enable smart quotes by default.

## Scope

1. Identify where pulldown-cmark options are configured in rustkyll
2. Disable smart punctuation (smart quotes) to match Jekyll's default behavior
3. Verify muan-blog body text matches Jekyll output

## Acceptance Criteria

- [ ] Straight apostrophes in markdown source remain as straight apostrophes in HTML output
- [ ] Smart punctuation is disabled by default, matching Jekyll's CommonMark behavior
- [ ] ~350 muan-blog text_differs diffs are resolved
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all new and existing tests
- [ ] Tests include content with apostrophes and single quotes

## Log

- 2026-03-18: Created from muan-blog comparison analysis.
