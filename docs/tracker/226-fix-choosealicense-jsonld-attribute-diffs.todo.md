# Issue 226: Fix choosealicense.com JSON-LD and attribute diffs

## Problem

choosealicense.com matches only 15/72 (21%). Main diffs: 211 jsonld_value_differs, 179 attribute_differs, 44 text_differs, 44 jsonld_missing_field. The site uses jekyll-seo-tag extensively.

## Scope

1. Build choosealicense.com with rustkyll and compare against Jekyll reference
2. Investigate the specific JSON-LD fields that differ (211 value diffs, 44 missing fields)
3. Investigate attribute_differs patterns (179 diffs)
4. Fix systematic patterns in JSON-LD generation and attribute rendering
5. Address text_differs (44 diffs) if related to the above

## Acceptance Criteria

- [ ] JSON-LD field values match Jekyll output for choosealicense.com pages
- [ ] Missing JSON-LD fields are added to match Jekyll output
- [ ] Attribute diffs are resolved or root-caused
- [ ] Match rate improves substantially from 21%
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all new and existing tests

## Log

- 2026-03-18: Created from cross-site comparison analysis.
