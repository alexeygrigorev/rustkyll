# Issue 267: JSON-LD startDate/endDate format matching

## Problem

On ~193 DTC podcast pages, JSON-LD `startDate` and `endDate` fields differ from Jekyll output. These values come from the DTC site's own `_layouts/podcast.html` template (NOT from rustkyll's built-in SEO tag). The template computes season dates via:

```liquid
{% assign season_dates = season_episodes | map: "date" | compact | sort %}
{% assign season_start = season_dates | first %}
{% assign season_end = season_dates | last %}
```

Then renders them directly: `"startDate": "{{ season_start }}"`.

## Root Cause

There are two distinct bugs:

### Bug 1: Collection item `date` field not expanded in site-level arrays

When building the Liquid context for collection items in `site.podcast` (etc.), the function `collection_item_to_liquid_slim()` in `src/generator.rs` (line ~569) inserts `item.date` as a raw string. For episodes with a bare `YYYY-MM-DD` front matter date, `item.date` is just `"2025-11-07"` because `extract_date()` in `src/collection.rs` returns the raw YAML string without expansion.

The date expansion (`expand_date_only_string_with_tz`) only runs in `yaml_mapping_to_object_with_tz()`, which is used for the `page` context but NOT for the collection item objects in site-level arrays like `site.podcast`.

So `site.podcast | map: "date"` yields `["2025-11-07", ...]` instead of `["2025-11-07 00:00:00 +0100", ...]`.

**Expected (Jekyll):** `"startDate": "2025-11-07 00:00:00 +0100"`
**Actual (rustkyll):** `"startDate": "2025-11-07"`

### Bug 2: Build timestamp uses UTC instead of site/local timezone

`build_timestamp()` in `src/collection.rs` (line 325-328) hardcodes `Utc::now()` with `+0000`. Jekyll uses the build machine's local timezone (or the `timezone` config key if set). When no timezone is configured (as in the DTC site), Jekyll uses the system's local timezone.

This affects `season_end` (which is the build timestamp for episodes without explicit dates) and any `season_start` that comes from a `backfill_default_dates()` call.

**Expected (Jekyll):** `"endDate": "2026-03-20 12:38:05 +0100"`
**Actual (rustkyll):** `"endDate": "2026-03-20 11:57:49 +0000"`

## Fix Locations

### Fix 1: Expand dates in `collection_item_to_liquid_slim()`

In `src/generator.rs`, function `collection_item_to_liquid_slim()` (line ~544), when inserting `item.date` into the Liquid object (line ~569-571), apply `expand_date_only_string_with_tz()` to the date string so that bare `YYYY-MM-DD` dates become `YYYY-MM-DD 00:00:00 +HHMM`.

This requires passing the site timezone into `collection_item_to_liquid_slim()`. The function signature needs to accept `site_tz: Option<chrono_tz::Tz>`.

### Fix 2: Use site timezone in `build_timestamp()`

