# Issue 225: Fix DTC title word truncation

## Problem

1 DTC page `how-do-data-professionals-use-data-engineering-tools-and-practices.html` has "Data" missing from title: "How Do Professionals Use..." instead of "How Do Data Professionals Use...". Also shows wrong image path dates (2025-04-15 vs 2025-04-29) and wrong description.

Likely a front matter parsing issue or duplicate post slug collision where two posts resolve to the same output path and one overwrites the other.

## Scope

1. Check for duplicate post slugs or conflicting output paths in the DTC site
2. If a slug collision, fix slug resolution to match Jekyll behavior
3. If a front matter parsing issue, identify and fix the parser bug
4. Verify the correct title, image path, and description appear in the output

## Acceptance Criteria

- [ ] Page title is "How Do Data Professionals Use..." (complete, no truncation)
- [ ] Image path dates and description match Jekyll output
- [ ] No duplicate slug collisions in DTC site output
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all new and existing tests

## Log

- 2026-03-18: Created from DTC comparison analysis.
