# Issue 116: Fix books listing timezone regression

books.html regressed to 0.38% pixel diff (27044 pixels) after timezone fix. The system timezone conversion changed book dates causing visual differences.

## Acceptance criteria
- books.html achieves 0% pixel diff
- No regressions on other pages

## Log

### [SWE] 2026-03-16

- Investigated: Timezone is NOT the root cause. Dates are identical between rustkyll and Jekyll regardless of system timezone. The `parse_date_string_with_tz` function ignores `_site_tz` for naive datetimes, matching Jekyll behavior.
- Root cause: Smart punctuation (pulldown-cmark ENABLE_SMART_PUNCTUATION) converted quotes inside kramdown IALs like `{:target="_blank"}` to curly quotes (U+201C, U+201D). The `parse_ial_attributes` function then didn't recognize curly quotes as string delimiters, producing `target="\u{201c}_blank\u{201d}"` instead of `target="_blank"`.
- Fix 1: Extended `protect_liquid_quotes` in frontmatter.rs to also protect kramdown IAL text (`{:...}`) from smart punctuation, same as Liquid tags.
- Fix 2: Added `html_unescape` call in `parse_ial_attributes` (kramdown.rs) to handle HTML-encoded quotes (`&quot;`) as a defense-in-depth measure.
- Fix 3: Corrected stale test `test_naive_datetime_converted_to_cet` that expected timezone conversion behavior that was intentionally removed.
- Tests added: 5 new tests (2 in kramdown.rs for HTML-encoded IAL parsing, 1 postprocess integration test, 2 in frontmatter.rs for IAL quote protection)
- Build: 1199 tests pass, 0 fail, clippy clean, fmt clean
- Verified: All 4 `target="_blank"` attributes in books.html now have straight quotes
- Files modified: src/kramdown.rs, src/frontmatter.rs, src/template/filters/mod.rs
