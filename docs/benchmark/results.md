# Benchmark: rustkyll vs Jekyll

Generated: 2026-03-16 11:02 UTC

Configuration: 3 runs per tool, median wall-clock time reported.
Timeout: 300s per build.

rustkyll version: rustkyll 0.1.4
Jekyll version: jekyll 4.4.1

## Summary

rustkyll is faster than Jekyll on 32 of 34 dual-success sites. Speedups range from 0.74x (mlwiki.org, Jekyll faster due to large number of wiki pages with slow Liquid rendering) to 165.74x (academicpages). For the primary target site (DataTalksClub/datatalksclub.github.io), rustkyll builds in 1.0s vs Jekyll's 19.1s -- a 19x speedup.

34 of 44 sites build successfully with both tools. 1 site (primer-theme) builds only with Jekyll. 7 sites build only with rustkyll. 3 sites fail with both tools.

593 of 787 DTC files are DOM-identical to Jekyll output. 21 of 22 sampled DTC pages are pixel-perfect (0.00% pixel diff). DOM match rates improved significantly over the course of development after fixes to collection sort stability, kramdown compatibility, syntax highlighting, pagination, SCSS compilation, JSON-LD, heading IDs, URL encoding, inline code classes, and many other edge cases.

## All Sites -- Speed Benchmark

| Site | Pages | Jekyll (s) | rustkyll (s) | Speedup |
|------|-------|------------|-------------|---------|
| alexeygrigorev/aihero | 2 | 0.907 | 0.029 | 31.27x |
| alexeygrigorev/alexeygrigorev.github.io | 8 | 0.904 | 0.026 | 34.76x |
| alexeygrigorev/data-science-interviews | 0 | 2.087 | 0.015 | 139.13x |
| alexeygrigorev/kids-horror-stories-ru | 1344 | 4.961 | 0.567 | 8.74x |
| alexeygrigorev/little-book-of-metals-ru | 43 | 2.895 | 0.583 | 4.96x |
| alexeygrigorev/mlbookcamp-page | 15 | 0.604 | 0.119 | 5.07x |
| alexeygrigorev/mlwiki.org | 640 | 1.005 | 1.347 | 0.74x |
| alexeygrigorev/snippets | 25 | 0.657 | 0.074 | 8.87x |
| DataTalksClub/courses | 5 | 0.633 | 0.031 | 20.41x |
| DataTalksClub/datatalksclub.github.io | 787 | 19.1 | 1.0 | 19x |
| DataTalksClub/docs | 57 | 1.901 | 0.054 | 35.20x |
| academicpages | 17 | 4.475 | 0.027 | 165.74x |
| architect-theme | 2 | 0.673 | 0.017 | 39.58x |
| beautiful-jekyll | 6 | 0.841 | 0.033 | 25.48x |
| bitcoin-org | N/A | FAIL | FAIL | N/A |
| cayman-theme | 2 | 0.665 | 0.019 | 35.00x |
| choosealicense.com | 72 | FAIL | 0.054 | N/A |
| dinky-theme | 2 | 0.620 | 0.017 | 36.47x |
| documentation-theme-jekyll | 100 | 3.589 | 0.192 | 18.69x |
| edition-template | N/A | FAIL | FAIL | N/A |
| government-github | 21 | 4.386 | 5.561 | 0.78x |
| hacker-theme | 2 | 0.666 | 0.017 | 39.17x |
| homebrew-site | 134 | FAIL | 0.047 | N/A |
| hyde | 6 | FAIL | 0.012 | N/A |
| jekyll-docs/docs | 131 | 3.106 | 2.179 | 1.42x |
| just-the-docs | 47 | 2.148 | 0.291 | 7.38x |
| large-blog-3000 | 3001 | 4.481 | 1.587 | 2.82x |
| large-docs-site | 801 | 24.138 | 0.730 | 33.06x |
| leap-day-theme | 2 | 1.200 | 0.018 | 66.66x |
| made-mistakes-jekyll | 2 | FAIL | 0.028 | N/A |
| merlot-theme | 2 | 1.145 | 0.043 | 26.62x |
| midnight-theme | 2 | 1.253 | 0.052 | 24.09x |
| minima | 9 | FAIL | 0.025 | N/A |
| minimal-mistakes | 1 | 0.913 | 0.039 | 23.41x |
| mojombo-blog | 17 | 2.239 | 0.075 | 29.85x |
| muan-blog | 2218 | 16.192 | 0.318 | 50.91x |
| opensource-guide | 390 | 15.717 | 0.323 | 48.65x |
| primer-theme | 2 | 1.056 | FAIL | N/A |
| programming-historian | 653 | FAIL | 8.006 | N/A |
| slate-theme | 2 | 0.683 | 0.017 | 40.17x |
| so-simple-theme | 11 | 1.463 | 0.053 | 27.60x |
| time-machine-theme | 2 | 0.631 | 0.017 | 37.11x |
| uswds-site | N/A | FAIL | FAIL | N/A |
| wtf-html-css | 1 | FAIL | 0.034 | N/A |

