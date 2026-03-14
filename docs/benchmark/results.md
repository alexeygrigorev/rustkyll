# Benchmark: rustkyll vs Jekyll

Generated: 2026-03-14 11:00 UTC

Configuration: 3 runs per tool, median wall-clock time reported.
Timeout: 120s per build.

rustkyll version: rustkyll 0.1.0
Jekyll version: jekyll 4.4.1

## Results

| Site | Pages | Jekyll (s) | rustkyll (s) | Speedup |
|------|-------|------------|-------------|---------|
| alexeygrigorev/aihero | 2 | FAIL | 0.017 | N/A |
| alexeygrigorev/alexeygrigorev.github.io | 8 | 0.566 | 0.010 | 56.60x |
| alexeygrigorev/data-science-interviews | 0 | FAIL | 0.008 | N/A |
| alexeygrigorev/kids-horror-stories-ru | 1345 | 3.816 | 0.297 | 12.84x |
| alexeygrigorev/little-book-of-metals-ru | 1 | 2.299 | 0.014 | 164.21x |
| alexeygrigorev/mlbookcamp-page | 15 | FAIL | 0.016 | N/A |
| alexeygrigorev/mlwiki.org | 2 | 0.971 | 0.009 | 107.88x |
| alexeygrigorev/snippets | 2 | 0.649 | 0.008 | 81.12x |
| DataTalksClub/courses | 5 | FAIL | 0.015 | N/A |
| DataTalksClub/datatalksclub.github.io | 784 | 19.155 | 5.925 | 3.23x |
| DataTalksClub/docs | 1 | 1.805 | 0.016 | 112.81x |
| academicpages | 1 | 4.418 | 0.028 | 157.78x |
| beautiful-jekyll | 3 | 0.804 | 0.022 | 36.54x |
| bitcoin-org | N/A | FAIL | FAIL | N/A |
| choosealicense.com | 2 | FAIL | 0.011 | N/A |
| documentation-theme-jekyll | 8 | 3.618 | 0.017 | 212.82x |
| edition-template | N/A | FAIL | FAIL | N/A |
| government-github | 13 | FAIL | 0.021 | N/A |
| homebrew-site | 53 | 1.212 | 0.027 | 44.88x |
| hyde | 5 | FAIL | 0.008 | N/A |
| jekyll-docs/docs | 7 | 2.974 | 0.060 | 49.56x |
| large-blog-3000 | 1 | 4.279 | 0.393 | 10.88x |
| large-docs-site | 1 | 23.227 | 0.009 | 2580.77x |
| made-mistakes-jekyll | 1 | FAIL | 0.007 | N/A |
| minima | 1 | FAIL | 0.009 | N/A |
| minimal-mistakes | 2 | 0.903 | 0.043 | 21.00x |
| muan-blog | 2218 | 15.818 | FAIL | N/A |
| opensource-guide | 4 | FAIL | 0.018 | N/A |
| programming-historian | 54 | FAIL | 0.264 | N/A |
| so-simple-theme | 2 | 1.439 | 0.014 | 102.78x |
| uswds-site | N/A | FAIL | FAIL | N/A |
| wtf-html-css | 1 | FAIL | 0.009 | N/A |

## Sites where both tools succeeded

| Site | Pages | Jekyll (s) | rustkyll (s) | Speedup |
|------|-------|------------|-------------|---------|
| alexeygrigorev/alexeygrigorev.github.io | 8 | 0.566 | 0.010 | 56.60x |
| alexeygrigorev/kids-horror-stories-ru | 1345 | 3.816 | 0.297 | 12.84x |
| alexeygrigorev/little-book-of-metals-ru | 1 | 2.299 | 0.014 | 164.21x |
| alexeygrigorev/mlwiki.org | 2 | 0.971 | 0.009 | 107.88x |
| alexeygrigorev/snippets | 2 | 0.649 | 0.008 | 81.12x |
| DataTalksClub/datatalksclub.github.io | 784 | 19.155 | 5.925 | 3.23x |
| DataTalksClub/docs | 1 | 1.805 | 0.016 | 112.81x |
| academicpages | 1 | 4.418 | 0.028 | 157.78x |
| beautiful-jekyll | 3 | 0.804 | 0.022 | 36.54x |
| documentation-theme-jekyll | 8 | 3.618 | 0.017 | 212.82x |
| homebrew-site | 53 | 1.212 | 0.027 | 44.88x |
| jekyll-docs/docs | 7 | 2.974 | 0.060 | 49.56x |
| large-blog-3000 | 1 | 4.279 | 0.393 | 10.88x |
| large-docs-site | 1 | 23.227 | 0.009 | 2580.77x |
| minimal-mistakes | 2 | 0.903 | 0.043 | 21.00x |
| so-simple-theme | 2 | 1.439 | 0.014 | 102.78x |

