# Issue 551: DTC main -- JSON-LD podcast season endDate/startDate fallback

## Problem

The DTC main site (DataTalksClub/datatalksclub.github.io) has 194 pages with JSON-LD differences, totaling 255 diffs (133 `jsonld_value_differs` + 122 `jsonld_missing_field`). All diffs are in the `PodcastSeason` JSON-LD `@graph` entry on podcast episode pages.

Two patterns:

1. **jsonld_value_differs (133 diffs)**: Rustkyll sets `endDate` to the episode's own date, but Jekyll uses `site.time` (the build timestamp) as the `endDate` for the PodcastSeason. The `startDate` matches between both.

2. **jsonld_missing_field (122 diffs)**: Rustkyll omits `startDate` and `endDate` entirely for podcast episodes that lack explicit date frontmatter. Jekyll falls back to `site.time` for both fields.

Example (page with value_differs):
- Jekyll PodcastSeason: `"startDate": "2025-11-07 00:00:00 +0100", "endDate": "2026-03-29 11:31:35 +0200"`
- Rustkyll PodcastSeason: `"startDate": "2025-11-07 00:00:00 +0100", "endDate": "2025-11-07 00:00:00 +0100"`

Example (page with missing_field):
- Jekyll PodcastSeason: `"startDate": "2026-03-29 11:31:35 +0200", "endDate": "2026-03-29 11:31:35 +0200"`
- Rustkyll PodcastSeason: no startDate or endDate fields

The root cause is in how rustkyll generates JSON-LD for PodcastSeason objects. The DTC site's Liquid template uses `site.time` as a fallback, and rustkyll needs to replicate this behavior.

## Scope

Fix the JSON-LD generation for PodcastSeason entries so that:
1. `endDate` falls back to `site.time` when not explicitly set (matching Jekyll behavior)
2. `startDate` and `endDate` are always populated using `site.time` as fallback when episode lacks explicit dates

This is a Liquid template rendering issue -- the DTC templates reference `site.time` in the JSON-LD template, and rustkyll needs to provide `site.time` correctly during rendering.

## Dependencies

None.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests still passing
- [ ] New test: JSON-LD PodcastSeason with explicit episode date has `endDate` set to `site.time`, not episode date
- [ ] New test: JSON-LD PodcastSeason without explicit date has both `startDate` and `endDate` set to `site.time`
- [ ] DTC main DOM comparison improves from 596/790 toward 790/790 (all 255 JSON-LD diffs resolved)
- [ ] DTC main DOM match count must not drop below 596/790
- [ ] No other site regresses in DOM match count

## Investigation Needed

Before implementing, the engineer should:
1. Find the DTC Liquid template that generates the PodcastSeason JSON-LD
2. Identify how `site.time` is referenced in the template
3. Check if rustkyll's `site.time` value matches Jekyll's behavior
4. Determine if this is a Liquid variable resolution issue or a JSON-LD generator issue

## Test Scenarios

### Unit: site.time in JSON-LD context
- Render a Liquid template containing `{{ site.time }}`, verify it outputs the current build time
- Render JSON-LD template with PodcastSeason that uses `site.time` as endDate fallback

### Integration: DTC main site
- Build DTC main site with rustkyll
- Run DOM comparison, verify JSON-LD diffs are resolved
- Compare specific podcast page JSON-LD output between Jekyll and rustkyll

## Output Verification

- Build DTC: `./target/release/rustkyll build --source websites/DataTalksClub/datatalksclub.github.io --destination /tmp/dtc_test`
- Extract JSON-LD from a podcast page and verify PodcastSeason has `endDate` matching build time
- Run DOM comparison: `uv run scripts/dom_compare.py --jekyll-dir websites/DataTalksClub/datatalksclub.github.io/_site_jekyll_cached --rustkyll-dir /tmp/dtc_test`

## DOM Baseline

- DTC main: 596/790 (target: ~790/790)
- DTC docs: 38/57 (must not regress)

## Note on Timestamp Comparison

Since `site.time` is the build timestamp, the DOM comparison will always show a diff between cached Jekyll output and fresh rustkyll output. The DOM comparison script should ideally treat timestamp-only differences as acceptable. However, for this issue, the goal is to ensure rustkyll generates the same _structure_ (fields present, same fallback logic) even if the exact timestamp values differ. The comparison script may need an update to handle this, or the Jekyll cache should be refreshed.

## Log

### [PM] 2026-04-02 grooming
- Identified root cause: PodcastSeason JSON-LD missing site.time fallback for endDate/startDate
- 133 value_differs (endDate uses episode date instead of site.time)
- 122 missing_field (startDate/endDate omitted entirely when episode lacks dates)
- 194 podcast pages affected out of 790 total
- Fixing this would push DTC from 75% to near 100%
