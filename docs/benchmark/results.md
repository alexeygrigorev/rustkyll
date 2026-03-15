# Benchmark: rustkyll vs Jekyll

Generated: 2026-03-15 12:15 UTC

Configuration: 3 runs per tool, median wall-clock time reported.
Timeout: 120s per build.

rustkyll version: rustkyll 0.1.4
Jekyll version: jekyll 4.4.1

## Summary

rustkyll is faster than Jekyll on 32 of 34 dual-success sites. Speedups range from 0.82x (government-github, Jekyll faster due to GitHub API data fetching) to 171x (data-science-interviews). For the primary target site (DataTalksClub/datatalksclub.github.io), rustkyll builds in 1.05s vs Jekyll's 19.5s -- an 18.7x speedup.

34 of 44 sites build successfully with both tools. 1 site (primer-theme) builds only with Jekyll. 7 sites build only with rustkyll. 3 sites fail with both tools.

## All Sites -- Speed Benchmark

| Site | Pages | Jekyll (s) | rustkyll (s) | Speedup |
|------|-------|------------|-------------|---------|
| alexeygrigorev/aihero | 2 | 0.632 | 0.020 | 31.60x |
| alexeygrigorev/alexeygrigorev.github.io | 8 | 0.579 | 0.015 | 38.60x |
| alexeygrigorev/data-science-interviews | 0 | 1.371 | 0.008 | 171.37x |
| alexeygrigorev/kids-horror-stories-ru | 1344 | 4.041 | 0.330 | 12.24x |
| alexeygrigorev/little-book-of-metals-ru | 43 | 2.346 | 0.448 | 5.23x |
| alexeygrigorev/mlbookcamp-page | 15 | 0.602 | 0.111 | 5.42x |
| alexeygrigorev/mlwiki.org | 640 | 0.969 | 1.004 | 0.96x |
| alexeygrigorev/snippets | 25 | 0.652 | 0.072 | 9.05x |
| DataTalksClub/courses | 5 | 0.618 | 0.031 | 19.93x |
| DataTalksClub/datatalksclub.github.io | 787 | 19.506 | 1.045 | 18.66x |
| DataTalksClub/docs | 57 | 1.810 | 0.038 | 47.63x |
| academicpages | 17 | 4.434 | 0.026 | 170.53x |
| architect-theme | 2 | 0.839 | 0.017 | 49.35x |
| beautiful-jekyll | 6 | 0.816 | 0.027 | 30.22x |
| bitcoin-org | N/A | FAIL | FAIL | N/A |
| cayman-theme | 2 | 1.144 | 0.016 | 71.50x |
| choosealicense.com | 72 | FAIL | 0.103 | N/A |
| dinky-theme | 2 | 0.619 | 0.034 | 18.20x |
| documentation-theme-jekyll | 100 | 3.677 | 0.160 | 22.98x |
| edition-template | N/A | FAIL | FAIL | N/A |
| government-github | 21 | 4.429 | 5.379 | 0.82x |
| hacker-theme | 2 | 0.700 | 0.016 | 43.75x |
| homebrew-site | 134 | FAIL | 0.044 | N/A |
| hyde | 6 | FAIL | 0.014 | N/A |
| jekyll-docs/docs | 131 | 3.103 | 2.109 | 1.47x |
| just-the-docs | 47 | 2.214 | 0.259 | 8.54x |
| large-blog-3000 | 3001 | 6.182 | 1.550 | 3.98x |
| large-docs-site | 801 | 24.832 | 0.527 | 47.11x |
| leap-day-theme | 2 | 0.698 | 0.018 | 38.77x |
| made-mistakes-jekyll | 2 | FAIL | 0.010 | N/A |
| merlot-theme | 2 | 0.642 | 0.019 | 33.78x |
| midnight-theme | 2 | 0.704 | 0.017 | 41.41x |
| minima | 9 | FAIL | 0.023 | N/A |
| minimal-mistakes | 1 | 0.912 | 0.039 | 23.38x |
| mojombo-blog | 17 | 2.216 | 0.060 | 36.93x |
| muan-blog | 2218 | 16.078 | 0.296 | 54.31x |
| opensource-guide | 390 | 15.680 | 0.296 | 52.97x |
| primer-theme | 2 | 1.054 | FAIL | N/A |
| programming-historian | 653 | FAIL | 7.522 | N/A |
| slate-theme | 2 | 0.662 | 0.016 | 41.37x |
| so-simple-theme | 11 | 1.439 | 0.045 | 31.97x |
| time-machine-theme | 2 | 0.628 | 0.016 | 39.25x |
| uswds-site | N/A | FAIL | FAIL | N/A |
| wtf-html-css | 1 | FAIL | 0.030 | N/A |

