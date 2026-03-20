# Issue 255: Support number_of_words filter with optional argument

## Problem

The `number_of_words` filter exists but does not accept the optional mode argument
that Jekyll supports. In Jekyll, this filter accepts an optional string argument
after the colon:

```liquid
{{ content | number_of_words }}
{{ content | number_of_words: 'auto' }}
{{ content | number_of_words: 'cjk' }}
```

When called as `number_of_words: 'auto'` (used by jekyll-theme-chirpy's
`read-time.html` include), rustkyll's Liquid parser fails with:

```
unexpected FilterChain; expected FilterChain
```

This is because the current implementation uses `ParseFilter` without
`FilterParameters`, so the parser does not expect any arguments after the colon.

## Background: Jekyll's number_of_words behavior

Jekyll's `number_of_words` filter (from `jekyll-utils` gem) has three modes:

1. **No argument (default):** Split on whitespace, count segments. Pure Latin word count.
2. **`'cjk'`:** Count each CJK character (Unicode ranges for CJK Unified Ideographs,
   Hiragana, Katakana, etc.) as one word. Non-CJK text is split on whitespace as usual.
   The total is CJK character count + whitespace-separated non-CJK word count.
3. **`'auto'`:** Same behavior as `'cjk'`. In Jekyll's Ruby implementation, both
   `'auto'` and `'cjk'` call the same CJK-aware counting logic.

Any other argument value (or an unrecognized string) falls back to the default
whitespace-only counting.

### CJK Unicode ranges to handle

At minimum, the following Unicode block ranges should be treated as CJK:

- CJK Unified Ideographs: U+4E00..U+9FFF
- CJK Unified Ideographs Extension A: U+3400..U+4DBF
- CJK Compatibility Ideographs: U+F900..U+FAFF
- Hiragana: U+3040..U+309F
- Katakana: U+30A0..U+30FF
- Hangul Syllables: U+AC00..U+D7AF
- CJK Unified Ideographs Extension B+: U+20000..U+2A6DF

This matches Jekyll's `cjk_charset` regex pattern.

## Scope

- Add an optional string parameter to the `number_of_words` filter
- Implement CJK-aware word counting for `'auto'` and `'cjk'` modes
- Default mode (no argument) retains current whitespace-only counting
- Unrecognized mode strings fall back to default counting

## Dependencies

- Issue 30 (missing filters): done

## Implementation notes

The current `NumberOfWordsFilter` uses the no-parameters derive pattern. It needs
to be changed to use `FilterParameters` + `FromFilterParameters`, following the
same pattern as `truncatewords.rs` or `sample.rs`. The argument should be an
`Option<Expression>` with `arg_type = "str"`.

## Acceptance Criteria

- [ ] `number_of_words` filter parses and renders without error when called with no argument (existing behavior preserved)
- [ ] `number_of_words: 'auto'` parses and renders without error
- [ ] `number_of_words: 'cjk'` parses and renders without error
- [ ] Default mode (no arg): counts words by splitting on whitespace (unchanged behavior)
- [ ] CJK mode (`'auto'` or `'cjk'`): each CJK character counts as one word, non-CJK segments counted by whitespace splitting
- [ ] Mixed content (Latin + CJK): correctly sums CJK character count and Latin word count
- [ ] Unrecognized mode string (e.g., `'foo'`) falls back to default whitespace counting
- [ ] Non-ASCII Latin text (accented characters like "cafe" with accents) is NOT treated as CJK -- only actual CJK Unicode ranges
- [ ] Empty string returns 0 in all modes
- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` passes
- [ ] All existing `number_of_words` tests continue to pass (no regressions)
- [ ] New tests cover all modes and edge cases (at least 10 new tests)

## Test Scenarios

### Unit: default mode (no argument)

- `"Hello world" | number_of_words` => 2 (existing test, must still pass)
- `"" | number_of_words` => 0 (existing test)
- `"  spaces  between  words  " | number_of_words` => 3 (existing test)

### Unit: CJK mode ('auto' and 'cjk')

- `"Hello world" | number_of_words: 'auto'` => 2 (pure Latin, same as default)
- `"Hello world" | number_of_words: 'cjk'` => 2 (pure Latin, same as default)
- Pure CJK: a string of 5 Chinese characters with `'auto'` => 5
- Pure Japanese hiragana: 3 hiragana characters with `'auto'` => 3
- Pure Korean hangul: 4 hangul syllables with `'auto'` => 4

### Unit: mixed content

- `"Hello 世界" | number_of_words: 'auto'` => 3 (1 Latin word + 2 CJK characters)
- Latin sentence with one CJK word embedded: verify correct sum
- CJK characters separated by spaces: each CJK char still counts individually (spaces between CJK do not merge them into "words")

### Unit: edge cases

- Empty string with `'auto'` => 0
- Whitespace-only string with `'auto'` => 0
- Unrecognized mode `'foo'` falls back to whitespace counting
- Non-ASCII Latin (e.g., accented French/German text) with `'auto'` is NOT counted as CJK -- counted by whitespace like normal
- String with only CJK punctuation (if applicable) -- verify reasonable behavior

### Integration: template rendering

- Template `{{ content | number_of_words: 'auto' }}` parses and renders successfully via `TemplateEngine`
- Template `{{ content | number_of_words }}` still works (regression check)

## Log

### [SWE] 2026-03-20
- TDD Step 1: Wrote 14 new tests covering all modes (auto, cjk, default, unrecognized), CJK character types (Chinese, Hiragana, Katakana, Hangul), mixed content, edge cases (empty, whitespace-only), non-ASCII Latin, and 1 regression test for default mode with CJK. Also added 2 integration tests for template rendering.
- TDD Step 2: Ran tests -- all 14 new tests FAIL with "expected at most 0 positional arguments". 6 existing tests PASS (no regressions).
- TDD Step 3: Implemented fix in src/template/filters/number_of_words.rs:
  - Added `NumberOfWordsArgs` with optional `mode: Option<Expression>` parameter
  - Changed filter to use `FilterParameters` + `FromFilterParameters` pattern (same as truncatewords.rs/sample.rs)
  - Added `is_cjk()` function covering CJK Unified Ideographs, Extension A, Compatibility Ideographs, Hiragana, Katakana, Hangul Syllables, Extension B+
  - Added `count_words_cjk()` function that counts each CJK character as one word and splits non-CJK text on whitespace
  - 'auto' and 'cjk' modes use CJK-aware counting; unrecognized modes fall back to whitespace splitting
- TDD Step 4: Ran tests -- all 20 tests PASS (6 existing + 14 new unit tests)
- Full test suite: all tests pass across all targets (126 total, 0 failures)
- Clippy: pre-existing error in vendored liquid-core crate (mismatched lifetime syntax), not caused by this change
- Formatting: clean after `cargo fmt`
- Files modified: src/template/filters/number_of_words.rs, tests/integration_templates.rs
