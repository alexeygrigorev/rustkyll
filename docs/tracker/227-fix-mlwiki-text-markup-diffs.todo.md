# Issue 227: Fix mlwiki.org text and markup diffs

## Problem

mlwiki.org matches only 214/639 (33%). Has 1058 text_differs, 991 tag_name_differs, 401 missing_element. The site uses MediaWiki-style markup. Many diffs likely from custom markup processing that rustkyll doesn't handle.

## Scope

1. Build mlwiki.org with rustkyll and compare against Jekyll reference
2. Investigate top failure patterns in tag_name_differs (991) and text_differs (1058)
3. Determine if the site uses custom plugins or markup processing
4. Fix systematic patterns that account for the most diffs
5. Identify any diffs that are out of scope (custom plugin behavior)

## Acceptance Criteria

- [ ] Top failure patterns are identified and documented
- [ ] Systematic markup rendering issues are fixed
- [ ] Match rate improves substantially from 33%
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all new and existing tests

## Log

- 2026-03-18: Created from cross-site comparison analysis.
