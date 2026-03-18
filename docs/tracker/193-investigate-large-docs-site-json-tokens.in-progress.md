# Issue 193: Investigate large-docs-site JSON string token splitting (still broken)

## Problem

Issue 180 added a JSON scope rule to merge string delimiter tokens, but large-docs-site still shows 500 pages with the same diff:
```
body > main > article > div > div > pre > code > span: text_differs
  expected: '"'
  actual:   '"example"'
```

The fix may not be matching the correct scope for this specific JSON code block format, or the synthetic site uses a different code block language tag.

## Goal

Investigate why the fix didn't work and fix it. Impact: +500 pages on large-docs-site.

## Approach (TDD)

1. Build large-docs-site with rustkyll and inspect the actual HTML output for a failing page
2. Check what language tag the code blocks use (json vs yaml vs none)
3. Write a test reproducing the exact code block from the site
4. Fix the scope mapping or post-processing
5. Verify with recount

## Log

### [SWE] 2026-03-18

- Investigated the 500-page diff in large-docs-site
- Root cause: The failing pages use YAML code blocks (not JSON), containing `setting2: "example"`. Issue 180 only added a JSON-specific scope rule (`source.json punctuation.definition.string` -> `s2`), missing YAML entirely.
- In YAML, syntect gives both the opening/closing quotes and the string content the `string.quoted.double` scope. Without YAML-specific rules, the generic `string.quoted.double` -> `s2` mapping applied to all three tokens, causing them to merge into one `<span class="s2">"example"</span>`.
- Rouge/Jekyll actually splits YAML double-quoted strings: opening quote gets `s2`, content gets `s` (generic string). These are different classes so they render as separate spans.
- Fix: Added two YAML-specific scope rules (before the generic rules):
  - `source.yaml punctuation.definition.string` -> `s2` (quotes get s2)
  - `source.yaml string.quoted.double` -> `s` (content gets s)
- Tests added: 2 new tests (test_yaml_double_quoted_string_split_spans, test_yaml_config_block_with_quoted_string)
- All 1494+ tests pass, 0 failures, clippy clean, fmt clean
- Files modified: src/syntax.rs
