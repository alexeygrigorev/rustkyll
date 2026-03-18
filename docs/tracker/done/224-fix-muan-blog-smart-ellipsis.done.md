# Issue 224: Fix muan-blog smart ellipsis

## Problem

~10 muan-blog pages show `...` (three dots U+002E) vs `…` (U+2026 HORIZONTAL ELLIPSIS). pulldown-cmark's smart punctuation converts three dots to the ellipsis character, but Jekyll's CommonMark doesn't perform this conversion by default.

Related to issue 220 (smart quotes) -- both stem from pulldown-cmark's smart punctuation feature.

## Scope

1. Verify that disabling smart punctuation (issue 220) also fixes ellipsis conversion
2. If not, identify and disable ellipsis-specific smart punctuation separately
3. Verify affected muan-blog pages match Jekyll output

## Acceptance Criteria

- [ ] Three dots `...` in markdown source remain as three dots in HTML output
- [ ] ~10 affected muan-blog pages match Jekyll output
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all new and existing tests
- [ ] Tests include content with three-dot ellipsis patterns

## Dependencies

- Issue 220 (smart quotes) -- may be resolved by the same fix

## Log

- 2026-03-18: Created from muan-blog comparison analysis.
- 2026-03-18: Resolved by issue 220. The fix to conditionally disable `ENABLE_SMART_PUNCTUATION` for non-kramdown processors fixes both smart quotes and smart ellipsis, since pulldown-cmark's smart punctuation option controls both. Confirmed by unit test `test_issue220_smart_punctuation_off_preserves_three_dots` which asserts that `...` (three U+002E) is preserved when smart punctuation is off.
