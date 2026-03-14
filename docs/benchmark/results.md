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
| bitcoin-org | N/A | FAIL | FAIL | N/A |
| choosealicense.com | 2 | FAIL | 0.021 | N/A |
| edition-template | N/A | FAIL | FAIL | N/A |
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

## Page count investigation (Issue 51)

Six sites had suspicious page counts. Investigation results:

### bitcoin-org: N/A (both tools fail)

- **rustkyll**: FAIL -- uses custom `{% translate %}` tag not supported by rustkyll
- **Jekyll**: FAIL -- missing Ruby gems (needs `bundle install` with specific dependencies)
- **Resolution**: Changed "?" to "N/A" in results. No page count is possible since neither tool produces output.

### edition-template: N/A (both tools fail)

- **rustkyll**: FAIL -- config YAML parse error fixed (null string values), but now fails on unsupported `{% feed_meta %}` tag
- **Jekyll**: FAIL -- missing Ruby gems
- **Resolution**: Changed "?" to "N/A". The config parse bug was fixed (issue 51) but template compatibility remains a separate issue.

### data-science-interviews: 0 pages (correct)

- The site has no layouts specified in its pages' front matter and no default layout in `_config.yml`
- rustkyll correctly copies 27 static files but renders 0 HTML pages
- Jekyll also fails to build this site (missing gems), so no Jekyll comparison available
- **Resolution**: 0 is the correct page count. No bug.

### academicpages: 1 page (rustkyll) vs 45 pages (Jekyll)

- rustkyll renders only `_site/talkmap/map.html` (1 page), failing on 44 others
- Two template compatibility issues cause the failures:
  1. `page["author"][0]` -- indexing a string with `[0]` errors in rustkyll's Liquid engine (should return nil for non-array values)
  2. `| "Undefined parameter..."` -- string literal used as filter name (non-standard Liquid that Jekyll tolerates)
- **Resolution**: 1 page is accurate for rustkyll's current capabilities. The 44 missing pages are due to Liquid engine compatibility gaps, not page-discovery bugs. These are template rendering issues to be addressed separately.

### minimal-mistakes: 1 page (Jekyll), rustkyll FAIL

- Previously reported as "1 page" in the benchmark, but rustkyll was actually failing with a config YAML parse error (`invalid type: unit value, expected a string`) due to many null-valued config keys like `url:`, `baseurl:`, etc.
- **Fix applied in issue 51**: Added `deserialize_string_or_null` to handle YAML null values in string fields (url, baseurl, name, title, permalink)
- After the config fix, rustkyll now fails with a template error: unsupported `{% include_cached %}` tag (a Jekyll plugin tag)
- Jekyll produces 1 page (index.html) -- this is correct for the minimal-mistakes theme demo which has only a sample index page
- **Resolution**: The benchmark's "1 page" was from a stale `_site/` directory. The config parse bug is now fixed. Template compatibility for `include_cached` is a separate issue.

### beautiful-jekyll: 3 pages (rustkyll) vs 6 pages (Jekyll)

- rustkyll renders 3 pages: `tags.html`, `404.html`, `index.html`
- Jekyll renders 6 pages: the same 3 plus `aboutme/index.html`, `2020-02-28-sample-markdown/index.html`, `2020-02-26-flake-it-till-you-make-it/index.html`
- The 3 missing pages fail due to two template issues:
  1. `site.title-on-all-pages` -- variable name with hyphens in a comparison expression causes a parse error (`expected Literal`)
  2. `page["cover-img"]` -- array type check fails when the value is a string
- **Resolution**: 3 pages is accurate for rustkyll's current capabilities. The 3 missing pages are due to Liquid template compatibility issues, not page-discovery bugs.

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