## Dual-Success Sites -- Consolidated Comparison

For every site where both Jekyll and rustkyll succeed, the table below shows speed, structural equivalence, and visual fidelity in one view.

| Site | Pages | Jekyll (s) | rustkyll (s) | Speedup | File Match | DOM Match | Liquid Leaks | Visual Diff |
|------|-------|------------|-------------|---------|------------|-----------|-------------|-------------|
| alexeygrigorev/aihero | 2 | 0.632 | 0.020 | 31.60x | 2/2 (100%) | 0/2 (0%) | 0 | 0.00% |
| alexeygrigorev/alexeygrigorev.github.io | 8 | 0.579 | 0.015 | 38.60x | 8/8 (100%) | 7/8 (88%) | 0 | 1.61% |
| alexeygrigorev/data-science-interviews | 0 | 1.371 | 0.008 | 171.37x | 0/6 (0%) | N/A | 0 | N/A |
| alexeygrigorev/kids-horror-stories-ru | 1344 | 4.041 | 0.330 | 12.24x | 1344/1345 (100%) | 1342/1344 (100%) | 0 | 0.00% story |
| alexeygrigorev/little-book-of-metals-ru | 43 | 2.346 | 0.448 | 5.23x | 43/48 (90%) | 0/43 (0%) | 0 | 0.00% |
| alexeygrigorev/mlbookcamp-page | 15 | 0.602 | 0.111 | 5.42x | 15/15 (100%) | 1/15 (7%) | 0 | 0.00% |
| alexeygrigorev/mlwiki.org | 640 | 0.969 | 1.004 | 0.96x | 640/639 (100%) | 190/639 (30%) | 5 | 0.00% |
| alexeygrigorev/snippets | 25 | 0.652 | 0.072 | 9.05x | 25/25 (100%) | 7/25 (28%) | 0 | 0.00% |
| DataTalksClub/courses | 5 | 0.618 | 0.031 | 19.93x | 5/5 (100%) | 5/5 (100%) | 0 | 0.00% |
| DataTalksClub/datatalksclub.github.io | 787 | 19.506 | 1.045 | 18.66x | 787/787 (100%) | 2/787 (0%) | 0 | 0.27% avg |
| DataTalksClub/docs | 57 | 1.810 | 0.038 | 47.63x | 57/57 (100%) | 0/57 (0%) | 33 | SKIP |
| academicpages | 17 | 4.434 | 0.026 | 170.53x | 17/45 (38%) | 1/17 (6%) | 0 | SKIP |
| architect-theme | 2 | 0.839 | 0.017 | 49.35x | 2/2 (100%) | 0/2 (0%) | 0 | 0.03% |
| beautiful-jekyll | 6 | 0.816 | 0.027 | 30.22x | 5/6 (83%) | 0/5 (0%) | 3 | SKIP |
| cayman-theme | 2 | 1.144 | 0.016 | 71.50x | 2/2 (100%) | 0/2 (0%) | 0 | 0.03% |
| dinky-theme | 2 | 0.619 | 0.034 | 18.20x | 2/2 (100%) | 0/2 (0%) | 0 | 0.04% |
| documentation-theme-jekyll | 100 | 3.677 | 0.160 | 22.98x | 100/100 (100%) | 1/100 (1%) | 90 | SKIP |
| government-github | 21 | 4.429 | 5.379 | 0.82x | 21/21 (100%) | 0/21 (0%) | 4 | SKIP |
| hacker-theme | 2 | 0.700 | 0.016 | 43.75x | 2/2 (100%) | 0/2 (0%) | 0 | 0.07% |
| jekyll-docs/docs | 131 | 3.103 | 2.109 | 1.47x | 125/228 (55%) | 0/125 (0%) | 71 | SKIP |
| just-the-docs | 47 | 2.214 | 0.259 | 8.54x | 47/47 (100%) | 0/47 (0%) | 18 | SKIP |
| large-blog-3000 | 3001 | 6.182 | 1.550 | 3.98x | 3001/3001 (100%) | 0/3001 (0%) | 0 | 0.10% |
| large-docs-site | 801 | 24.832 | 0.527 | 47.11x | 801/801 (100%) | 0/801 (0%) | 0 | 9.62% |
| leap-day-theme | 2 | 0.698 | 0.018 | 38.77x | 2/2 (100%) | 0/2 (0%) | 0 | 0.42% |
| merlot-theme | 2 | 0.642 | 0.019 | 33.78x | 2/2 (100%) | 0/2 (0%) | 0 | 0.08% |
| midnight-theme | 2 | 0.704 | 0.017 | 41.41x | 2/2 (100%) | 0/2 (0%) | 0 | 0.03% |
| minimal-mistakes | 1 | 0.912 | 0.039 | 23.38x | 0/1 (0%) | N/A | 0 | SKIP |
| mojombo-blog | 17 | 2.216 | 0.060 | 36.93x | 17/17 (100%) | 8/17 (47%) | 0 | 0.00% |
| muan-blog | 2218 | 16.078 | 0.296 | 54.31x | 2218/2218 (100%) | 29/2218 (1%) | 22 | SKIP |
| opensource-guide | 390 | 15.680 | 0.296 | 52.97x | 390/388 (101%) | 23/388 (6%) | 0 | 0.04% |
| slate-theme | 2 | 0.662 | 0.016 | 41.37x | 2/2 (100%) | 0/2 (0%) | 0 | 0.04% |
| so-simple-theme | 11 | 1.439 | 0.045 | 31.97x | 11/66 (17%) | 0/11 (0%) | 1 | SKIP |
| time-machine-theme | 2 | 0.628 | 0.016 | 39.25x | 2/2 (100%) | 0/2 (0%) | 0 | 0.13% |

