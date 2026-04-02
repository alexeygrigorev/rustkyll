# Issue 480: Extend jekyll-archives with date-based archives (year/month/day)

## Problem

The `jekyll-archives` plugin was implemented in issue #258 with support for **tags** and **categories** archive types. However, the date-based archive types (`year`, `month`, `day`) were explicitly descoped. These are needed by sites like al-folio, which configures:

```yaml
jekyll-archives:
  posts:
    enabled: [year, tags, categories]
    permalinks:
      year: "/blog/:year/"
```

Currently, rustkyll silently ignores `year`, `month`, and `day` in the `enabled` list (see `src/archives.rs` line 149). This issue adds support for these date-based archive types.

## Background: How date-based archives work in jekyll-archives

### Archive types

- **`year`**: One page per year that has posts. E.g., if posts exist in 2023 and 2024, generates `/2023/index.html` and `/2024/index.html`.
- **`month`**: One page per year-month combination. E.g., `/2024/01/index.html`, `/2024/02/index.html`.
- **`day`**: One page per year-month-day combination. E.g., `/2024/01/15/index.html`.

### Permalink placeholders

Date archive permalinks support these placeholders:
- `:year` -- 4-digit year (e.g., "2024")
- `:month` -- 2-digit month (e.g., "01")
- `:day` -- 2-digit day (e.g., "15")

Default permalinks if not specified:
- year: `/:year/`
- month: `/:year/:month/`
- day: `/:year/:month/:day/`

### Page context

Each date archive page gets:
- `page.title` -- the date string (e.g., "2024" for year, "2024-01" for month)
- `page.type` -- `"year"`, `"month"`, or `"day"`
- `page.date` -- the date represented (first day of period)
- `page.posts` -- array of post objects within that time period, sorted newest-first

### Layout resolution

The `layouts` map can include `year`, `month`, and `day` keys:
```yaml
jekyll-archives:
  layouts:
    year: year-archive
    month: month-archive
    day: day-archive
```

If no layout is specified for a date type, the singular `layout` key is used as fallback.

## Scope

### In scope

1. Parse `year`, `month`, `day` from the `enabled` array in `jekyll-archives` config
2. Add `ArchiveType::Year`, `ArchiveType::Month`, `ArchiveType::Day` variants
3. Generate one page per unique year/month/day that has posts
4. Support `:year`, `:month`, `:day` permalink placeholders
5. Support layout configuration for each date type
6. Set `page.title`, `page.type`, `page.posts`, `page.url` on generated pages
7. Group posts by their date field (parsed from front matter or filename)

### Out of scope (separate issues)

- Per-collection archive config (v2 format used by al-folio) -- see issue #507
- The `:type` permalink placeholder -- see issue #507
- Multi-collection archive support -- see issue #507

## Dependencies

- Issue #258 (done): base jekyll-archives implementation for tags/categories

## DTC DOM Baseline

- DTC DOM match count: 596/790 (must not drop below this)
- DTC does not use jekyll-archives at all, so this change should have zero impact on DTC output

## Acceptance Criteria

- [ ] `ArchiveType` enum includes `Year`, `Month`, and `Day` variants
- [ ] `ArchivesConfig::from_config` parses `year`, `month`, `day` from the `enabled` array
- [ ] When `enabled` contains `year`, one HTML file is generated per unique year that has posts
- [ ] When `enabled` contains `month`, one HTML file is generated per unique year-month that has posts
- [ ] When `enabled` contains `day`, one HTML file is generated per unique year-month-day that has posts
- [ ] Permalink patterns support `:year`, `:month`, `:day` placeholders
- [ ] Default permalinks are used when not specified: `/:year/`, `/:year/:month/`, `/:year/:month/:day/`
- [ ] Generated pages have `page.title` set appropriately (year string, year-month, or full date)
- [ ] Generated pages have `page.type` set to `"year"`, `"month"`, or `"day"`
- [ ] Generated pages have `page.posts` as an array of post objects within the time period, sorted newest-first
- [ ] Layout configuration works for date types (via `layouts.year`, `layouts.month`, `layouts.day` or fallback to singular `layout`)
- [ ] Posts with no date are excluded from date-based archives
- [ ] Existing tag/category archive functionality is not broken (all existing tests pass)
- [ ] DTC DOM match count does not drop below 596/790
- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `./scripts/cargo-safe test` passes with no regressions
- [ ] 8+ new tests covering date-based archives

