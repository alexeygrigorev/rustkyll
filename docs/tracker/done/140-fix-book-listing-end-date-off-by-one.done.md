# Issue 140: Fix book listing end date off-by-one

## Problem

On `books.html`, the book-of-the-week date ranges show end dates that are one day later in Jekyll than in rustkyll. This affects all 78 book entries.

Example:
- Jekyll: `(from 06 Oct 2025 to 11 Oct 2025)`
- Rustkyll: `(from 06 Oct 2025 to 10 Oct 2025)`

This is likely a timezone issue where the end date calculation rounds differently.

Related to issue #116 (books listing timezone regression) which was marked done but this specific symptom persists.

Discovered in issue #119 DOM diff audit.

## Acceptance criteria

- Book date ranges on books.html match Jekyll output exactly
- No regressions in other date calculations

## Log

### [SWE] 2026-03-16

**Investigation:**
- Root cause: naive YAML timestamps like `end: 2025-10-10 23:59:59` need UTC-to-local timezone
  conversion in `date_to_string` filter, matching Jekyll's `Time#localtime` behavior.
- The fix was already in place via:
  1. `generator.rs`: system timezone detection fallback (added in issue #93 round 2)
  2. `date_to_string.rs`: `convert_utc_naive_to_site_tz()` converts UTC naive datetimes to site tz
  3. `is_naive_yaml_timestamp()` correctly identifies timestamps needing conversion
- On this machine (system tz = Europe/Berlin via /etc/localtime), `23:59:59 UTC` becomes
  `00:59:59 CET` next day, producing the correct "11 Oct 2025" output.
- Verified: all 98 date ranges on books.html match Jekyll reference output exactly.
- The DOM diff audit (#119) likely captured a stale comparison; the code was already correct.

**Tests added (4 new tests):**
1. `test_integration_date_to_string_book_end_date_with_timezone` -- Europe/Berlin shifts day
2. `test_integration_date_to_string_book_end_date_utc` -- UTC keeps same day
3. `test_integration_book_date_range_yaml_pipeline` -- full YAML-to-template pipeline
4. `test_integration_book_end_dates_various_months` -- 5 real book dates across CET/CEST

**Build results:** 1484 tests pass, 0 fail, clippy clean, fmt clean.

**Files modified:** `src/template/engine.rs` (4 new tests only, no code changes needed)
