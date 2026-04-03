# Benchmark: rustkyll vs Jekyll

Generated: 2026-04-03 07:12 UTC

Configuration: 3 runs per tool, median wall-clock time reported.
Timeout: 300s per build.

rustkyll version: rustkyll 0.3.0
Jekyll version: jekyll 4.4.1

## Results

| Site | Pages | Jekyll (s) | rustkyll (s) | Speedup |
|------|-------|------------|-------------|---------|
| large-blog-3000 | 3001 | 4.429 | 0.960 | 4.61x |

## Notes

- FAIL means the tool could not build the site (template error, missing plugin, etc.)
- TIMEOUT means the build exceeded 300s and was killed
- Speedup = Jekyll time / rustkyll time (higher is better for rustkyll)
- Page count is the number of HTML files generated in _site/
- Each build starts from a clean _site/ directory (no caching)
- Jekyll builds use bundle exec when a Gemfile is present
- rustkyll is pre-compiled in release mode