## Test Scenarios

### Unit: Config parsing for date types

- Parse config with `enabled: [year]` -- verify `ArchiveType::Year` is in enabled list
- Parse config with `enabled: [year, month, day, tags, categories]` -- verify all five types parsed
- Parse config with `enabled: [tags]` (no date types) -- verify no date types enabled
- Parse config with year/month/day permalink patterns -- verify correct extraction
- Parse config with no date permalinks -- verify defaults used

### Unit: Date extraction from posts

- Extract year "2024" from post with date "2024-03-15" -- verify correct grouping
- Extract month "2024-03" from post with date "2024-03-15"
- Extract day "2024-03-15" from post with date "2024-03-15"
- Post with no date field -- verify excluded from date archives
- Non-ASCII content in post titles within date archives -- verify no encoding issues

### Unit: Permalink resolution for dates

- Permalink `/:year/` with year "2024" produces `/2024/`
- Permalink `/:year/:month/` with year "2024" and month "03" produces `/2024/03/`
- Permalink `/blog/:year/` with year "2024" produces `/blog/2024/`

### Integration: Year archive generation

- Create 3 posts across 2 years, enable year archives -- verify 2 year archive pages generated
- Verify each year page lists only posts from that year
- Verify posts within each year page are in reverse chronological order

### Integration: Month archive generation

- Create 3 posts across 2 months, enable month archives -- verify 2 month archive pages generated
- Verify each month page lists only posts from that month

### Integration: Day archive generation

- Create 2 posts on different days, enable day archives -- verify correct day pages generated

### Integration: Combined date and tag/category

- Enable all types (year, tags, categories) together -- verify both date and tag/category archives generated without interference
- Verify total page count is correct (sum of unique years + unique tags + unique categories)

### Integration: Layout rendering for date archives

- Create a layout for year archives, generate year archive -- verify layout renders with page.title as the year and page.type as "year"

### Regression: Existing functionality

- All existing `archives` module tests continue to pass unchanged

## Log

### [SWE] 2026-04-02

