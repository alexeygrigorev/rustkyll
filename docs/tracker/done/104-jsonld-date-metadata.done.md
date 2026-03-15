# Issue 104: Fix JSON-LD date metadata (D14, D15)

Descoped from issue #90. JSON-LD structured data uses different date values than Jekyll:
- D14: datePublished format differs
- D15: dateModified format differs

Low priority — no user-visible impact, only affects search engine metadata.

## Acceptance criteria
- JSON-LD datePublished matches Jekyll format
- JSON-LD dateModified matches Jekyll format
- Structured data validates with Google's testing tool

## Log

### [SWE] 2026-03-15

**Root cause analysis:**
- D14/D15 are about podcast episode JSON-LD (inline in the podcast.html template)
- In Jekyll, every collection item gets a `date` even if not specified in front matter -- it defaults to the build timestamp (`site.time`)
- In rustkyll, `page.date` was only set when explicitly present in front matter or filename
- The podcast template uses `{{ page.date }}` for `dateModified` and `season_dates` (for `startDate`/`endDate`)
- Without `page.date`, the template fell through to `page.dateadded`, producing different values
- The `normalize_date` function in `jsonld.rs` also had a bug: it would double-append timezone for dates with format `"YYYY-MM-DD HH:MM:SS +0000"`

**Fixes applied:**
1. `src/collection.rs`: Added `build_timestamp()` and `backfill_default_dates()` functions that set a default build-time date on collection items without explicit dates, matching Jekyll behavior
2. `src/main.rs`: Call `backfill_default_dates()` after loading all collections, using a single build timestamp for consistency
3. `src/jsonld.rs`: Fixed `normalize_date()` to correctly handle 3-part date strings with timezone suffix (e.g., `"2026-03-15 10:30:00 +0000"` -> `"2026-03-15T10:30:00+00:00"`)

**Tests added:** 8 new tests
- `test_build_timestamp_format` - verifies Jekyll-compatible format
- `test_backfill_default_dates_fills_missing` - verifies dates are filled
- `test_backfill_default_dates_preserves_existing` - verifies existing dates are not overwritten
- `test_backfill_default_dates_mixed_items` - verifies mixed case
- `test_normalize_date_with_time_and_timezone` - build timestamp format
- `test_normalize_date_with_positive_timezone` - positive tz
- `test_normalize_date_with_negative_timezone` - negative tz
- `test_normalize_date_iso_passthrough` - ISO format passthrough
- `test_book_jsonld_date_published_with_timezone` - end-to-end book JSON-LD

**Build:** 1130+ tests pass, 0 fail, clippy clean, fmt clean

**Files modified:**
- `src/collection.rs` - added `build_timestamp()`, `backfill_default_dates()`, 4 tests
- `src/main.rs` - call `backfill_default_dates()` after loading collections
- `src/jsonld.rs` - fixed `normalize_date()` for timezone handling, 5 new tests