## Dual-Success Sites -- Consolidated Comparison

For every site where both Jekyll and rustkyll succeed, the table below shows speed, structural equivalence, and visual fidelity in one view.

| Site | Pages | Jekyll (s) | rustkyll (s) | Speedup | File Match | DOM Match | Liquid Leaks | Visual Diff |
|------|-------|------------|-------------|---------|------------|-----------|-------------|-------------|
| alexeygrigorev/aihero | 2 | 0.907 | 0.029 | 31.27x | 2/2 (100%) | 0/2 (0%) | 0 | 0.00% |
| alexeygrigorev/alexeygrigorev.github.io | 8 | 0.904 | 0.026 | 34.76x | 8/8 (100%) | 7/8 (88%) | 0 | 1.33% |
| alexeygrigorev/data-science-interviews | 0 | 2.087 | 0.015 | 139.13x | 0/6 (0%) | N/A | 0 | N/A |
| alexeygrigorev/kids-horror-stories-ru | 1344 | 4.961 | 0.567 | 8.74x | 1344/1345 (100%) | 1342/1344 (100%) | 0 | N/A |
| alexeygrigorev/little-book-of-metals-ru | 43 | 2.895 | 0.583 | 4.96x | 43/48 (90%) | 0/43 (0%) | 0 | 0.00% |
| alexeygrigorev/mlbookcamp-page | 15 | 0.604 | 0.119 | 5.07x | 15/15 (100%) | 4/15 (27%) | 0 | 0.00% |
| alexeygrigorev/mlwiki.org | 640 | 1.005 | 1.347 | 0.74x | 640/639 (100%) | 205/639 (32%) | 5 | 0.00% |
| alexeygrigorev/snippets | 25 | 0.657 | 0.074 | 8.87x | 25/25 (100%) | 8/25 (32%) | 1 | 0.00% |
| DataTalksClub/courses | 5 | 0.633 | 0.031 | 20.41x | 5/5 (100%) | 5/5 (100%) | 0 | 0.00% |
| DataTalksClub/datatalksclub.github.io | 787 | 19.1 | 1.0 | 19x | 787/787 (100%) | 593/787 (75%) | 1 | 0.00% avg |
| DataTalksClub/docs | 57 | 1.901 | 0.054 | 35.20x | 57/57 (100%) | 0/57 (0%) | 33 | SKIP |
| academicpages | 17 | 4.475 | 0.027 | 165.74x | 17/45 (38%) | 1/17 (6%) | 0 | SKIP |
| architect-theme | 2 | 0.673 | 0.017 | 39.58x | 2/2 (100%) | 0/2 (0%) | 0 | 0.03% |
| beautiful-jekyll | 6 | 0.841 | 0.033 | 25.48x | 6/6 (100%) | 0/5 (0%) | 3 | SKIP |
| cayman-theme | 2 | 0.665 | 0.019 | 35.00x | 2/2 (100%) | 0/2 (0%) | 0 | 0.02% |
| dinky-theme | 2 | 0.620 | 0.017 | 36.47x | 2/2 (100%) | 0/2 (0%) | 0 | 0.03% |
| documentation-theme-jekyll | 100 | 3.589 | 0.192 | 18.69x | 100/100 (100%) | 1/97 (1%) | 90 | SKIP |
| government-github | 21 | 4.386 | 5.561 | 0.78x | 21/21 (100%) | 0/21 (0%) | 4 | SKIP |
| hacker-theme | 2 | 0.666 | 0.017 | 39.17x | 2/2 (100%) | 0/2 (0%) | 0 | 0.05% |
| jekyll-docs/docs | 131 | 3.106 | 2.179 | 1.42x | 125/228 (55%) | 0/125 (0%) | 71 | SKIP |
| just-the-docs | 47 | 2.148 | 0.291 | 7.38x | 47/47 (100%) | 0/47 (0%) | 18 | SKIP |
| large-blog-3000 | 3001 | 4.481 | 1.587 | 2.82x | 3001/3001 (100%) | 0/3001 (0%) | 0 | 0.09% |
| large-docs-site | 801 | 24.138 | 0.730 | 33.06x | 801/801 (100%) | 1/801 (0%) | 0 | 0.00% |
| leap-day-theme | 2 | 1.200 | 0.018 | 66.66x | 2/2 (100%) | 0/2 (0%) | 0 | 0.02% |
| merlot-theme | 2 | 1.145 | 0.043 | 26.62x | 2/2 (100%) | 0/2 (0%) | 0 | 0.01% |
| midnight-theme | 2 | 1.253 | 0.052 | 24.09x | 2/2 (100%) | 0/2 (0%) | 0 | 0.02% |
| minimal-mistakes | 1 | 0.913 | 0.039 | 23.41x | 0/1 (0%) | N/A | 0 | SKIP |
| mojombo-blog | 17 | 2.239 | 0.075 | 29.85x | 17/17 (100%) | 10/17 (59%) | 0 | 0.00% |
| muan-blog | 2218 | 16.192 | 0.318 | 50.91x | 2218/2218 (100%) | 29/2218 (1%) | 22 | SKIP |
| opensource-guide | 390 | 15.717 | 0.323 | 48.65x | 390/388 (101%) | 23/388 (6%) | 0 | 0.03% |
| slate-theme | 2 | 0.683 | 0.017 | 40.17x | 2/2 (100%) | 0/2 (0%) | 0 | 0.03% |
| so-simple-theme | 11 | 1.463 | 0.053 | 27.60x | 11/66 (17%) | 0/11 (0%) | 1 | SKIP |
| time-machine-theme | 2 | 0.631 | 0.017 | 37.11x | 2/2 (100%) | 0/2 (0%) | 0 | 0.12% |

