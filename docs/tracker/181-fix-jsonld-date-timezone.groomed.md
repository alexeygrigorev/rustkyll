# Issue 181: Fix JSON-LD datePublished/startDate/endDate timezone

## Checklist Category

**JSON-LD datePublished timezone offset** -- 34 pages. This issue covers the full category.

## Problem

Jekyll's jekyll-seo-tag uses the site's configured timezone offset in JSON-LD date fields. rustkyll always uses UTC (+00:00).

Sample diff:
```
jsonld.@graph[0].datePublished: jsonld_value_differs
  expected: '2023-12-11T00:00:00+01:00'
  actual:   '2023-12-11T00:00:00+00:00'
```

This affects ~34 blog pages with datePublished diffs and ~193 event pages (startDate/endDate) on DTC.

## Goal

Use the site's configured timezone (from `_config.yml` `timezone` field) when formatting dates in JSON-LD output, matching Jekyll's jekyll-seo-tag behavior.

## Affected Sites

- DataTalksClub/datatalksclub.github.io: ~287 pages affected (blog datePublished + event startDate/endDate)

## Dependencies

None.

## Approach (TDD)

1. Write a test that configures a site with `timezone: Europe/Berlin`, renders a page with date `2023-12-11`, and asserts JSON-LD `datePublished` is `2023-12-11T00:00:00+01:00` (CET offset)
2. Verify the test fails (currently produces `+00:00`)
3. Fix the SEO tag / JSON-LD implementation to use site timezone
4. Verify the test passes
5. Run `./scripts/recount-all-dom.sh --site DataTalksClub-datatalksclub.github.io` to confirm improvement

## Acceptance Criteria

- [ ] JSON-LD `datePublished` includes the correct timezone offset from `_config.yml` `timezone` setting (e.g., `+01:00` for `Europe/Berlin` in winter, `+02:00` in summer)
- [ ] JSON-LD `startDate` and `endDate` (for events) also use the correct timezone offset
- [ ] When no timezone is configured, dates default to UTC (`+00:00`) -- current behavior preserved
- [ ] DST transitions are handled correctly (summer dates get summer offset, winter dates get winter offset)
- [ ] `cargo test` passes
- [ ] DTC DOM match count improves

## Test Scenarios

### Unit: Timezone-aware date formatting (write FIRST, must fail before fix)

- **Test `test_jsonld_date_with_site_timezone`**: Create a site context with `timezone: "Europe/Berlin"`. Format date `2023-12-11` for JSON-LD. Assert result is `2023-12-11T00:00:00+01:00` (not `+00:00`).
- **Test `test_jsonld_date_summer_dst`**: Same site, date `2023-07-15`. Assert result is `2023-07-15T00:00:00+02:00` (CEST).
- **Test `test_jsonld_date_no_timezone_defaults_utc`**: Site with no `timezone` key. Assert date formats with `+00:00`.
- **Test `test_jsonld_event_start_end_dates`**: Render an event page with `startDate: 2024-01-15` and `endDate: 2024-01-19` in a `Europe/Berlin` timezone site. Assert both dates use `+01:00`.

### Regression: Non-timezone behavior preserved

- **Test `test_jsonld_date_with_explicit_time`**: Date string `2023-12-11T14:30:00` should still format correctly with timezone applied.
- **Test `test_non_jsonld_date_filters_unaffected`**: The `| date` Liquid filter should continue working as before (this change is JSON-LD specific).

### Integration: Full site verification

- Build DTC site and verify JSON-LD dates in blog pages and event pages include the correct timezone offset.
