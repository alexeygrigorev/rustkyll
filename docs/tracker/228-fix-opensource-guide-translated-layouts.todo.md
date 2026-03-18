# Issue 228: Fix opensource-guide translated page layouts

## Problem

opensource-guide matches only 23/388 (6%). Translated pages (ar/, es/, fa/, etc.) don't have layouts applied. The nil-contains fix in issue 196 was supposed to help but didn't. Investigate remaining Liquid errors preventing layout application.

## Scope

1. Build opensource-guide with rustkyll and check build logs for Liquid errors
2. Identify why translated pages don't get layouts applied
3. Check if the issue is in front matter parsing, layout resolution, or Liquid rendering
4. Fix layout application for translated pages
5. Investigate remaining diffs after layouts are applied

## Acceptance Criteria

- [ ] Translated pages (ar/, es/, fa/, etc.) have layouts applied correctly
- [ ] Match rate improves substantially from 6%
- [ ] Liquid errors related to translated pages are resolved
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all new and existing tests
- [ ] Tests include non-ASCII content from translated pages

## Dependencies

- Issue 196 (nil-contains fix) -- already done, but insufficient

## Log

- 2026-03-18: Created from cross-site comparison analysis.
