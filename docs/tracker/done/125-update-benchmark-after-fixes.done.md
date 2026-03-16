# Issue 125: Update benchmark results after all fixes

## Problem

docs/benchmark/results.md was last generated on 2026-03-15 12:15 UTC (issue #73). Since then, a large number of DOM-level and rendering fixes have landed: #112 (collection sort stability), #113 (syntect/rouge token mapping), #114 (kramdown bare text wrapping), #116 (books listing timezone regression), #117 (book detail markdownify), #119 (DOM diff audit and fix), #121 (sidebar nav sort order), #122 (data listing order), #124 (kramdown loose list wrapping), #137-#148 (JSON-LD fixes, heading IDs, URL encoding, accordion script placement, inline code classes, ordered list start attributes, target blank, misc markdown edge cases).

These fixes should significantly improve DOM match percentages, reduce visual diffs, and possibly change speed numbers. The current results.md is stale and must be regenerated with real data.

## Dependencies

- Issues #119, #121, #122, #124, #137-#148 must be `.done.md` (they are)
- Issues #120 (fix-theme-sites-comparison) and #123 (fix-google-fonts-css) are still `.todo.md` -- they do NOT block this issue, but the engineer must note in results where those known issues still affect numbers

## Scope

This is a full benchmark rerun: speed, structural comparison, and visual comparison for all sites. No code changes to `src/`. Only documentation and benchmark artifacts are updated.

## Acceptance Criteria

### Speed benchmark

- [ ] Full benchmark run using `scripts/benchmark.sh` across all sites in `websites/`, with at least 3 runs per tool per site, median reported
- [ ] `docs/benchmark/results.md` updated with actual timings from this run
- [ ] The "Generated" timestamp at the top of results.md reflects when this run was performed
- [ ] rustkyll version string in results.md matches the current binary's `--version` output
- [ ] README.md benchmark table updated to match the fresh results (the 5-row summary table in the "Tested sites" section)
- [ ] Any site whose build status changed (FAIL to success or vice versa) is noted in the results

### Structural equivalence (DOM comparison)

- [ ] `scripts/dom_compare.py` (or `dom_compare_full.py`) run on ALL dual-success sites
- [ ] Per-site results documented: file match count/percentage, DOM match count/percentage, Liquid leak count
- [ ] The "Dual-Success Sites -- Consolidated Comparison" table in results.md is fully regenerated with fresh numbers
- [ ] Tier classification (Tier 1/2/3) in the "Structural Equivalence Details" section is updated to reflect new DOM match percentages
- [ ] Any site that improved from Tier 3 to Tier 2, or Tier 2 to Tier 1, is explicitly called out

### Visual comparison (Playwright)

- [ ] `scripts/visual-compare.sh` run on all dual-success sites that have a valid homepage
- [ ] Sites served over HTTP with full CSS/images/fonts/JS (not raw HTML file comparison)
- [ ] At minimum: homepage + one content page per site (where applicable)
- [ ] Per-site visual diff percentages documented in the consolidated table
- [ ] The "Visual Comparison Details" section is fully regenerated: pixel-perfect pages, near-perfect pages, minor diffs, and >5% diffs
- [ ] Every visual diff >1% has a documented root cause explanation
- [ ] Diff images saved under `playwright/screenshots/` organized by site name
- [ ] Sites where visual comparison is skipped are listed with the reason (e.g., "no valid homepage HTML")

### Consolidated results file

- [ ] `docs/benchmark/results.md` contains all three comparison dimensions for every site in one place: speed, structural equivalence, and visual fidelity
- [ ] The "Summary" paragraph at the top is rewritten to accurately reflect the new numbers (number of dual-success sites, speedup range, DTC speedup)
- [ ] The "Compatibility Summary" section is updated (counts of dual-success, rustkyll-only, Jekyll-only, both-fail sites)
- [ ] Column definitions are preserved and accurate

### README updates

- [ ] The "Tested sites" table in README.md has updated numbers matching results.md (pages, Jekyll time, rustkyll time, speedup)
- [ ] The summary line ("34 of 44 sites build with both tools" or whatever the new number is) is updated
- [ ] No stale numbers remain in README.md

### Integrity checks

- [ ] No code changes to `src/` in this issue
- [ ] The engineer must verify that the numbers in results.md match what the benchmark scripts actually produced (not manually edited to look better)
- [ ] If any site regressed (slower speed, worse DOM match, or higher visual diff compared to the previous results.md), the regression is explicitly documented and explained

## Test Scenarios

This issue has no `cargo test` component since it is a documentation/benchmark update. Verification is done by the PM and tester inspecting the outputs.

### Verification: Speed numbers

- Compare the new DTC timing against the previous 1.045s -- it should be in the same ballpark (within 20%) unless hardware changed
- Spot-check 3-5 sites by re-running `scripts/benchmark.sh --site SITE --runs 1` and confirming numbers are consistent with the results file

### Verification: Structural numbers

- Pick 2-3 sites from the consolidated table and manually re-run `scripts/dom_compare.py` on them to confirm the reported DOM match percentages
- Check that no site reports 100% DOM match but also has Liquid leaks (this would be contradictory)

### Verification: Visual numbers

- Pick 2-3 sites and re-run `scripts/visual-compare-site.sh` to confirm the reported pixel diff percentages
- For any site previously showing >1% diff that now shows 0%, verify this is real by inspecting the diff images

### Verification: Consistency

- Every site in the speed table must also appear in the consolidated comparison table (for dual-success sites)
- README numbers must be a strict subset of results.md numbers (no README number that doesn't appear in results.md)
- The "Generated" timestamp must be later than 2026-03-15 12:15 UTC (the previous run)

## Notes

- Issues #120 and #123 are still open and may affect theme site comparison results and Google Fonts-related visual diffs. The engineer should note these as known issues in the results file where relevant, but should NOT block this issue on them.
- The previous benchmark run (issue #73) serves as the baseline for detecting regressions.

## Log

### [SWE] 2026-03-16 11:30
- Ran full speed benchmark (3 runs per tool, 44 sites, median times)
- Built all 34 dual-success sites with both Jekyll and rustkyll
- Ran DOM comparison (dom_compare.py) on all 34 dual-success sites
- Ran visual comparison (Playwright/Chromium) on 21 sites with valid homepages
- Updated docs/benchmark/results.md with all three comparison dimensions
- Updated README.md benchmark table with fresh numbers
- No src/ changes made (benchmark/documentation only)
- All 1383 existing tests pass, clippy clean, fmt clean

Key improvements found:
- DTC DOM matches: 2/787 (0%) -> 428/787 (54%) -- massive improvement
- DTC visual: all 22 pages now pixel-perfect (0.00%), including course-ml-zoomcamp (was 4.11%) and tools (was 1.27%)
- mojombo-blog: DOM matches 8/17 -> 10/17, all posts now 0.00% visual (was 1.5-3.5%)
- large-docs-site: visual 9.62% -> 0.00% (sidebar sort order fixed)
- mlbookcamp-page: DOM matches 1/15 -> 4/15
- mlwiki.org: DOM matches 190/639 -> 205/639
- No regressions in DOM or visual

Speed notes: Some timing variations (hardware load), e.g., kids-horror-stories-ru slower (0.33s -> 0.57s) but Jekyll also slower on same run. No build status changes.

Known issues documented: #120 (theme sites DOM comparison) and #123 (Google Fonts CSS)

Files modified:
- docs/benchmark/results.md (full rewrite with fresh data)
- README.md (updated benchmark table)
- docs/tracker/125-update-benchmark-after-fixes.in-progress.md (this file)