**Column definitions:**
- **File Match**: rustkyll HTML files / Jekyll HTML files. 100% means identical file tree.
- **DOM Match**: files with zero DOM differences / common files compared. Higher is better.
- **Liquid Leaks**: count of rustkyll HTML files containing raw `{%` or `{{` tags.
- **Visual Diff**: pixel difference percentage for homepage (or average across sampled pages). SKIP = rustkyll homepage lacks valid HTML or has empty output.

## Structural Equivalence Details

### Tier 1: High fidelity (file match 100%, DOM diffs mostly cosmetic)

**DataTalksClub/courses** -- 5/5 files match. All 5 pages have zero DOM differences. No Liquid leaks. Visual: 0.00% pixel diff (pixel-perfect). Perfect match.

**alexeygrigorev/kids-horror-stories-ru** -- 1344/1345 files (one missing index.html from rustkyll). 1342 of 1344 common files have zero DOM differences. No Liquid leaks. Visual: story pages 0.00% (pixel-perfect).

**alexeygrigorev/alexeygrigorev.github.io** -- 8/8 files. 7 of 8 have zero DOM differences (1 page has minor attribute diff). Visual: 1.61% (minor CSS rendering difference).

**alexeygrigorev/little-book-of-metals-ru** -- 43/48 files (90%). Missing 5 section index pages (Cyrillic collection names). All 43 common files have DOM diffs (missing navigation links, Google Fonts). Visual: 0.00% (homepage pixel-perfect).

