# Issue 506: Fix collection output permalink to match Jekyll pretty URL behavior for al-folio

## Problem

al-folio collection items (projects, teachings, news, books) are output with `.html` extensions in rustkyll but Jekyll generates pretty URLs with `/index.html` inside directories.

Examples of the mismatch:
- Jekyll: `projects/1_project/index.html` -- rustkyll: `projects/1_project.html`
- Jekyll: `teachings/data-science-fundamentals/index.html` -- rustkyll: `teachings/data-science-fundamentals.html`
- Jekyll: `news/announcement_1/index.html` -- rustkyll: `news/announcement_1.html`
- Jekyll: `books/the_godfather/index.html` -- rustkyll: `books/the_godfather.html`

This causes 15 pages to appear as "only in Jekyll" in the DOM comparison because the paths do not match.

## Root Cause

Jekyll's default collection permalink format uses pretty URLs (`/:collection/:name/`) when the collection has `output: true`. The al-folio config does not set explicit collection permalinks but relies on Jekyll's default behavior. rustkyll appears to use `/:collection/:name:output_ext` instead, producing `.html` files rather than `name/index.html` directories.

## Scope

1. Ensure rustkyll's default collection output permalink matches Jekyll's default (`/:collection/:name/` producing `name/index.html`).
2. Verify the fix for al-folio's projects, teachings, news, and books collections.
3. Ensure no regression on sites that set explicit collection permalinks.

## Baseline

- al-folio DOM: 3/45 (common files), 60/108 (file coverage)
- DTC DOM baseline: 790/790

## Acceptance Criteria

- [ ] Collection items without explicit permalink settings are output as `name/index.html` (pretty URLs), matching Jekyll's default behavior.
- [ ] al-folio projects (9 pages), teachings (2 pages), news (3 pages), and books (1 page) are generated at the correct paths.
- [ ] The al-folio common file count increases (currently 45, should increase to at least 60).
- [ ] DTC DOM match count does not drop below 790/790.
- [ ] `cargo build` compiles without errors; `cargo clippy` clean; `cargo fmt` clean.

## Test Scenarios

### Unit: collection permalink generation
- Create a collection item with `output: true` and no explicit permalink, verify the output path is `collection_name/item_name/index.html`.
- Create a collection item with an explicit `permalink: /custom/:name.html`, verify it respects the explicit setting.
- Verify that posts (which have their own permalink config) are unaffected.

### Integration: al-folio collection output
- Build `websites/al-folio/` and verify `projects/1_project/index.html` exists (not `projects/1_project.html`).
- Verify `teachings/introduction-to-machine-learning/index.html` exists.
- Run DOM comparison and confirm the common file count increases.

## Dependencies

- Issue #235 (al-folio site is set up)
