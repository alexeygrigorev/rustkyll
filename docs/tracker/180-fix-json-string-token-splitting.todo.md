# Issue 180: Fix JSON string token splitting in syntax highlighting

## Problem

In JSON code blocks, Jekyll/Rouge renders `"example"` as a single `<span class="s2">"example"</span>` token. rustkyll/syntect splits it into separate tokens: `"` + `example` + `"`. This causes 500 DOM diffs on large-docs-site (every api-reference page).

Sample diff:
```
body > main > article > div > div > pre > code > span: text_differs
  expected: '"'
  actual:   '"example"'
body > main > article > div > div > pre > code > span: missing_element
  expected: '<span>'
  actual:   '(none)'
```

## Goal

Merge adjacent JSON string delimiter tokens with string content tokens in `src/syntax.rs` so the output matches Rouge.

## Affected Sites

- large-docs-site: 500/801 pages affected (currently 301/801 match, expected ~801/801 after fix)

## Approach (TDD)

1. Write a test in syntax.rs that highlights a JSON snippet with `"example"` and asserts it produces a single `<span class="s2">"example"</span>`
2. Verify the test fails
3. Add JSON-specific post-processing in `src/syntax.rs` to merge string delimiter + content tokens
4. Verify the test passes
5. Run `./scripts/recount-all-dom.sh --site large-docs-site` to confirm improvement

## Acceptance Criteria

- [ ] JSON `"string"` values render as single `<span>` matching Rouge output
- [ ] Existing syntax highlighting tests still pass
- [ ] large-docs-site DOM match improves significantly