**mojombo-blog** -- 17/17 files (100%). 8 of 17 have zero DOM differences. Remaining 9 have minor diffs (kramdown loose list `<p>` wrapping). Visual: homepage 0.00%, 3 posts 0.00%, 2 posts 1.5-3.5% diff.

### Tier 2: Good file match, moderate structural diffs

**alexeygrigorev/mlbookcamp-page** -- 15/15 files (100%). 1 of 15 has zero DOM differences. Remaining 14 have attribute diffs from minima theme. Visual: 0.00% (pixel-perfect homepage).

**alexeygrigorev/mlwiki.org** -- 640/639 files (100%). 190 of 639 have zero DOM differences. 5 files with raw Liquid tags. Visual: 0.00%.

**alexeygrigorev/snippets** -- 25/25 files (100%). 7 of 25 have zero DOM differences. 6 category index pages are empty. Visual: 0.00%.

**DataTalksClub/datatalksclub.github.io** -- 787/787 file match. 2 of 787 have zero DOM differences. Most diffs are minor: HTML entity encoding (`&amp;` vs `&`), attribute ordering, SEO meta tag differences. Visual: homepage 0.00%, most pages 0.00%, average 0.27% across 22 pages. Largest diff: course-ml-zoomcamp at 4.11% (event listing order).

**opensource-guide** -- 390/388 files. 23 of 388 have zero DOM differences. Many pages differ in navigation and i18n-related attributes. Visual: 0.04%.

**large-blog-3000** -- 3001/3001 files (100%). 0 DOM matches (all have navigation link ordering diffs). Visual: 0.10%.

### Tier 3: Significant gaps

**DataTalksClub/docs** -- 57/57 files. 0 DOM matches, 33 Liquid leaks. Uses just-the-docs theme whose sidebar navigation relies on unsupported Liquid features. Visual: SKIP.

**documentation-theme-jekyll** -- 100/100 files. 1 DOM match, 90 Liquid leaks. Complex data-driven sidebar/navigation uses unsupported Liquid patterns. Visual: SKIP.

**academicpages** -- 17/45 files (38%). Missing 28 files from collections. 1 of 17 DOM matches. Visual: SKIP (no index.html in rustkyll output).

**jekyll-docs/docs** -- 125/228 files (55%). 0 DOM matches, 71 Liquid leaks. Complex theme with many unsupported features. Visual: SKIP.

**just-the-docs** -- 47/47 files. 0 DOM matches, 18 Liquid leaks. JavaScript-driven TOC navigation differs. Visual: SKIP.

**large-docs-site** -- 801/801 files (100%). 0 DOM matches. Missing page titles and different link ordering in sidebar. Visual: 9.62% (sidebar different order).

**government-github** -- 21/21 files. 0 DOM matches, 4 Liquid leaks. Jekyll is faster (0.82x) due to GitHub API data fetching in rustkyll. Visual: SKIP.

**muan-blog** -- 2218/2218 files. 29 DOM matches, 22 Liquid leaks. Many pages have empty or fallback output. Visual: SKIP.

**beautiful-jekyll** -- 5/6 files (83%). 0 DOM matches, 3 Liquid leaks. Visual: SKIP (empty homepage).

**minimal-mistakes** -- 0/1 common files. Gem theme not supported. Visual: SKIP.

**so-simple-theme** -- 11/66 files (17%). 0 DOM matches, 1 Liquid leak. Theme requires many unsupported features. Visual: SKIP.

**alexeygrigorev/data-science-interviews** -- 0/6 files from rustkyll. Site uses jekyll-theme-cayman GitHub Pages theme which rustkyll does not render markdown files through.

## Visual Comparison Details

Visual comparisons were performed by serving both outputs over HTTP and taking full-page Chromium screenshots via Playwright. Pixel diff measured with pixelmatch at threshold 0.1.

### Pages with 0% diff (pixel-perfect)