In `src/collection.rs`, `build_timestamp()` should accept an optional timezone parameter and format the timestamp in that timezone instead of always using UTC. When a site timezone is configured, use it. When not configured, use UTC (matching the current behavior -- the timezone discrepancy with Jekyll's local-tz default is a known limitation and not worth chasing since it depends on the build machine).

Alternatively, if the codebase already has a mechanism for passing the site timezone through the build pipeline, thread it through to `build_timestamp()` and `backfill_default_dates()`.

## Impact

Fixes ~193 of remaining DTC DOM diffs (the second largest bucket after bio wrapping).

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes (including new tests)
- [ ] When a podcast collection item has `date: 2025-11-07` (bare date) in front matter, `site.podcast | map: "date" | first` renders as `2025-11-07 00:00:00 +0000` (or with site timezone offset if configured)
- [ ] `collection_item_to_liquid_slim()` applies date expansion to the `date` field, matching the behavior of `yaml_mapping_to_object_with_tz()` for the `page` context
- [ ] `build_timestamp()` respects the site timezone when one is configured (instead of always using UTC)
- [ ] `backfill_default_dates()` passes through the site timezone to `build_timestamp()` (or accepts an already-formatted timestamp)
- [ ] The `date` field on collection items in site-level arrays (`site.posts`, `site.podcast`, etc.) matches what Jekyll produces for `{{ document.date }}`
- [ ] No regressions in existing date-related tests

## Test Scenarios

### Unit: Date expansion in collection items

- Create a `CollectionItem` with `date: "2025-11-07"` (bare date), convert via `collection_item_to_liquid_slim()`, verify the `date` field in the resulting Liquid object is `"2025-11-07 00:00:00 +0000"`
- Same test with a site timezone (e.g., `Europe/Berlin`), verify the offset matches (e.g., `+0100` in winter)
- Create a `CollectionItem` with `date: "2025-11-07 00:00:00 +0200"` (already expanded), verify it passes through unchanged

### Unit: Build timestamp with timezone

- Call `build_timestamp()` with no timezone, verify format is `YYYY-MM-DD HH:MM:SS +0000`
- Call `build_timestamp()` with `Europe/Berlin` timezone, verify the offset is not `+0000` (it should be `+0100` or `+0200` depending on DST)

### Integration: Template rendering of collection dates

- Set up a minimal site with a podcast collection containing episodes with bare `YYYY-MM-DD` dates
- Render a template that does `{{ site.podcast | map: "date" | first }}`
- Verify the output includes the time component (`00:00:00 +0000`)

### Integration: JSON-LD season dates in podcast layout

- Set up a minimal podcast layout template with `season_start`/`season_end` computation (matching the DTC pattern)
- Render a podcast page, extract the JSON-LD `startDate` field
- Verify it includes the full datetime format (not just `YYYY-MM-DD`)

## Dependencies

None -- this issue is independent and can be worked on immediately.

## Log

### [SWE] 2026-03-20

**TDD Cycle:**

1. Wrote 7 new tests (4 in generator.rs, 3 in collection.rs) before implementation
2. Compiled -- tests fail (expected: function signatures don't accept timezone parameter yet)
3. Implemented fixes:
   - Made `expand_date_only_string_with_tz` pub(crate) in src/template/context.rs
   - Updated `build_timestamp()` in src/collection.rs to accept `Option<chrono_tz::Tz>`
   - Updated `collection_item_to_liquid_slim()` in src/generator.rs to accept `Option<chrono_tz::Tz>` and expand bare dates
   - Threaded timezone through all callers: `build_site_context`, `build_related_posts`, `build_per_post_related_posts_lenient`, `build_categories_and_tags`
   - Updated main.rs to pass site timezone to `build_timestamp()`
4. All 7 new tests pass, 1998 total lib tests pass

**Tests added:**
- `test_slim_bare_date_expanded_no_tz` -- bare YYYY-MM-DD expanded to YYYY-MM-DD 00:00:00 +0000
- `test_slim_bare_date_expanded_with_tz` -- bare date with Europe/Berlin gets +0100
- `test_slim_already_expanded_date_unchanged` -- already-expanded dates pass through
- `test_slim_no_date_field_when_none` -- no date inserted when item.date is None
- `test_build_timestamp_with_no_tz` -- UTC default
- `test_build_timestamp_with_berlin_tz` -- non-zero offset with Europe/Berlin
- `test_build_timestamp_with_utc_tz` -- explicit UTC

**Build:** 1998 lib tests pass, 1 pre-existing failure (test_issue268 from issue 268 WIP, unrelated). Clippy has pre-existing vendor issue. Fmt clean.

**Files modified:**
- src/generator.rs -- updated `collection_item_to_liquid_slim` signature and date expansion, updated all callers, 4 new tests
- src/collection.rs -- updated `build_timestamp` to accept timezone, 3 new tests
- src/main.rs -- thread site timezone to `build_timestamp`
- src/template/context.rs -- made `expand_date_only_string_with_tz` pub(crate)
- docs/tracker/267-dtc-jsonld-date-format.in-progress.md -- this log
