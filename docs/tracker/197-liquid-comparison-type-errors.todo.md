# Issue 197: Fix Liquid comparison type errors

## Origin

Descoped from issue 196 (fix layout not applied). These are Liquid rendering errors, not layout resolution issues.

## Problem

Several sites fail to render pages because of Liquid type errors in comparisons and filters:

- **academicpages (5 posts)**: `template render error: liquid: --> 16:53`
- **beautiful-jekyll (6 pages)**: `Expected array, found string` and render errors at line 16:56
- **government-github (8 pages)**: Liquid render/parse errors at line 12:16 and 32:16
- **muan-blog (5 pages)**: `Invalid input` errors from Liquid filters on non-array types (e.g., `reverse` on nil)
- **just-the-docs (1 page)**: Liquid parse error at line 36:22

## Root Cause

The liquid-rs crate is stricter than Ruby Liquid about type coercion:
- Filters like `reverse`, `sort`, `join` fail on nil instead of returning empty
- Comparison operators (`>`, `<`) may fail on float vs integer comparisons
- Some Liquid syntax patterns in real-world templates are not handled

## Acceptance Criteria

- [ ] Filters on nil values return sensible defaults (empty string, empty array) instead of errors
- [ ] Float-to-integer comparisons work in Liquid conditionals
- [ ] At least the academicpages and beautiful-jekyll pages render correctly
