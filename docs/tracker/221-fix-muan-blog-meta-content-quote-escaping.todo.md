# Issue 221: Fix muan-blog meta content quote escaping

## Problem

~350 muan-blog pages show `attribute_differs` in meta content tags. Jekyll uses double quotes with escaped apostrophe: `content="...doesn\'t..."`. Rustkyll uses single quotes: `content='...doesn't...'`. The SEO tag template should use double-quoted attributes when content contains apostrophes.

## Scope

1. Identify where meta tag attributes are rendered (likely SEO tag / template code)
2. Ensure meta content attributes use double quotes, with proper escaping of contained quotes
3. Match Jekyll's attribute quoting behavior

## Acceptance Criteria

- [ ] Meta content attributes use double quotes matching Jekyll output
- [ ] Apostrophes inside double-quoted attributes are properly escaped
- [ ] ~350 muan-blog attribute_differs diffs are resolved
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all new and existing tests
- [ ] Tests include meta content with apostrophes and double quotes

## Log

- 2026-03-18: Created from muan-blog comparison analysis.