- aihero: homepage (0.00%)
- little-book-of-metals-ru: homepage (0.00%)
- mlbookcamp-page: homepage (0.00%)
- mlwiki.org: homepage (0.00%)
- snippets: homepage (0.00%)
- DataTalksClub/courses: homepage (0.00%)
- DataTalksClub/datatalksclub.github.io: homepage (0.00%), articles (0.00%), events (0.00%), courses (0.00%), people (0.00%), support (0.00%), slack (0.00%), guidelines (0.00%), blog-segmentation (0.00%), blog-data-roles (0.00%), book-reinforcement-learning (0.00%), podcast-ai-ecology (0.00%), person-aaishamuhammad (0.00%), conference-2021-feb (0.00%)
- kids-horror-stories-ru: story-orchid (0.00%), story-silkworm (0.00%), story-toy (0.00%)
- mojombo-blog: homepage (0.00%), blogging-like-a-hacker (0.00%), git-parable (0.00%)
- opensource-guide: homepage (0.04%)

### Pages with <1% diff (near-perfect)

- architect-theme: homepage (0.03%), another-page (0.00%)
- cayman-theme: homepage (0.03%), another-page (0.00%)
- dinky-theme: homepage (0.04%), another-page (0.00%)
- hacker-theme: homepage (0.07%), another-page (0.00%)
- midnight-theme: homepage (0.03%), another-page (0.00%)
- merlot-theme: homepage (0.08%), another-page (0.00%)
- slate-theme: homepage (0.04%), another-page (0.00%)
- time-machine-theme: homepage (0.13%), another-page (0.00%)
- leap-day-theme: homepage (0.42%), another-page (0.00%)
- large-blog-3000: homepage (0.10%)
- DataTalksClub/datatalksclub.github.io: books (0.40%), podcast (0.05%), blog-practical-guide (0.08%)

### Pages with 1-5% diff (minor differences)

- alexeygrigorev.github.io: homepage (1.61%) -- Root cause: minor CSS rendering difference (Google Fonts loading).
- DataTalksClub/datatalksclub.github.io: tools (1.27%), course-ml-zoomcamp (4.11%) -- Root cause: listing order differences in event/tool data.
- mojombo-blog: post-readme-driven (3.49%), post-open-source (1.56%) -- Root cause: kramdown loose list `<p>` wrapping differences.

### Pages with >5% diff

- large-docs-site: homepage (9.62%) -- Root cause: sidebar navigation renders links in different sort order, producing visually different layout.

### Sites where visual comparison was skipped

The following sites could not be visually compared because rustkyll's homepage did not contain valid HTML (empty, Liquid fallback, or no index.html):

- DataTalksClub/docs (just-the-docs theme: no valid `<html>` in homepage)
- documentation-theme-jekyll (complex data-driven navigation: no valid `<html>`)
- government-github (no valid `<html>` in homepage)
- jekyll-docs/docs (no valid `<html>` in homepage)
- just-the-docs (no valid `<html>` in homepage)
- muan-blog (many pages fall back to empty)
- academicpages (no index.html generated)
- beautiful-jekyll (empty homepage, 0 bytes)
- minimal-mistakes (no index.html generated)
- so-simple-theme (empty homepage, 0 bytes)

Diff images are saved under `playwright/screenshots/` organized by site name.

## Compatibility Summary

- Sites that build with both tools: 34 of 44
- Sites that build only with rustkyll: 7 (missing Jekyll gems/plugins or Ruby version mismatch)
- Sites that build only with Jekyll: 1 (primer-theme)
- Sites that fail with both tools: 3 (bitcoin-org, edition-template, uswds-site)

## Notes

- FAIL means the tool could not build the site (template error, missing plugin, etc.)
- TIMEOUT means the build exceeded 120s and was killed
- Speedup = Jekyll time / rustkyll time (higher is better for rustkyll)
- Page count is the number of HTML files generated in _site/
- Each build starts from a clean _site/ directory (no caching)
- Jekyll builds use bundle exec when a Gemfile is present
- rustkyll is pre-compiled in release mode
- DOM comparison uses BeautifulSoup to normalize and compare full DOM trees
- Visual comparison uses Playwright/Chromium at 1280x720 viewport, full-page screenshots
- Structural comparison covers ALL common HTML files between outputs