**Fix 1: Add Year/Month/Day to ArchiveType enum and config parsing**
- Wrote tests: test_config_parsing_year_enabled, test_config_parsing_all_five_types, test_config_parsing_tags_only_no_date_types, test_config_parsing_date_permalinks, test_config_parsing_date_permalinks_defaults, test_config_parsing_date_layouts, test_config_parsing_date_layout_fallback_to_singular
- Ran tests: FAILS -- compilation errors (ArchiveType::Year, year_enabled(), year_permalink etc. don't exist)
- Added Year/Month/Day variants to ArchiveType enum, added year/month/day layout and permalink fields to ArchivesConfig, updated parse_enabled() and from_config()
- Ran tests: PASSES

**Fix 2: Add date permalink resolution**
- Wrote tests: test_resolve_date_permalink_year, test_resolve_date_permalink_month, test_resolve_date_permalink_day, test_resolve_date_permalink_blog_prefix
- Ran tests: FAILS -- resolve_date_permalink function doesn't exist
- Implemented resolve_date_permalink() and parse_date_components() helper
- Ran tests: PASSES

**Fix 3: Add date-based archive page generation**
- Wrote tests: test_year_archive_generation, test_year_archive_reverse_chronological, test_month_archive_generation, test_day_archive_generation, test_combined_date_and_tag_archives, test_year_archive_with_layout, test_post_without_date_excluded_from_date_archives, test_date_archive_unicode_post_titles
- Ran tests: FAILS -- generate_single_date_archive_page doesn't exist, ArchivesConfig struct missing new fields in old tests
- Implemented generate_single_date_archive_page(), updated generate_archive_pages() to group posts by year/month/day and generate date archives, updated all existing test configs with new fields
- Ran tests: PASSES (all 37 archive tests pass)

**Summary:**
- Files modified: src/archives.rs
- Tests added: 20 new tests covering date-based archives (config parsing, permalink resolution, year/month/day generation, layout rendering, post exclusion, unicode, combined types)
- All existing 17 archive tests continue to pass unchanged
- Full test suite: 3600+ tests pass, 0 fail
- Clippy: clean (0 warnings)
- Fmt: clean
- DTC DOM: 596/790 matched, 255 total diffs (no regression from baseline of 596/790)
- DTC build time: 0.72s (under 1.0s threshold)

### [QA] 2026-04-02
- Tests: 4005 passed, 0 failed, 2 ignored
- Clippy: clean (no rustkyll warnings)
- Fmt: clean
- DTC DOM: 596/790 matched, no regression from baseline of 596/790
- DTC build time: 0.59s (under 1.0s threshold)
- TDD compliance: PASS -- SWE log shows 3 cycles with tests written first, verified failing, then implemented

Acceptance criteria:
- [x] ArchiveType enum includes Year, Month, Day variants: PASS
- [x] from_config parses year, month, day from enabled array: PASS (test_config_parsing_year_enabled, test_config_parsing_all_five_types)
- [x] Year archives generate one HTML per unique year: PASS (test_year_archive_generation)
- [x] Month archives generate one HTML per unique year-month: PASS (test_month_archive_generation)
- [x] Day archives generate one HTML per unique year-month-day: PASS (test_day_archive_generation)
- [x] Permalink patterns support :year, :month, :day: PASS (4 permalink resolution tests)
- [x] Default permalinks used when not specified: PASS (test_config_parsing_date_permalinks_defaults)
- [x] page.title set appropriately: PASS (test_year_archive_with_layout verifies title="2024")
- [x] page.type set to year/month/day: PASS (test_year_archive_with_layout verifies type="year")
- [x] page.posts sorted newest-first: PASS (test_year_archive_reverse_chronological)
- [x] Layout configuration works with fallback: PASS (test_config_parsing_date_layouts, test_config_parsing_date_layout_fallback_to_singular, test_year_archive_with_layout)
- [x] Posts with no date excluded: PASS (test_post_without_date_excluded_from_date_archives)
- [x] Existing tag/category archives not broken: PASS (all 17 original tests pass)
- [x] DTC DOM not below 596/790: PASS (596/790)
- [x] cargo build: PASS
- [x] clippy: PASS
- [x] fmt: PASS
- [x] tests pass with no regressions: PASS (4005 passed)
- [x] 8+ new tests: PASS (20 new tests)

- VERDICT: PASS

### [PM] 2026-04-02 17:30
- Reviewed diff: 1 file changed (src/archives.rs), ~967 lines added
- Output verification: Verified DTC DOM via `bash scripts/recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io` -- 596/790, no regression
- Code review: Clean implementation. ArchiveType enum extended with Year/Month/Day. Config parsing follows existing tag/category pattern. `parse_date_components` helper is simple and correct. `generate_single_date_archive_page` properly sorts posts newest-first and handles layout rendering with fallback. No site-specific hardcoding.
- Tests: 20 new tests are meaningful -- config parsing (7), permalink resolution (4), generation integration (9) including unicode, no-date exclusion, combined types, layout rendering, and reverse chronological ordering
- Acceptance criteria: all 18 criteria met per QA verification, independently confirmed DTC DOM baseline
- Follow-up issues: none needed (al-folio per-collection config already tracked in #507)
- VERDICT: ACCEPT
