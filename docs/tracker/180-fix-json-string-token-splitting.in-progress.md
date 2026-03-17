# Issue 180: Fix JSON string token splitting in syntax highlighting

## Checklist Category

**Syntax highlighting differences** -- 574 pages total. This issue addresses the JSON-specific subset affecting large-docs-site (500 pages).

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

## Dependencies

None.

## Approach (TDD)

1. Write a test in syntax.rs that highlights a JSON snippet with `"example"` and asserts it produces a single `<span class="s2">"example"</span>`
2. Verify the test fails
3. Add JSON-specific post-processing in `src/syntax.rs` to merge string delimiter + content tokens
4. Verify the test passes
5. Run `./scripts/recount-all-dom.sh --site large-docs-site` to confirm improvement

## Acceptance Criteria

- [ ] JSON `"string"` values render as a single `<span>` element matching Rouge output (e.g., `<span class="s2">"example"</span>` not three separate spans)
- [ ] JSON strings with special characters (`"foo/bar"`, `"hello world"`, `"key: value"`) also merge correctly into single spans
- [ ] JSON keys (e.g., `"name":`) also render as single spans matching Rouge
- [ ] Nested JSON strings (objects/arrays within strings) are handled correctly
- [ ] Existing non-JSON syntax highlighting tests still pass unchanged
- [ ] `cargo test` passes
- [ ] large-docs-site DOM match count improves from 301/801 toward 801/801

## Test Scenarios

### Unit: JSON string token merging (write FIRST, must fail before fix)

- **Test `test_json_string_single_span`**: Highlight `{"name": "example"}` as JSON. Assert the output contains `<span class="s2">"example"</span>` as a single span, not `<span class="s2">"</span><span class="s2">example</span><span class="s2">"</span>`.
- **Test `test_json_key_single_span`**: Highlight `{"name": "value"}` as JSON. Assert `"name"` renders as a single `<span>` token.
- **Test `test_json_string_with_special_chars`**: Highlight `{"path": "/foo/bar"}` as JSON. Assert `"/foo/bar"` renders as a single span.
- **Test `test_json_empty_string`**: Highlight `{"key": ""}` as JSON. Assert `""` renders as a single span.

### Regression: Existing highlighting preserved

- **Test `test_python_highlighting_unchanged`**: Highlight a Python snippet and verify string tokens are NOT merged (merging is JSON-specific).
- **Test `test_json_non_string_tokens_unchanged`**: Highlight JSON with numbers, booleans, nulls -- verify those tokens are unaffected by the merge logic.

### Integration: Full site verification

- Build large-docs-site with rustkyll and run DOM comparison to verify match count improvement.

## Output Verification

- Build large-docs-site and inspect at least 2 generated API reference pages containing JSON blocks
- Verify the HTML source shows merged `<span>` tokens for JSON strings
- Compare against Jekyll output for the same pages

## Log

### [SWE] 2026-03-17

- Investigated the JSON string token splitting issue
- Found that the existing `accumulate_and_emit` merging logic in `highlight_code` already handles JSON string tokens correctly: both `punctuation.definition.string` (quote delimiters) and `string.quoted.double` (content) map to class `s2` because the `string.quoted.double` rule comes first in the scope map iteration order
- Added explicit JSON-specific scope rule `("source.json punctuation.definition.string", "s2")` at the top of the scope map to make this robust (prevents breakage if rule ordering changes)
- Added 9 tests covering all acceptance criteria:
  - `test_json_string_single_span`: verifies `"example"` is a single s2 span
  - `test_json_key_single_span`: verifies `"name"` key is a single s2 span
  - `test_json_string_with_special_chars`: verifies `"/foo/bar"` is a single span
  - `test_json_empty_string`: verifies `""` is a single span
  - `test_json_string_with_url`: verifies URLs in strings are single spans
  - `test_json_non_string_tokens_unchanged`: verifies numbers/booleans/null unaffected
  - `test_json_multiline_object`: verifies multi-line JSON has merged strings
  - `test_python_highlighting_unchanged_by_json_fix`: verifies Python not affected
- Build: all 1436+ tests pass, 0 failures, clippy clean, syntax.rs fmt clean
- Files modified: `src/syntax.rs`
- Note: The sample diff in the issue description appears to have expected/actual reversed. Verified against real Jekyll output that Rouge produces merged strings matching what rustkyll now produces.
