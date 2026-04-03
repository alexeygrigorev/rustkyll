# Issue 518: Remove backfilled dates from non-post collection items in iteration context

## Problem

Rustkyll unconditionally includes backfilled dates (set to `build_time`) in the
Liquid representation of non-post collection items. This causes templates that
check `{% if post.date %}` to show dates where Jekyll does not.

### Root cause (corrected from original filing)

The original filing (and issue #474 analysis) claimed that Jekyll's
`DocumentDrop#date` exposes `site.time` for ALL documents in iteration context.
Extensive instrumentation of Jekyll 3.10.0 proves this is WRONG for the
academicpages case:

In Jekyll, `data["date"]` for non-post items starts as `nil`. It only gets set
to `site.time` as a **side effect** of layout rendering, specifically via
`Renderer#place_in_layouts` -> `add_regenerator_dependencies` ->
`document.write?` -> `Document#date` (which does `data["date"] ||= site.time`).

This means the date's visibility depends on **rendering order**:
- When a template iterates `site.portfolio` during `render_pages`, the portfolio
  documents may or may not have had their dates set depending on whether they
  were already processed by `render_docs`.
- For academicpages, the Jekyll cached output shows portfolio items WITHOUT dates
  in the listing context (confirmed with fresh Jekyll 3.10.0 build).

However, for DTC podcast items, the same `map: "date"` filter DOES return build
times. This is because DTC podcast documents are rendered during `render_docs`,
and by the time their layouts run, the `document.date` side effect fires, setting
`data["date"]`. The podcast template then accesses `site.podcast` in the layout
context (not the content body), where dates from already-rendered documents are
visible.

### Key finding: the behavior is non-deterministic in Jekyll

Jekyll's date availability for non-post items depends on rendering order, which
itself depends on filesystem ordering and collection processing order. Different
builds may produce different results.

### Visible symptoms (academicpages, 1 page, 11 diffs)

**portfolio/index.html (11 diffs)**: Portfolio items show BOTH date AND excerpt
in rustkyll, but ONLY excerpt in Jekyll. The `{% elsif post.date %}` branch in
`archive-single.html` triggers because `post.date` is truthy (backfilled), but
in Jekyll it is nil (not yet backfilled at render time).

### What changed since original filing

The original filing expected 7 pages / ~155 diffs. Current DOM comparison shows:
- portfolio/index.html: 11 diffs (date issue)
- portfolio-1/index.html: 1 diff (build timestamp only)
- portfolio-2/index.html: 1 diff (build timestamp only)
- collection-archive/index.html: 18 diffs (missing `site.collections` -- separate issue)
- sitemap/index.html: 57 diffs (multiple issues, mostly ordering/collection)

The portfolio-1 and portfolio-2 pages are now essentially matching (only build
timestamp diffs). The `article:published_time` meta tag is NOT present in
Jekyll's output either, so that acceptance criterion from the original filing
was incorrect.

### Constraint: DTC must not regress

DTC podcast pages depend on backfilled dates being available via `map: "date"`
for computing `season_start`/`season_end`. Removing backfilled dates from the
Liquid context would regress ~197 DTC podcast pages (currently 596/790 DOM match).

## Proposed fix

The safest fix that matches Jekyll's behavior for academicpages without
regressing DTC:

**Do not include `date` in `collection_item_to_liquid_slim` (used for
`site.<collection>` iteration arrays) when the item is a non-post collection
item AND the date was backfilled (not from front matter or filename).**

Specifically, in `generator.rs`:
- `collection_item_to_liquid_slim`: only include `date` if
  `item.front_matter.contains_key("date")` OR `item.collection_name == "posts"`
- `collection_item_to_liquid_full`: same guard (used for `collection.docs`)
- `collection_item_to_yaml_value`: same guard

The `item.date` field (used internally for sorting, `map: "date"` on
site-level arrays, etc.) continues to be backfilled for all items.

**IMPORTANT RISK**: This will affect DTC podcast `season_dates` computation.
For seasons where NO episode has an explicit `date:` field (seasons 1, 2, 11,
15, 18, 19, 21, 23), `season_start` and `season_end` would become empty
instead of the build time. This would regress ~80 DTC podcast pages.

### Alternative: Add date to liquid only for posts AND items with explicit dates

If the DTC regression is unacceptable, the fix should be limited to:
1. Only fix the `page_fm` context (already done by #474)
2. Accept the 11 diffs on portfolio/index.html as a known limitation
3. Create a follow-up issue to properly implement rendering-order-dependent
   date exposure

### Recommended path: Investigate DTC impact first

Before implementing, the SWE should:
1. Build DTC with the proposed fix
2. Run DOM comparison
3. If DTC regresses, use the alternative approach (accept the 11 diffs)
4. If DTC does not regress (because the affected diffs are already filtered
   as acceptable), proceed with the fix

## Affected Sites

- academicpages (primary, 1 page affected: portfolio/index.html, 11 diffs)
- DTC (risk of regression on ~80 podcast pages -- must verify)

## Dependencies

- Issue #474 (done) -- established the posts-only date backfill for page context

## DTC DOM Baseline

- Current: 596/790
- Must not drop below: 596/790
- Academicpages baseline: 10/45

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes clean
- [ ] `cargo fmt` produces no changes
- [ ] `cargo test` passes (all existing tests, plus new tests)
- [ ] DTC DOM match count must not drop below 596/790
- [ ] DTC total diff count must not increase above 255
- [ ] Academicpages DOM match count must not decrease below 10/45
- [ ] If the fix is applied: academicpages `portfolio/index.html` no longer
      shows `<p class="page__date">` for portfolio items (matches Jekyll
      behavior -- excerpt only)
- [ ] If DTC regression makes the fix unsafe: issue is closed as wontfix or
      descoped to a follow-up with explicit rationale documented in the log
- [ ] The existing #474 test `test_backfill_non_post_sets_item_date_but_not_frontmatter`
      continues to pass
- [ ] The DTC podcast `season_end` computation still produces correct output
      (verified by building DTC and checking a season-1 podcast page)

## Test Scenarios

### Unit: Date visibility in liquid context for non-post items

- Create a portfolio-type collection item with NO date in front matter
- Verify `collection_item_to_liquid_slim` does NOT include `date` field
- Create a portfolio-type item WITH explicit date in front matter
- Verify `collection_item_to_liquid_slim` DOES include `date` field
- Verify posts always include `date` in liquid context (backfilled or explicit)

### Unit: Regression - item.date still backfilled for sorting

- Create a non-post item with no date
- After `backfill_default_dates`, verify `item.date` is set (for internal use)
- Verify `item.front_matter` does NOT contain `date` (for non-posts)

### Integration: Academicpages portfolio listing

- Build academicpages
- Check `portfolio/index.html` does NOT contain `<p class="page__date">`
- Check `portfolio/index.html` DOES contain `<p class="archive__item-excerpt">`

### Integration: DTC podcast season dates

- Build DTC
- Check a season-1 podcast page (e.g., `building-data-team.html`)
- Verify `startDate` and `endDate` are present in JSON-LD output
- If they become empty, the fix must be reverted for the liquid context

### Integration: DOM comparison

- Run DOM comparison on academicpages, verify 10+ of 45 pages match
- Run DOM comparison on DTC, verify 596+ of 790 pages match

## Log

### [PM] 2026-04-02 16:20
- Investigated issue thoroughly with fresh Jekyll 3.10.0 builds
- Original analysis was incorrect: Jekyll's date behavior for non-post items
  is a side effect of rendering order, not a deliberate nil-vs-truthy distinction
- Verified academicpages baseline: 10/45 (not 38/45 as original filing claimed)
- Verified DTC baseline: 596/790
- Identified risk: naive fix would regress DTC podcast pages
- Corrected acceptance criteria to reflect actual behavior and constraints
- Groomed with conditional fix approach: implement if DTC-safe, otherwise descope