rustkyll is faster than Jekyll on every site where both tools succeed. Speedups range from 3x (DTC main site, 784 pages) to 2581x (large-docs-site, 801 Jekyll pages vs 1 rustkyll page). For sites where rustkyll renders comparable page counts, the speedup is typically 10-160x.

Notable improvement: kids-horror-stories-ru (1345 pages) previously took 72s with rustkyll and now completes in 0.297s (12.8x faster than Jekyll). The DTC main site (784 pages) previously timed out at 300s and now builds in 5.9s.

## New sites added (issue 56)

The following 8 sites were added to expand benchmark coverage:

| Site | Category | Source files | Jekyll pages | Notes |
|------|----------|-------------|-------------|-------|
| documentation-theme-jekyll | Documentation | 136 | 100 | Tom Johnson's technical writing theme |
| homebrew-site | Community/docs | 107 | 134 | Homebrew package manager website |
| large-blog-3000 | Synthetic blog | 3000 posts | 3001 | Generated benchmark site with categories and tags |
| large-docs-site | Synthetic docs | 800 pages | 801 | Generated documentation site across 10 sections |
| muan-blog | Blog/portfolio | 2224 | 2218 | Large personal blog, Jekyll only (rustkyll FAIL) |
| programming-historian | Educational | 610 | N/A | Digital humanities tutorials (Jekyll FAIL: plugin error) |
| made-mistakes-jekyll | Blog | 1123 | N/A | Large blog (Jekyll FAIL: missing jekyll-tagging gem) |
| uswds-site | Government | 864 | N/A | US Web Design System (both FAIL: needs pre-build step) |

Categories represented: documentation, blog, community, educational, government, synthetic benchmarks.

Sites with 100+ pages (from best tool): documentation-theme-jekyll (100), homebrew-site (134), large-blog-3000 (3001), large-docs-site (801), muan-blog (2218) = 5 sites.

Sites where Jekyll takes 5+ seconds: muan-blog (15.8s), large-docs-site (23.2s), DataTalksClub/datatalksclub.github.io (19.2s) = 3 sites.

## Page count discrepancies

The "Pages" column in the results table shows the page count from the first tool that succeeds (rustkyll runs first). For several sites, Jekyll produces significantly more pages than rustkyll due to template compatibility gaps:

- documentation-theme-jekyll: rustkyll 8 pages vs Jekyll 100 pages
- homebrew-site: rustkyll 53 pages vs Jekyll 134 pages
- large-blog-3000: rustkyll 1 page vs Jekyll 3001 pages (rustkyll only renders index)
- large-docs-site: rustkyll 1 page vs Jekyll 801 pages (rustkyll only renders index)

## Compatibility summary

- Sites that build with both tools: 16 of 32
- Sites that build only with rustkyll: 12 (missing Jekyll gems or incompatible Jekyll version)
- Sites that build only with Jekyll: 1 (muan-blog -- template issues in rustkyll)
- Sites that fail with both tools: 3 (bitcoin-org, edition-template, uswds-site)

## Notes

- FAIL means the tool could not build the site (template error, missing plugin, etc.)
- TIMEOUT means the build exceeded 120s and was killed
- Speedup = Jekyll time / rustkyll time (higher is better for rustkyll)
- Page count is the number of HTML files generated in _site/
- Each build starts from a clean _site/ directory (no caching)
- Jekyll builds use bundle exec when a Gemfile is present
- rustkyll is pre-compiled in release mode
- Jekyll FAIL entries are often due to missing Ruby gems (bundle install not run) rather than tool limitations
- Synthetic sites (large-blog-3000, large-docs-site) were created specifically for benchmarking large page counts
