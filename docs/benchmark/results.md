# Benchmark: rustkyll vs Jekyll

Generated: 2026-03-19 04:38 UTC

Configuration: 3 runs per tool, median wall-clock time reported.
Timeout: 300s per build.

rustkyll version: rustkyll 0.2.3
Jekyll version: jekyll 4.4.1

## Results

| Site | Pages | Jekyll (s) | rustkyll (s) | Speedup |
|------|-------|------------|-------------|---------|
| alexeygrigorev/aihero | 2 | 0.612 | 0.024 | 25.50x |
| alexeygrigorev/alexeygrigorev.github.io | 8 | 0.564 | 0.015 | 37.60x |
| alexeygrigorev/data-science-interviews | 0 | 1.405 | 0.010 | 140.50x |
| alexeygrigorev/kids-horror-stories-ru | 1344 | 3.830 | 0.366 | 10.46x |
| alexeygrigorev/little-book-of-metals-ru | 43 | 2.350 | 0.562 | 4.18x |
| alexeygrigorev/mlbookcamp-page | 15 | 0.610 | 0.148 | 4.12x |
| alexeygrigorev/mlwiki.org | 640 | 0.980 | 1.787 | .54x |
| alexeygrigorev/snippets | 25 | 0.648 | 0.074 | 8.75x |
| DataTalksClub/courses | 5 | 0.606 | 0.032 | 18.93x |
| DataTalksClub/datatalksclub.github.io | 787 | 19.748 | 1.596 | 12.37x |
| DataTalksClub/docs | 57 | 2.387 | 0.618 | 3.86x |
| academicpages | 17 | 6.394 | 0.052 | 122.96x |
| architect-theme | 2 | 0.676 | 0.018 | 37.55x |
| beautiful-jekyll | 6 | 0.837 | 0.030 | 27.90x |
| bitcoin-org | N/A | FAIL | FAIL | N/A |
| cayman-theme | 2 | 0.683 | 0.018 | 37.94x |
| choosealicense.com | 72 | FAIL | 0.069 | N/A |
| dinky-theme | 2 | 0.616 | 0.018 | 34.22x |
| documentation-theme-jekyll | 100 | 3.728 | 0.248 | 15.03x |
| edition-template | 13 | FAIL | 0.013 | N/A |
| government-github | 21 | 4.354 | 15.012 | .29x |
| hacker-theme | 2 | 0.655 | 0.018 | 36.38x |
| homebrew-site | 134 | FAIL | 0.052 | N/A |
| hyde | 6 | FAIL | 0.022 | N/A |
| jekyll-docs/docs | 131 | 3.017 | 2.245 | 1.34x |
| just-the-docs | 47 | 2.186 | 0.318 | 6.87x |
| large-blog-3000 | 3001 | 4.497 | 1.092 | 4.11x |
| large-docs-site | 801 | 24.441 | 0.688 | 35.52x |
| leap-day-theme | 2 | 0.677 | 0.018 | 37.61x |
| made-mistakes-jekyll | 2 | FAIL | 0.011 | N/A |
| merlot-theme | 2 | 0.621 | 0.019 | 32.68x |
| midnight-theme | 2 | 0.678 | 0.018 | 37.66x |
| minima | 9 | FAIL | 0.026 | N/A |
| minimal-mistakes | 1 | FAIL | 0.045 | N/A |
| mojombo-blog | 17 | 2.767 | 0.109 | 25.38x |
| muan-blog | 2219 | 16.757 | 0.801 | 20.92x |
| opensource-guide | 390 | 15.881 | 0.456 | 34.82x |
| primer-theme | 2 | 1.065 | 0.018 | 59.16x |
| programming-historian | 653 | FAIL | 10.824 | N/A |
| slate-theme | 2 | 0.671 | 0.017 | 39.47x |
| so-simple-theme | 11 | 1.485 | 0.050 | 29.70x |
| time-machine-theme | 2 | 0.618 | 0.018 | 34.33x |
| uswds-site | 764 | FAIL | 1.010 | N/A |
| wtf-html-css | 1 | FAIL | 0.033 | N/A |

## Notes

- FAIL means the tool could not build the site (template error, missing plugin, etc.)
- TIMEOUT means the build exceeded 300s and was killed
- Speedup = Jekyll time / rustkyll time (higher is better for rustkyll)
- Page count is the number of HTML files generated in _site/
- Each build starts from a clean _site/ directory (no caching)
- Jekyll builds use bundle exec when a Gemfile is present
- rustkyll is pre-compiled in release mode
