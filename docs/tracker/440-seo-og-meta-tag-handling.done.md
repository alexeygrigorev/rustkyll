# Issue 440: SEO/OG meta tag handling for theme sites

## Problem

Several sites have SEO/OG meta tag ordering and content issues in `<head>`.
The `jekyll-seo-tag` plugin generates meta tags in a specific order that
rustkyll doesn't match for complex theme configurations.

## Affected Sites

- aihero (0/2, 178 diffs) — OG tag ordering
- so-simple-theme (0/11, 624 diffs) — author data corruption in meta tags
- basically-basic (0/7, 399 diffs) — title mismatch, OG ordering

## Root Cause

`src/template/seo_tag.rs` doesn't handle complex `site.author` objects
(maps with name/email/twitter/links) correctly. Data is serialized as
`__key_order...` strings instead of properly rendered.

## Scope

Fix SEO tag rendering for complex author/site config objects.

## Log

### [SWE] 2026-03-28
- TDD: Wrote 5 tests for complex author object handling (string author, object author with name extraction, empty name suppression, JSON-LD with object author, page author object overriding site author)
- Ran tests: 4 FAILED, 1 passed as expected -- object authors were serialized as concatenated key-value strings (e.g., "emailjohn@example.comnameJohn Smith")
- Root cause: `get_nested_str()` calls `val.to_kstr()` which flattens map objects into concatenated strings
- Implemented fix: added `get_author_name()` function that checks `val.as_object()` and extracts the "name" field when author is a map, falls back to `to_kstr()` for scalar strings
- Ran tests: all 5 new tests PASS, all 3044 total tests PASS (0 failures)
- Clippy: clean (pre-existing warnings in generator.rs only, not in seo_tag.rs)
- Fmt: clean for seo_tag.rs
- DTC DOM: 790/790 (no regression)
- aihero DOM: 0/2 (pre-existing, unrelated to author object handling -- aihero uses string author)
- Files modified: src/template/seo_tag.rs
