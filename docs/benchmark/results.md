# Benchmark: rustkyll vs Jekyll

Generated: 2026-04-02 19:32 UTC

Configuration: 3 runs per tool, median wall-clock time reported.
Timeout: 300s per build.

rustkyll version: rustkyll 0.3.0
Jekyll version: jekyll 4.4.1

## Results

| Site | Pages | Jekyll (s) | rustkyll (s) | Speedup |
|------|-------|------------|-------------|---------|
| alexeygrigorev/aihero | 2 | 0.620 | 0.022 | 28.18x |
| alexeygrigorev/alexeygrigorev.github.io | 8 | 0.592 | 0.014 | 42.28x |
| alexeygrigorev/data-science-interviews | 0 | 1.384 | 0.010 | 138.40x |
| alexeygrigorev/kids-horror-stories-ru | 1344 | 3.850 | 0.403 | 9.55x |
| alexeygrigorev/little-book-of-metals-ru | 48 | 2.331 | 0.650 | 3.58x |
| alexeygrigorev/mlbookcamp-page | 15 | 0.642 | 0.150 | 4.28x |
| alexeygrigorev/mlwiki.org | 645 | 1.025 | 1.862 | .55x |
| alexeygrigorev/snippets | 25 | 0.667 | 0.089 | 7.49x |
| DataTalksClub/courses | 5 | 0.653 | 0.026 | 25.11x |
| DataTalksClub/datatalksclub.github.io | 790 | 19.609 | 0.632 | 31.02x |
| DataTalksClub/docs | 57 | 1.844 | 0.489 | 3.77x |
| academicpages | 45 | 4.546 | 0.086 | 52.86x |
| al-folio | 102 | 17.657 | 0.383 | 46.10x |
| architect-theme | 2 | 0.629 | 0.020 | 31.45x |
| basically-basic | 13 | 0.748 | 0.045 | 16.62x |
| beautiful-jekyll | 6 | 0.849 | 0.031 | 27.38x |
| bitcoin-org | 142 | TIMEOUT | 0.525 | N/A |
| cayman-theme | 2 | 0.639 | 0.020 | 31.95x |
| chirpy | 17 | 0.890 | 0.122 | 7.29x |
| choosealicense.com | 72 | FAIL | 0.059 | N/A |
| dinky-theme | 2 | 0.632 | 0.020 | 31.60x |
| documentation-theme-jekyll | 100 | 3.676 | 0.255 | 14.41x |
| edition-template | 13 | 0.661 | 0.016 | 41.31x |
| government-github | 21 | 4.420 | 23.237 | .19x |
| hacker-theme | 2 | 0.633 | 0.020 | 31.65x |
| homebrew-site | 134 | 1.320 | 0.080 | 16.50x |
| hyde | 6 | 1.035 | 0.025 | 41.40x |
| hydeout | 34 | 1.394 | 0.055 | 25.34x |
| jasper2 | 29 | 1.075 | 0.048 | 22.39x |
| jekyll-docs/docs | 131 | 3.138 | 1.620 | 1.93x |
| jekyll-theme-chirpy | 17 | 0.898 | 0.126 | 7.12x |
| jekyll-vitepress-theme | 17 | 0.937 | 0.130 | 7.20x |
| just-the-docs | 47 | 2.157 | 0.489 | 4.41x |
| lanyon | 6 | 0.757 | 0.020 | 37.85x |
| large-blog-3000 | 3001 | 4.522 | 1.032 | 4.38x |
| large-docs-site | 801 | 23.915 | 0.701 | 34.11x |
| leap-day-theme | 2 | 0.647 | 0.021 | 30.80x |
| made-mistakes-jekyll | 1039 | 62.916 | 0.980 | 64.20x |
| mediumish | 24 | 0.963 | 0.075 | 12.84x |
| merlot-theme | 2 | 0.643 | 0.021 | 30.61x |
| midnight-theme | 2 | 0.653 | 0.021 | 31.09x |
| minima | 9 | 0.878 | 0.032 | 27.43x |
| minimal-mistakes | 32 | FAIL | 0.115 | N/A |
| mojombo-blog | 17 | 2.237 | 0.037 | 60.45x |
| muan-blog | 2219 | 16.191 | 0.605 | 26.76x |
| opensource-guide | 390 | 15.758 | 0.578 | 27.26x |
| primer-theme | 2 | 1.075 | 0.037 | 29.05x |
| programming-historian | 653 | TIMEOUT | 10.002 | N/A |
| slate-theme | 2 | 0.646 | 0.020 | 32.30x |
| so-simple-theme | 11 | 1.495 | 0.059 | 25.33x |
| text-theme | 11 | 0.915 | 0.097 | 9.43x |
| time-machine-theme | 2 | 0.639 | 0.020 | 31.95x |
| type-theme | 8 | 2.185 | 0.021 | 104.04x |
| uswds-site | 764 | 43.869 | 1.781 | 24.63x |
| wtf-html-css | 1 | 0.539 | 0.037 | 14.56x |
| yat | 20 | 1.960 | 0.102 | 19.21x |

## Notes

- FAIL means the tool could not build the site (template error, missing plugin, etc.)
- TIMEOUT means the build exceeded 300s and was killed
- Speedup = Jekyll time / rustkyll time (higher is better for rustkyll)
- Page count is the number of HTML files generated in _site/
- Each build starts from a clean _site/ directory (no caching)
- Jekyll builds use bundle exec when a Gemfile is present
- rustkyll is pre-compiled in release mode
