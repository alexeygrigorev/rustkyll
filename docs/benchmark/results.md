# Benchmark: rustkyll vs Jekyll

Generated: 2026-03-14 22:26 UTC

Configuration: 3 runs per tool, median wall-clock time reported.
Timeout: 120s per build.

rustkyll version: rustkyll 0.1.4
Jekyll version: jekyll 4.4.1

## Summary

rustkyll is faster than Jekyll on every site where both tools succeed. Speedups range from 2x (mlwiki.org) to 142x (academicpages). For the primary target site (DataTalksClub/datatalksclub.github.io), rustkyll builds in 1.9s vs Jekyll's 19.1s -- a 10x speedup.

16 of 32 sites build successfully with both tools (beautiful-jekyll and jekyll-docs/docs restored after fixing DateFilter panic in issue #86; homebrew-site Jekyll failure is environmental -- Ruby version mismatch, not a rustkyll issue). Structural equivalence varies widely: sites with simple layouts (kids-horror-stories-ru, alexeygrigorev.github.io) have near-perfect match, while sites with complex theme layouts (so-simple-theme, documentation-theme-jekyll) show significant structural differences due to missing Liquid features. Visual fidelity is excellent for well-supported sites (0% pixel diff for courses, people pages) and degrades proportionally with structural gaps.

## All Sites -- Speed Benchmark

| Site | Pages | Jekyll (s) | rustkyll (s) | Speedup |
|------|-------|------------|-------------|---------|
| alexeygrigorev/aihero | 2 | FAIL | 0.021 | N/A |
| alexeygrigorev/alexeygrigorev.github.io | 8 | 0.568 | 0.014 | 40.57x |
| alexeygrigorev/data-science-interviews | 0 | FAIL | 0.008 | N/A |
| alexeygrigorev/kids-horror-stories-ru | 1345 | 3.787 | 0.503 | 7.52x |
| alexeygrigorev/little-book-of-metals-ru | 43 | 2.276 | 0.389 | 5.85x |
| alexeygrigorev/mlbookcamp-page | 15 | FAIL | 0.065 | N/A |
| alexeygrigorev/mlwiki.org | 640 | 0.973 | 0.489 | 1.98x |
| alexeygrigorev/snippets | 25 | 0.639 | 0.013 | 49.15x |
| DataTalksClub/courses | 5 | FAIL | 0.031 | N/A |
| DataTalksClub/datatalksclub.github.io | 787 | 19.145 | 1.877 | 10.19x |
| DataTalksClub/docs | 57 | 1.788 | 0.033 | 54.18x |
| academicpages | 17 | 4.414 | 0.031 | 142.38x |
| beautiful-jekyll | 9 | 0.806 | 0.020 | 40.30x |
| bitcoin-org | N/A | FAIL | FAIL | N/A |
| choosealicense.com | 72 | FAIL | 0.039 | N/A |
| documentation-theme-jekyll | 100 | 3.600 | 0.072 | 50.00x |
| edition-template | N/A | FAIL | FAIL | N/A |
| government-github | 21 | FAIL | 5.106 | N/A |
| homebrew-site | 134 | FAIL | 0.049 | N/A |
| hyde | 6 | FAIL | 0.010 | N/A |
| jekyll-docs/docs | 132 | 2.974 | 1.070 | 2.78x |
| large-blog-3000 | 3001 | 4.303 | 1.430 | 3.00x |
| large-docs-site | 801 | 23.366 | 0.282 | 82.85x |
| made-mistakes-jekyll | 2 | FAIL | 0.010 | N/A |
| minima | 9 | FAIL | 0.011 | N/A |
| minimal-mistakes | 2 | 0.922 | 0.055 | 16.76x |
| muan-blog | 2218 | 15.869 | 0.364 | 43.59x |
| opensource-guide | 390 | FAIL | 1.178 | N/A |
| programming-historian | N/A | FAIL | FAIL | N/A |
| so-simple-theme | 11 | 1.476 | 0.025 | 59.04x |
| uswds-site | N/A | FAIL | FAIL | N/A |
| wtf-html-css | 1 | FAIL | 0.013 | N/A |

## Dual-Success Sites -- Consolidated Comparison

For every site where both Jekyll and rustkyll succeed, the table below shows speed, structural equivalence, and visual fidelity in one view.

| Site | Pages | Jekyll (s) | rustkyll (s) | Speedup | File Match | Struct Diffs | Liquid Leaks | Visual Diff |
|------|-------|------------|-------------|---------|------------|-------------|-------------|-------------|
| alexeygrigorev/alexeygrigorev.github.io | 8 | 0.568 | 0.014 | 40.57x | 8/8 (100%) | 0/8 (0%) | 0 | 1.69% |
| alexeygrigorev/kids-horror-stories-ru | 1345 | 3.787 | 0.503 | 7.52x | 1345/1345 (100%) | 0/51 (0%) | 0 | 0.85% avg |
| alexeygrigorev/little-book-of-metals-ru | 43 | 2.276 | 0.389 | 5.85x | 43/48 (90%) | 43/43 (100%) | 0 | 0.00% |
| alexeygrigorev/mlwiki.org | 640 | 0.973 | 0.489 | 1.98x | 640/639 (100%) | 49/51 (96%) | 5 | 0.00% |
| alexeygrigorev/snippets | 25 | 0.639 | 0.013 | 49.15x | 25/25 (100%) | 24/25 (96%) | 0 | 0.04% |
| DataTalksClub/datatalksclub.github.io | 787 | 19.145 | 1.877 | 10.19x | 787/787 (100%) | 13/51 (25%) | 0 | 1.29% avg |
| DataTalksClub/docs | 57 | 1.788 | 0.033 | 54.18x | 57/57 (100%) | 50/51 (98%) | 33 | SKIP |
| academicpages | 17 | 4.414 | 0.031 | 142.38x | 17/45 (38%) | 16/17 (94%) | 0 | 3.18% |
| documentation-theme-jekyll | 100 | 3.600 | 0.072 | 50.00x | 100/100 (100%) | 50/51 (98%) | 90 | SKIP |
| large-blog-3000 | 3001 | 4.303 | 1.430 | 3.00x | 3001/3001 (100%) | 50/51 (98%) | 0 | 0.10% |
| large-docs-site | 801 | 23.366 | 0.282 | 82.85x | 801/801 (100%) | 50/51 (98%) | 0 | 9.62% |
| minimal-mistakes | 2 | 0.922 | 0.055 | 16.76x | 2/1 (200%) | 1/1 (100%) | 0 | SKIP |
| muan-blog | 2218 | 15.869 | 0.364 | 43.59x | 2218/2218 (100%) | 25/51 (49%) | 22 | SKIP |
| so-simple-theme | 11 | 1.476 | 0.025 | 59.04x | 11/66 (17%) | 11/11 (100%) | 1 | SKIP |

**Column definitions:**
- **File Match**: rustkyll HTML files / Jekyll HTML files. 100% means identical file tree.
- **Struct Diffs**: files with structural differences / files sampled (up to 51). Lower is better.
- **Liquid Leaks**: count of rustkyll HTML files containing raw `{%` or `{{` tags.
- **Visual Diff**: pixel difference percentage for homepage (or average across sampled pages). SKIP = rustkyll homepage lacks valid HTML, preventing screenshot comparison.

## Structural Equivalence Details

### Tier 1: High fidelity (file match 100%, struct diffs < 30%)

**alexeygrigorev/alexeygrigorev.github.io** -- Perfect structural match. 8/8 files identical, 0/8 structural diffs. No Liquid leaks. Visual diff 1.69% (minor CSS rendering difference).

**alexeygrigorev/kids-horror-stories-ru** -- Perfect structural match. 1345/1345 files, 0/51 sampled diffs. No Liquid leaks. Visual diffs: homepage 0.85%, story pages 0.00-0.03%.

**DataTalksClub/datatalksclub.github.io** -- 787/787 file match. 13/51 structural diffs (mostly HTML entity encoding: `&amp;` vs `&` in URLs). No Liquid leaks. Visual diffs: homepage 2.21%, blog-post 0.00%, courses 0.00%, people 0.00%, books 2.12%, events 1.80%, articles 2.93%. Root causes: minor sort order differences in listing pages; HTML entity encoding differences.

### Tier 2: Good file match, moderate structural diffs

**alexeygrigorev/little-book-of-metals-ru** -- 43/48 files (90%). Missing 5 section index pages (Cyrillic collection names). All 43 common files have structural diffs because rustkyll omits navigation links, headings, and Google Fonts. Visual: 0.00% (homepage renders identically).

**alexeygrigorev/mlwiki.org** -- 640/639 files (100%). 49/51 structural diffs: missing GitHub edit links (theme-specific include). 5 files with raw Liquid tags. Visual: 0.00%.

**alexeygrigorev/snippets** -- 25/25 files (100%). 24/25 structural diffs: 6 category index pages are empty (0 bytes) in rustkyll due to missing layout support. Remaining diffs are missing navigation links. Visual: 0.04%.

**large-blog-3000** -- 3001/3001 files (100%). 50/51 structural diffs: different link ordering in navigation sidebar (sort order difference). No Liquid leaks. Visual: 0.10%.

**muan-blog** -- 2218/2218 files (100%). 25/51 structural diffs. 455 empty HTML files, 22 with raw Liquid tags. Layout partially renders but many pages fall back. Visual: SKIP (homepage not valid HTML).

### Tier 3: Significant gaps

**DataTalksClub/docs** -- 57/57 files (100%). 50/51 structural diffs, 33 Liquid leaks. Uses just-the-docs theme whose sidebar navigation relies on unsupported Liquid features. Visual: SKIP.

**academicpages** -- 17/45 files (38%). Missing 28 files from collections (publications, talks, teaching, portfolio). 16/17 structural diffs: complex Minimal Mistakes-based theme with sidebars. Visual: 3.18%.

**documentation-theme-jekyll** -- 100/100 files (100%). 50/51 structural diffs, 90 Liquid leaks, 12 empty files. Complex data-driven sidebar/navigation uses unsupported Liquid patterns. Visual: SKIP.

**large-docs-site** -- 801/801 files (100%). 50/51 structural diffs: missing page titles and different link ordering in sidebar navigation. Visual: 9.62% (sidebar links different order).

**minimal-mistakes** -- 2/1 files. Rustkyll generates near-empty index.html (0 bytes). Minimal Mistakes gem theme not supported. Visual: SKIP.

**so-simple-theme** -- 11/66 files (17%). Missing 55 files. 7 empty files, 1 Liquid leak. Theme requires many unsupported Liquid features. Visual: SKIP.

## Visual Comparison Details

Visual comparisons were performed by serving both outputs over HTTP and taking full-page Chromium screenshots via Playwright. Pages with >0% diff have root causes noted below.

### Pages with 0% diff (pixel-perfect)

- kids-horror-stories-ru: story-toy (0.00%)
- DataTalksClub/datatalksclub.github.io: blog-post (0.00%), courses (0.00%), people (0.00%)
- little-book-of-metals-ru: homepage (0.00%)
- mlwiki.org: homepage (0.00%)

### Pages with <1% diff (near-perfect)

- kids-horror-stories-ru: homepage (0.85%), story-orchid (0.03%), story-silkworm (0.03%)
- snippets: homepage (0.04%)
- large-blog-3000: homepage (0.10%)

### Pages with 1-5% diff (minor differences)

- alexeygrigorev.github.io: homepage (1.69%) -- Root cause: minor CSS rendering difference (Google Fonts loading timing).
- DataTalksClub/datatalksclub.github.io: homepage (2.21%) -- Root cause: event sort order slightly different.
- DataTalksClub/datatalksclub.github.io: books (2.12%) -- Root cause: book listing order difference.
- DataTalksClub/datatalksclub.github.io: events (1.80%) -- Root cause: event listing sort difference.
- DataTalksClub/datatalksclub.github.io: articles (2.93%) -- Root cause: article listing includes slightly different set.
- academicpages: homepage (3.18%) -- Root cause: missing sidebar and navigation elements from unsupported theme features.

### Pages with >5% diff

- large-docs-site: homepage (9.62%) -- Root cause: sidebar navigation renders links in different sort order, producing visually different layout.

### Sites where visual comparison was skipped

The following sites could not be visually compared because rustkyll's homepage did not contain valid HTML (empty or Liquid fallback output):

- DataTalksClub/docs (just-the-docs theme unsupported)
- documentation-theme-jekyll (complex data-driven navigation)
- minimal-mistakes (gem theme not supported)
- muan-blog (many pages fall back to empty)
- so-simple-theme (theme requires unsupported features)

Diff images are saved under `playwright/screenshots/` organized by site name.

## Compatibility Summary

- Sites that build with both tools: 16 of 32
- Sites that build only with rustkyll: 14 (missing Jekyll gems or incompatible Jekyll version)
- Sites that build only with Jekyll: 0
- Sites that fail with both tools: 4 (bitcoin-org, edition-template, programming-historian, uswds-site)

### Changes from previous benchmark

- **Gained**: muan-blog now builds with rustkyll (was FAIL before)
- **Restored (issue #86)**: beautiful-jekyll (was FAIL, now 0.020s); jekyll-docs/docs (was FAIL, now 1.070s). Root cause: DateFilter panic on chrono format error, fixed with safe_chrono_format helper.
- **Jekyll environment issue**: homebrew-site Jekyll FAIL caused by Ruby version mismatch (Gemfile specifies Ruby 4.0.1, local system has 3.3.7). Rustkyll builds it successfully (0.049s, 134 pages).
- **DTC speedup improved**: 5.925s -> 1.877s (from 3.2x to 10.2x vs Jekyll)
- **Page counts increased**: little-book-of-metals-ru 1->43, mlwiki.org 2->640, snippets 2->25, DataTalksClub/docs 1->57, academicpages 1->17, so-simple-theme 2->11 (kramdown + template improvements)

## Notes

- FAIL means the tool could not build the site (template error, missing plugin, etc.)
- TIMEOUT means the build exceeded 120s and was killed
- Speedup = Jekyll time / rustkyll time (higher is better for rustkyll)
- Page count is the number of HTML files generated in _site/
- Each build starts from a clean _site/ directory (no caching)
- Jekyll builds use bundle exec when a Gemfile is present
- rustkyll is pre-compiled in release mode
- Jekyll FAIL entries are often due to missing Ruby gems (bundle install not run) rather than tool limitations. homebrew-site Jekyll failure is specifically caused by Ruby version mismatch (Gemfile requires Ruby 4.0.1, local system has 3.3.7)
- Structural comparison samples up to 51 common files per site
- Visual comparison uses Playwright/Chromium at 1280x720 viewport, full-page screenshots
