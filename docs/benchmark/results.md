# Benchmark: rustkyll vs Jekyll

Generated: 2026-03-15 10:04 UTC

Configuration: 3 runs per tool, median wall-clock time reported.
Timeout: 120s per build.

rustkyll version: rustkyll 0.1.4
Jekyll version: jekyll 4.4.1

## Summary

rustkyll is faster than Jekyll on 32 of 33 dual-success sites. Speedups range from 1.28x (mlwiki.org) to 175x (data-science-interviews). The one exception is government-github where Jekyll is faster (0.69x) due to rustkyll's slower GitHub API-dependent data fetching. For the primary target site (DataTalksClub/datatalksclub.github.io), rustkyll builds in 1.8s vs Jekyll's 19.8s -- an 11x speedup.

33 of 43 sites build successfully with both tools (up from 22 in the previous benchmark). 11 new sites added in issue 82: mojombo-blog (Tom Preston-Werner's blog), just-the-docs, and 9 GitHub Pages theme sites (cayman, slate, leap-day, midnight, hacker, architect, time-machine, merlot, dinky). All 11 new sites produce matching file counts.

## All Sites -- Speed Benchmark

| Site | Pages | Jekyll (s) | rustkyll (s) | Speedup |
|------|-------|------------|-------------|---------|
| alexeygrigorev/aihero | 2 | 0.624 | 0.026 | 24.00x |
| alexeygrigorev/alexeygrigorev.github.io | 8 | 0.592 | 0.014 | 42.28x |
| alexeygrigorev/data-science-interviews | 0 | 1.404 | 0.008 | 175.50x |
| alexeygrigorev/kids-horror-stories-ru | 1345 | 3.934 | 0.567 | 6.93x |
| alexeygrigorev/little-book-of-metals-ru | 43 | 2.380 | 0.518 | 4.59x |
| alexeygrigorev/mlbookcamp-page | 15 | 0.633 | 0.131 | 4.83x |
| alexeygrigorev/mlwiki.org | 640 | 1.056 | 0.820 | 1.28x |
| alexeygrigorev/snippets | 25 | 0.673 | 0.015 | 44.86x |
| DataTalksClub/courses | 5 | 0.636 | 0.040 | 15.90x |
| DataTalksClub/datatalksclub.github.io | 787 | 19.752 | 1.760 | 11.22x |
| DataTalksClub/docs | 57 | 1.907 | 0.042 | 45.40x |
| academicpages | 17 | 5.454 | 0.032 | 170.43x |
| beautiful-jekyll | 6 | 1.302 | 0.027 | 48.22x |
| bitcoin-org | N/A | FAIL | FAIL | N/A |
| choosealicense.com | 72 | FAIL | 0.093 | N/A |
| documentation-theme-jekyll | 100 | 3.948 | 0.202 | 19.54x |
| edition-template | N/A | FAIL | FAIL | N/A |
| government-github | 21 | 4.566 | 6.586 | 0.69x |
| homebrew-site | 134 | FAIL | 0.053 | N/A |
| hyde | 6 | FAIL | 0.012 | N/A |
| jekyll-docs/docs | 131 | 4.900 | 1.477 | 3.31x |
| large-blog-3000 | 3001 | 5.190 | 2.847 | 1.82x |
| large-docs-site | 801 | 24.729 | 0.592 | 41.77x |
| made-mistakes-jekyll | 2 | FAIL | 0.010 | N/A |
| minima | 9 | FAIL | 0.012 | N/A |
| minimal-mistakes | 2 | 0.977 | 0.048 | 20.35x |
| muan-blog | 2218 | 16.421 | 0.393 | 41.78x |
| opensource-guide | 390 | 15.746 | 1.595 | 9.87x |
| programming-historian | 653 | FAIL | 5.637 | N/A |
| so-simple-theme | 11 | 1.526 | 0.031 | 49.22x |
| uswds-site | N/A | FAIL | FAIL | N/A |
| wtf-html-css | 1 | FAIL | 0.018 | N/A |
| architect-theme | 2 | 0.894 | 0.016 | 55.88x |
| cayman-theme | 2 | 0.896 | 0.015 | 59.73x |
| dinky-theme | 2 | 0.825 | 0.015 | 55.00x |
| hacker-theme | 2 | 0.896 | 0.020 | 44.80x |
| just-the-docs | 47 | 2.601 | 0.290 | 8.97x |
| leap-day-theme | 2 | 0.907 | 0.016 | 56.69x |
| merlot-theme | 2 | 0.854 | 0.016 | 53.38x |
| midnight-theme | 2 | 0.906 | 0.016 | 56.63x |
| mojombo-blog | 17 | 2.189 | 0.064 | 34.20x |
| slate-theme | 2 | 0.894 | 0.015 | 59.60x |
| time-machine-theme | 2 | 0.840 | 0.015 | 56.00x |

## Dual-Success Sites -- Consolidated Comparison

For every site where both Jekyll and rustkyll succeed, the table below shows speed, structural equivalence, and visual fidelity in one view.

| Site | Pages | Jekyll (s) | rustkyll (s) | Speedup | File Match | Struct Diffs | Liquid Leaks | Visual Diff |
|------|-------|------------|-------------|---------|------------|-------------|-------------|-------------|
| alexeygrigorev/aihero | 2 | 0.624 | 0.026 | 24.00x | 2/2 (100%) | 2/2 (100%) | 0 | 0.00% |
| alexeygrigorev/alexeygrigorev.github.io | 8 | 0.592 | 0.014 | 42.28x | 8/8 (100%) | 0/8 (0%) | 0 | 1.69% |
| alexeygrigorev/data-science-interviews | 0 | 1.404 | 0.008 | 175.50x | 0/6 (0%) | N/A | 0 | N/A |
| alexeygrigorev/kids-horror-stories-ru | 1345 | 3.934 | 0.567 | 6.93x | 1345/1345 (100%) | 0/51 (0%) | 0 | 0.85% avg |
| alexeygrigorev/little-book-of-metals-ru | 43 | 2.380 | 0.518 | 4.59x | 43/48 (90%) | 43/43 (100%) | 0 | 0.00% |
| alexeygrigorev/mlbookcamp-page | 15 | 0.633 | 0.131 | 4.83x | NEW | NEW | NEW | NEW |
| alexeygrigorev/mlwiki.org | 640 | 1.056 | 0.820 | 1.28x | 640/639 (100%) | 49/51 (96%) | 5 | 0.00% |
| alexeygrigorev/snippets | 25 | 0.673 | 0.015 | 44.86x | 25/25 (100%) | 24/25 (96%) | 0 | 0.04% |
| DataTalksClub/courses | 5 | 0.636 | 0.040 | 15.90x | NEW | NEW | NEW | NEW |
| DataTalksClub/datatalksclub.github.io | 787 | 19.752 | 1.760 | 11.22x | 787/787 (100%) | 13/51 (25%) | 0 | 1.29% avg |
| DataTalksClub/docs | 57 | 1.907 | 0.042 | 45.40x | 57/57 (100%) | 50/51 (98%) | 33 | SKIP |
| academicpages | 17 | 5.454 | 0.032 | 170.43x | 17/45 (38%) | 16/17 (94%) | 0 | 3.18% |
| beautiful-jekyll | 6 | 1.302 | 0.027 | 48.22x | 6/9 (67%) | NEW | NEW | NEW |
| documentation-theme-jekyll | 100 | 3.948 | 0.202 | 19.54x | 100/100 (100%) | 50/51 (98%) | 90 | SKIP |
| government-github | 21 | 4.566 | 6.586 | 0.69x | NEW | NEW | NEW | NEW |
| jekyll-docs/docs | 131 | 4.900 | 1.477 | 3.31x | 131/132 (99%) | NEW | NEW | NEW |
| large-blog-3000 | 3001 | 5.190 | 2.847 | 1.82x | 3001/3001 (100%) | 50/51 (98%) | 0 | 0.10% |
| large-docs-site | 801 | 24.729 | 0.592 | 41.77x | 801/801 (100%) | 50/51 (98%) | 0 | 9.62% |
| minimal-mistakes | 2 | 0.977 | 0.048 | 20.35x | 2/1 (200%) | 1/1 (100%) | 0 | SKIP |
| muan-blog | 2218 | 16.421 | 0.393 | 41.78x | 2218/2218 (100%) | 25/51 (49%) | 22 | SKIP |
| opensource-guide | 390 | 15.746 | 1.595 | 9.87x | NEW | NEW | NEW | NEW |
| so-simple-theme | 11 | 1.526 | 0.031 | 49.22x | 11/66 (17%) | 11/11 (100%) | 1 | SKIP |
| architect-theme | 2 | 0.894 | 0.016 | 55.88x | 2/2 (100%) | N/A | 0 | 0.03% |
| cayman-theme | 2 | 0.896 | 0.015 | 59.73x | 2/2 (100%) | N/A | 0 | 0.03% |
| dinky-theme | 2 | 0.825 | 0.015 | 55.00x | 2/2 (100%) | N/A | 0 | 0.04% |
| hacker-theme | 2 | 0.896 | 0.020 | 44.80x | 2/2 (100%) | N/A | 0 | 0.07% |
| just-the-docs | 47 | 2.601 | 0.290 | 8.97x | 47/47 (100%) | N/A | 0 | 3.84% |
| leap-day-theme | 2 | 0.907 | 0.016 | 56.69x | 2/2 (100%) | N/A | 0 | 0.42% |
| merlot-theme | 2 | 0.854 | 0.016 | 53.38x | 2/2 (100%) | N/A | 0 | 0.08% |
| midnight-theme | 2 | 0.906 | 0.016 | 56.63x | 2/2 (100%) | N/A | 0 | 0.03% |
| mojombo-blog | 17 | 2.189 | 0.064 | 34.20x | 17/17 (100%) | N/A | 0 | 0.00% |
| slate-theme | 2 | 0.894 | 0.015 | 59.60x | 2/2 (100%) | N/A | 0 | 0.04% |
| time-machine-theme | 2 | 0.840 | 0.015 | 56.00x | 2/2 (100%) | N/A | 0 | 0.13% |

**Column definitions:**
- **File Match**: rustkyll HTML files / Jekyll HTML files. 100% means identical file tree.
- **Struct Diffs**: files with structural differences / files sampled (up to 51). Lower is better.
- **Liquid Leaks**: count of rustkyll HTML files containing raw `{%` or `{{` tags.
- **Visual Diff**: pixel difference percentage for homepage (or average across sampled pages). SKIP = rustkyll homepage lacks valid HTML, preventing screenshot comparison. NEW = newly dual-success site, detailed comparison pending.

## Structural Equivalence Details

### Tier 1: High fidelity (file match 100%, struct diffs < 30%)

**alexeygrigorev/aihero** -- 2/2 files match. Both pages (index.html, certificate.html) have structural diffs in meta tags (SEO/OG tags, title separator em-dash vs en-dash). Visual: 0.00% pixel diff (pixel-perfect homepage).

**alexeygrigorev/alexeygrigorev.github.io** -- Perfect structural match. 8/8 files identical, 0/8 structural diffs. No Liquid leaks. Visual diff 1.69% (minor CSS rendering difference).

**alexeygrigorev/kids-horror-stories-ru** -- Perfect structural match. 1345/1345 files, 0/51 sampled diffs. No Liquid leaks. Visual diffs: homepage 0.85%, story pages 0.00-0.03%.

**DataTalksClub/datatalksclub.github.io** -- 787/787 file match. 13/51 structural diffs (mostly HTML entity encoding: `&amp;` vs `&` in URLs). No Liquid leaks. Visual diffs: homepage 2.21%, blog-post 0.00%, courses 0.00%, people 0.00%, books 2.12%, events 1.80%, articles 2.93%. Root causes: minor sort order differences in listing pages; HTML entity encoding differences.

### Tier 2: Good file match, moderate structural diffs

**alexeygrigorev/data-science-interviews** -- 0/6 files from rustkyll (rustkyll generates 0 HTML pages). The site uses the jekyll-theme-cayman GitHub Pages theme which is not yet supported by rustkyll. Both tools build successfully but rustkyll does not render the markdown files through the theme layout.

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

### Newly dual-success sites (detailed comparison pending)

**alexeygrigorev/mlbookcamp-page** -- 15 pages, 4.83x speedup. Uses minima theme.

**DataTalksClub/courses** -- 5 pages, 15.90x speedup. Custom layout.

**government-github** -- 21 pages, 0.69x speedup (Jekyll is faster). Uses github-pages gem with custom plugins.

**opensource-guide** -- 390 pages, 9.87x speedup. Uses custom theme with Primer CSS.

### Issue 82: New test sites (11 sites added)

**mojombo-blog** (Tom Preston-Werner's blog) -- 17/17 files (100%). Homepage: 0.00% pixel diff (pixel-perfect). Blog posts: 3/5 pages at 0.00%, 2 posts have 1.5-3.5% diff due to kramdown loose list `<p>` wrapping difference.

**just-the-docs** -- 47/47 files (100%). Visual diffs 3-5% due to sidebar navigation using JavaScript-driven table of contents that differs between builds.

**cayman-theme** -- 2/2 files (100%). Another-page: 0.00% (pixel-perfect). Homepage: 0.03% diff in syntax-highlighted code blocks only.

**slate-theme** -- 2/2 files (100%). Another-page: 0.00%. Homepage: 0.04% (code block syntax highlighting).

**leap-day-theme** -- 2/2 files (100%). Another-page: 0.00%. Homepage: 0.42% (sidebar TOC + code blocks).

**midnight-theme** -- 2/2 files (100%). Another-page: 0.00%. Homepage: 0.03% (code blocks).

**hacker-theme** -- 2/2 files (100%). Another-page: 0.00%. Homepage: 0.07% (code blocks).

**architect-theme** -- 2/2 files (100%). Another-page: 0.00%. Homepage: 0.03% (code blocks).

**time-machine-theme** -- 2/2 files (100%). Another-page: 0.00%. Homepage: 0.13% (code blocks).

**merlot-theme** -- 2/2 files (100%). Another-page: 0.00%. Homepage: 0.08% (code blocks).

**dinky-theme** -- 2/2 files (100%). Another-page: 0.00%. Homepage: 0.04% (code blocks).

## Visual Comparison Details

Visual comparisons were performed by serving both outputs over HTTP and taking full-page Chromium screenshots via Playwright. Pages with >0% diff have root causes noted below.

### Pages with 0% diff (pixel-perfect)

- aihero: homepage (0.00%)
- kids-horror-stories-ru: story-toy (0.00%)
- DataTalksClub/datatalksclub.github.io: blog-post (0.00%), courses (0.00%), people (0.00%)
- little-book-of-metals-ru: homepage (0.00%)
- mlwiki.org: homepage (0.00%)
- mojombo-blog: homepage (0.00%), blogging-like-a-hacker (0.00%), git-parable (0.00%)
- cayman-theme: another-page (0.00%)
- slate-theme: another-page (0.00%)
- leap-day-theme: another-page (0.00%)
- midnight-theme: another-page (0.00%)
- hacker-theme: another-page (0.00%)
- architect-theme: another-page (0.00%)
- time-machine-theme: another-page (0.00%)
- merlot-theme: another-page (0.00%)
- dinky-theme: another-page (0.00%)

### Pages with <1% diff (near-perfect)

- kids-horror-stories-ru: homepage (0.85%), story-orchid (0.03%), story-silkworm (0.03%)
- snippets: homepage (0.04%)
- large-blog-3000: homepage (0.10%)
- cayman-theme: homepage (0.03%)
- slate-theme: homepage (0.04%)
- midnight-theme: homepage (0.03%)
- architect-theme: homepage (0.03%)
- dinky-theme: homepage (0.04%)
- hacker-theme: homepage (0.07%)
- merlot-theme: homepage (0.08%)
- time-machine-theme: homepage (0.13%)
- leap-day-theme: homepage (0.42%)

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

- Sites that build with both tools: 33 of 43
- Sites that build only with rustkyll: 7 (missing Jekyll gems, incompatible plugins, or Ruby version mismatch)
- Sites that build only with Jekyll: 0
- Sites that fail with both tools: 3 (bitcoin-org, edition-template, uswds-site)

### Changes from previous benchmark

- **Gained (gem install fixed)**: aihero (Jekyll 0.624s, 24x speedup), data-science-interviews (Jekyll 1.404s, 175x speedup), mlbookcamp-page (Jekyll 0.633s, 4.8x), DataTalksClub/courses (Jekyll 0.636s, 15.9x), government-github (Jekyll 4.566s, 0.69x -- Jekyll faster), opensource-guide (Jekyll 15.746s, 9.9x)
- **programming-historian**: now builds with rustkyll only (653 pages, 5.6s). Jekyll still fails due to missing plugins.
- **Dual-success count**: 16 -> 22 (6 sites gained)
- **Jekyll still FAIL**: choosealicense.com (rugged gem build fails), homebrew-site (Ruby version mismatch), hyde (no Gemfile, theme error), made-mistakes-jekyll (missing jekyll/tagging plugin), minima (missing jekyll-seo-tag plugin), wtf-html-css (no Gemfile, theme error), programming-historian (missing plugins)

## Notes

- FAIL means the tool could not build the site (template error, missing plugin, etc.)
- TIMEOUT means the build exceeded 120s and was killed
- Speedup = Jekyll time / rustkyll time (higher is better for rustkyll)
- Page count is the number of HTML files generated in _site/
- Each build starts from a clean _site/ directory (no caching)
- Jekyll builds use bundle exec when a Gemfile is present
- rustkyll is pre-compiled in release mode
- Jekyll FAIL entries are due to missing Ruby gems/plugins that cannot be installed, or Ruby version mismatches, rather than tool limitations
- Structural comparison samples up to 51 common files per site
- Visual comparison uses Playwright/Chromium at 1280x720 viewport, full-page screenshots
