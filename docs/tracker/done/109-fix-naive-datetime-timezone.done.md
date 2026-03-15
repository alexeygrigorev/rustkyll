# Issue 109: Fix NaiveDateTime timezone handling for dates without timezone

## Problem

YAML dates without timezone (e.g. `2020-12-18 23:59:59`) are treated as NaiveDateTime by rustkyll, showing "18 Dec 2020". Jekyll treats them as UTC then converts to local time, showing "19 Dec 2020" in CET.

This causes a 51-pixel diff on /people/alexeygrigorev.html and likely affects other pages with timezone-edge dates.

## Acceptance criteria

- Dates without timezone produce same output as Jekyll
- /people/alexeygrigorev.html achieves 0 pixel diff
- No regressions on other date formatting

## Log

### [SWE] 2026-03-15 12:00
- Root cause: `parse_date_string()` returned NaiveDateTime as-is for dates without timezone (e.g. `2020-12-18 23:59:59`). Jekyll treats these as UTC and converts to the site's configured timezone before display.
- Fix: Added `chrono-tz` dependency for IANA timezone support.
- Added `parse_date_string_with_tz(s, site_tz)` that converts naive datetimes from UTC to the site timezone.
- Added `get_site_timezone(runtime)` helper that reads `site.timezone` from the Liquid runtime context (sourced from `_config.yml`'s `timezone` key).
- Updated all 5 date filter files (date.rs, date_to_string.rs, date_to_long_string.rs, date_to_rfc822.rs, date_to_xmlschema.rs) to use `parse_date_string_with_tz` with the site timezone from the runtime.
- Dates WITH explicit timezone (e.g. `+0200`) still use `naive_local()` and are NOT affected by site timezone (preserves D10 behavior).
- When no `timezone` is configured, dates are kept as-is (UTC) for reproducible builds.
- To match Jekyll's CET output, users should add `timezone: Europe/Berlin` to `_config.yml`.
- Tests added: 8 new tests for timezone resolution and conversion in mod.rs.
- Build: 1276 tests pass, 0 fail, clippy clean, fmt clean.
- Files modified: Cargo.toml, src/template/filters/mod.rs, src/template/filters/date.rs, src/template/filters/date_to_string.rs, src/template/filters/date_to_long_string.rs, src/template/filters/date_to_rfc822.rs, src/template/filters/date_to_xmlschema.rs