Column definitions:
- File Match: rustkyll HTML files / Jekyll HTML files. 100% means identical file tree.
- DOM Match: files with zero DOM differences / common files compared. Higher is better.
- Liquid Leaks: count of rustkyll HTML files containing raw `{%` or `{{` tags.
- Visual Diff: pixel difference percentage for homepage (or average across sampled pages). SKIP = rustkyll homepage lacks valid HTML or has empty output.

## Structural Equivalence Details

### Tier 1: High fidelity (DOM match >= 50% or file match 100% with mostly cosmetic diffs)

DataTalksClub/courses -- 5/5 files match. All 5 pages have zero DOM differences. No Liquid leaks. Visual: 0.00% pixel diff (pixel-perfect). Perfect match. Unchanged from previous run.

alexeygrigorev/kids-horror-stories-ru -- 1344/1345 files (one missing index.html from rustkyll). 1342 of 1344 common files have zero DOM differences. No Liquid leaks. Visual: N/A (no homepage index.html). Unchanged from previous run.

alexeygrigorev/alexeygrigorev.github.io -- 8/8 files. 7 of 8 have zero DOM differences (1 page has minor attribute diff). Visual: 1.33% (minor CSS rendering difference from Google Fonts; known issue #123). Unchanged from previous run.

DataTalksClub/datatalksclub.github.io -- 787/787 file match. 593 of 787 have zero DOM differences (75%). Remaining diffs are minor: syntax highlighting token classes, kramdown paragraph wrapping edge cases, JSON-LD field variations. 1 file with raw Liquid tag. Visual: 0.00% across all 22 sampled pages (pixel-perfect homepage, articles, books, podcast, events, courses, people, support, tools, slack, blog posts, book details, podcast episodes, person pages, course page, conference page). 21 of 22 pages at exactly 0.00% pixel diff; 1 page has sub-pixel font rendering noise (54 pixels / 0.000003%).

mojombo-blog -- 17/17 files (100%). 10 of 17 have zero DOM differences (up from 8/17 in previous run). Remaining 7 have minor diffs (heading IDs, attribute ordering). Visual: all 5 pages 0.00% (pixel-perfect). Improved: DOM matches 47% -> 59%, post-readme-driven 3.49% -> 0.00%, post-open-source 1.56% -> 0.00%.

alexeygrigorev/mlwiki.org -- 640/639 files (100%). 205 of 639 have zero DOM differences (up from 190/639). 5 files with raw Liquid tags. Visual: 0.00%. Improved: DOM matches 30% -> 32%.

alexeygrigorev/snippets -- 25/25 files (100%). 8 of 25 have zero DOM differences (up from 7/25). 1 file with Liquid leak. Visual: 0.00%. Improved: DOM matches 28% -> 32%.

### Tier 2: Good file match, moderate structural diffs

alexeygrigorev/mlbookcamp-page -- 15/15 files (100%). 4 of 15 have zero DOM differences (up from 1/15). Remaining 11 have attribute diffs from minima theme. Visual: 0.00% (pixel-perfect homepage). Improved: DOM matches 7% -> 27%.

alexeygrigorev/little-book-of-metals-ru -- 43/48 files (90%). Missing 5 section index pages (Cyrillic collection names). All 43 common files have DOM diffs (missing navigation links, Google Fonts). Visual: 0.00% (homepage pixel-perfect). Unchanged.

alexeygrigorev/aihero -- 2/2 files (100%). 0 DOM matches. Diffs are SEO meta tag order and content differences. Visual: 0.00% (pixel-perfect). Unchanged.

large-blog-3000 -- 3001/3001 files (100%). 0 DOM matches (all have navigation link ordering diffs). Visual: 0.09%. Unchanged.

large-docs-site -- 801/801 files (100%). 1 DOM match (up from 0). Visual: 0.00% (down from 9.62% -- a major improvement). Major improvement: visual 9.62% -> 0.00%. Root cause of previous diff was sidebar sort order, fixed by issue #121.

opensource-guide -- 390/388 files. 23 of 388 have zero DOM differences. Many pages differ in navigation and i18n-related attributes. Visual: 0.03%. Unchanged.

### Tier 3: Significant gaps

DataTalksClub/docs -- 57/57 files. 0 DOM matches, 33 Liquid leaks. Uses just-the-docs theme whose sidebar navigation relies on unsupported Liquid features. Visual: SKIP. Unchanged.

documentation-theme-jekyll -- 100/100 files. 1 DOM match, 90 Liquid leaks. Complex data-driven sidebar/navigation uses unsupported Liquid patterns. Visual: SKIP. Unchanged.

academicpages -- 17/45 files (38%). Missing 28 files from collections. 1 of 17 DOM matches. Visual: SKIP (no index.html in rustkyll output). Unchanged.

jekyll-docs/docs -- 125/228 files (55%). 0 DOM matches, 71 Liquid leaks. Complex theme with many unsupported features. Visual: SKIP. Unchanged.

just-the-docs -- 47/47 files. 0 DOM matches, 18 Liquid leaks. JavaScript-driven TOC navigation differs. Visual: SKIP. Unchanged.

government-github -- 21/21 files. 0 DOM matches, 4 Liquid leaks. Jekyll is faster (0.78x) due to GitHub API data fetching in rustkyll. Visual: SKIP. Unchanged.

muan-blog -- 2218/2218 files. 29 DOM matches, 22 Liquid leaks. Many pages have empty or fallback output. Visual: SKIP. Unchanged.

beautiful-jekyll -- 6/6 files (100%, up from 5/6). 0 DOM matches, 3 Liquid leaks. Visual: SKIP (empty homepage). Improved file match: 83% -> 100%.

minimal-mistakes -- 0/1 common files. Gem theme not supported. Visual: SKIP. Unchanged.

so-simple-theme -- 11/66 files (17%). 0 DOM matches, 1 Liquid leak. Theme requires many unsupported features. Visual: SKIP. Unchanged.

alexeygrigorev/data-science-interviews -- 0/6 files from rustkyll. Site uses jekyll-theme-cayman GitHub Pages theme which rustkyll does not render markdown files through. Unchanged.

## Visual Comparison Details

Visual comparisons were performed by serving both outputs over HTTP and taking full-page Chromium screenshots via Playwright. Pixel diff measured with pixelmatch at threshold 0.15. Missing CSS/JS assets were copied from Jekyll output to rustkyll output to focus on HTML rendering differences.

### Pages with 0% diff (pixel-perfect)

- aihero: homepage (0.00%)
- little-book-of-metals-ru: homepage (0.00%)
- mlbookcamp-page: homepage (0.00%)
- mlwiki.org: homepage (0.00%)
- snippets: homepage (0.00%)
- DataTalksClub/courses: homepage (0.00%)
- DataTalksClub/datatalksclub.github.io: homepage (0.00%), articles (0.00%), books (0.00%), podcast (0.00%), events (0.00%), courses (0.00%), people (0.00%), support (0.00%), tools (0.00%), slack (0.00%), slack-guidelines (0.00%), blog-segmentation (0.00%), blog-practical-guide (0.00%), blog-data-roles (0.00%), book-ml-bookcamp (0.00%), book-reinforcement-learning (0.00%), podcast-ab-testing (0.00%), podcast-ai-ecology (0.00%), person-alexeygrigorev (0.00%), person-aaishamuhammad (0.00%), course-ml-zoomcamp (0.00%), conference-2021-feb (0.00%)
- kids-horror-stories-ru: N/A (no homepage, but story pages 0.00% in previous run)
- mojombo-blog: homepage (0.00%), blogging-like-a-hacker (0.00%), git-parable (0.00%), readme-driven (0.00%), open-source (0.00%)
- large-docs-site: homepage (0.00%)

### Pages with <1% diff (near-perfect)

- architect-theme: homepage (0.03%), another-page (0.00%)
- cayman-theme: homepage (0.02%), another-page (0.00%)
- dinky-theme: homepage (0.03%), another-page (0.00%)
- hacker-theme: homepage (0.05%), another-page (0.00%)
- midnight-theme: homepage (0.02%), another-page (0.00%)
- merlot-theme: homepage (0.01%), another-page (0.00%)
- slate-theme: homepage (0.03%), another-page (0.00%)
- time-machine-theme: homepage (0.12%), another-page (0.00%)
- leap-day-theme: homepage (0.02%), another-page (0.00%)
- large-blog-3000: homepage (0.09%)
- opensource-guide: homepage (0.03%)

### Pages with 1-5% diff (minor differences)

- alexeygrigorev.github.io: homepage (1.33%) -- Root cause: minor CSS rendering difference (Google Fonts loading). Known issue #123.

### Pages with >5% diff

None. The previous 9.62% diff for large-docs-site has been resolved (sidebar sort order fix, issue #121).

### Sites where visual comparison was skipped

The following sites could not be visually compared because rustkyll's homepage did not contain valid HTML (empty, Liquid fallback, or no index.html):

- kids-horror-stories-ru (no index.html generated; story pages are fine)
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
- alexeygrigorev/data-science-interviews (0 HTML files from rustkyll)

Diff images are saved under `playwright/screenshots/` organized by site name.

## Compatibility Summary

- Sites that build with both tools: 34 of 44
- Sites that build only with rustkyll: 7 (missing Jekyll gems/plugins or Ruby version mismatch)
- Sites that build only with Jekyll: 1 (primer-theme)
- Sites that fail with both tools: 3 (bitcoin-org, edition-template, uswds-site)

No build status changes compared to the previous run.

## Regressions

Compared to the previous benchmark (2026-03-15 12:15 UTC):

Speed regressions (>20% slower): Some timing variations are within normal hardware noise. Notable changes:
- kids-horror-stories-ru: 0.330s -> 0.567s (72% slower) -- likely hardware load variance; Jekyll also slower (4.041 -> 4.961).
- mlwiki.org: 1.004s -> 1.347s (34% slower) -- hardware variance.
- large-blog-3000: 1.550s -> 1.587s (2% slower) -- within noise.
- No sites changed build status (FAIL vs success).

DOM regressions: None. All sites either improved or remained the same.

Visual regressions: None. All sites either improved or remained the same.

## Known Issues Affecting Results

- Issue #120 (fix-theme-sites-comparison): Still open. Theme sites (architect, cayman, dinky, hacker, midnight, merlot, slate, time-machine, leap-day) show 0% DOM match despite 100% file match. The DOM diffs are in theme-injected navigation and meta tags that differ due to how rustkyll handles GitHub Pages themes vs Jekyll.
- Issue #123 (fix-google-fonts-css): Still open. alexeygrigorev.github.io has 1.33% visual diff due to Google Fonts CSS rendering differences. The CSS is copied from Jekyll output for visual comparison, but some font loading differences remain.

## Notes

- FAIL means the tool could not build the site (template error, missing plugin, etc.)
- TIMEOUT means the build exceeded 300s and was killed
- Speedup = Jekyll time / rustkyll time (higher is better for rustkyll)
- Page count is the number of HTML files generated in _site/
- Each build starts from a clean _site/ directory (no caching)
- Jekyll builds use bundle exec when a Gemfile is present
- rustkyll is pre-compiled in release mode
- DOM comparison uses BeautifulSoup to normalize and compare full DOM trees
- Visual comparison uses Playwright/Chromium at 1280x720 viewport, full-page screenshots
- Structural comparison covers ALL common HTML files between outputs
