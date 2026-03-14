# Benchmark: rustkyll vs Jekyll

Generated: 2026-03-14 07:33 UTC

Configuration: 3 runs per tool, median wall-clock time reported.
Timeout: 300s per build.

rustkyll version: rustkyll 0.1.0
Jekyll version: jekyll 4.4.1

## Results

| Site | Pages | Jekyll (s) | rustkyll (s) | Speedup |
|------|-------|------------|-------------|---------|
| alexeygrigorev/aihero | 2 | FAIL | 0.018 | N/A |
| alexeygrigorev/alexeygrigorev.github.io | 8 | 0.554 | 0.011 | 50.36x |
| alexeygrigorev/data-science-interviews | 0 | FAIL | 0.009 | N/A |
| alexeygrigorev/kids-horror-stories-ru | 1345 | 3.820 | 72.379 | 0.05x |
| alexeygrigorev/little-book-of-metals-ru | 1 | 2.303 | 0.014 | 164.50x |
| alexeygrigorev/mlbookcamp-page | 15 | FAIL | 0.029 | N/A |
| alexeygrigorev/mlwiki.org | 2 | 0.980 | 0.008 | 122.50x |
| alexeygrigorev/snippets | 2 | 0.647 | 0.007 | 92.42x |
| DataTalksClub/courses | 5 | FAIL | 0.015 | N/A |
| DataTalksClub/datatalksclub.github.io | 787 | 19.420 | TIMEOUT | N/A |
| DataTalksClub/docs | 57 | 1.793 | FAIL | N/A |
| academicpages | 1 | 4.405 | 0.357 | 12.33x |
| beautiful-jekyll | 3 | 0.829 | 0.152 | 5.45x |
| bitcoin-org | ? | FAIL | FAIL | N/A |
| choosealicense.com | 2 | FAIL | 0.021 | N/A |
| edition-template | ? | FAIL | FAIL | N/A |
| government-github | 13 | FAIL | 0.064 | N/A |
| hyde | 5 | FAIL | 0.010 | N/A |
| jekyll-docs/docs | 228 | 3.039 | FAIL | N/A |
| minima | 1 | FAIL | 0.018 | N/A |
| minimal-mistakes | 1 | 0.900 | FAIL | N/A |
| opensource-guide | 4 | FAIL | 0.782 | N/A |
| so-simple-theme | 66 | 1.466 | FAIL | N/A |
| wtf-html-css | 1 | FAIL | 0.009 | N/A |

## Sites where both tools succeeded

| Site | Pages | Jekyll (s) | rustkyll (s) | Speedup |
|------|-------|------------|-------------|---------|
| alexeygrigorev/alexeygrigorev.github.io | 8 | 0.554 | 0.011 | 50.36x |
| alexeygrigorev/kids-horror-stories-ru | 1345 | 3.820 | 72.379 | 0.05x |
| alexeygrigorev/little-book-of-metals-ru | 1 | 2.303 | 0.014 | 164.50x |
| alexeygrigorev/mlwiki.org | 2 | 0.980 | 0.008 | 122.50x |
| alexeygrigorev/snippets | 2 | 0.647 | 0.007 | 92.42x |
| academicpages | 1 | 4.405 | 0.357 | 12.33x |
| beautiful-jekyll | 3 | 0.829 | 0.152 | 5.45x |

For small sites (under 10 pages), rustkyll is 5x-164x faster than Jekyll. The speedup comes from avoiding Ruby startup overhead and using compiled code.

For the large site kids-horror-stories-ru (1345 pages), rustkyll is 20x slower than Jekyll (72s vs 3.8s). This indicates a performance bottleneck in rustkyll's template rendering that scales poorly with page count.

The primary reference site (DataTalksClub/datatalksclub.github.io, 787 pages) timed out at 300s with rustkyll while Jekyll completed in 19.4s. This is likely the same scalability issue as kids-horror-stories-ru.

## Compatibility summary

- Sites that build with both tools: 7 of 24
- Sites that build only with rustkyll: 9 (missing Jekyll gems or incompatible Jekyll version)
- Sites that build only with Jekyll: 3 (missing rustkyll features)
- Sites that fail with both tools: 2 (bitcoin-org, edition-template)
- Sites where rustkyll times out: 1 (DTC main site)

## Notes

- FAIL means the tool could not build the site (template error, missing plugin, etc.)
- TIMEOUT means the build exceeded 300s and was killed
- Speedup = Jekyll time / rustkyll time (higher is better for rustkyll)
- Page count is the number of HTML files generated in _site/
- Each build starts from a clean _site/ directory (no caching)
- Jekyll builds use bundle exec when a Gemfile is present
- rustkyll is pre-compiled in release mode
- Jekyll FAIL entries are often due to missing Ruby gems (bundle install not run) rather than tool limitations
- The large-site performance regression suggests rustkyll has O(n^2) or worse scaling in template rendering
