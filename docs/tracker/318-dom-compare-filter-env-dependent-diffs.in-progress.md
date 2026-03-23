# Issue 318: Filter environment-dependent diffs in dom_compare.py

## Problem

choosealicense.com shows 17/72 (24%) match rate, but the vast majority of the
55 "diff" pages have ONLY environment-dependent differences that are not bugs
in rustkyll. When these are excluded, the true match rate is approximately
66/72 (92%).

The environment-dependent diffs fall into three categories:

### Category A: Jekyll version string (55 pages)

Every page has a `<meta>` tag with the Jekyll generator version:
- Cached Jekyll: `content='Jekyll v3.10.0'`
- Rustkyll:      `content='Jekyll v4.4.1'`

This differs because the cached site was built with an older Jekyll version.
Rustkyll reports the current Jekyll version. This is NOT a bug.

### Category B: GitHub Pages URL pattern (149 diffs across 55 pages)

The cached Jekyll site was built on GitHub Pages infrastructure, which uses a
different URL pattern than local builds:
- Cached Jekyll: `https://github.com/pages/github/choosealicense.com/`
- Rustkyll:      `https://github.github.io/choosealicense.com/`

This appears in breadcrumb JSON-LD `@id` values, canonical URLs, and other
`site.github.url`-derived values. Both are correct for their respective build
environments. This is NOT a bug.

### Category C: Build timestamps (45 pages)

The cached Jekyll site has timestamps from when it was built. Rustkyll produces
current timestamps. This is NOT a bug.

These three categories already have a precedent in the codebase:
`is_acceptable_build_time_diff()` exists in `dom_compare.py` and filters
timestamp differences. Categories A and B need similar filters.

## Impact

Filtering these diffs would:
- Reveal choosealicense's true score: ~66/72 (up from reported 17/72)
- Apply automatically to any future sites with GitHub Pages cached output
- Reduce noise in DOM comparison reports, making real diffs easier to spot

The remaining ~6 real diffs on choosealicense are:
- IAL `{:.bullets}` class applied to wrong element (3 pages)
- JSON-LD `&#39;` entity escaping (2 pages)
- Description whitespace normalization (1 page)

## Scope

### In scope

1. **Add `is_acceptable_jekyll_version_diff()` filter** -- filter diffs where
   the only difference is the Jekyll generator version string in a `<meta>`
   tag. Pattern: `attribute_differs` on a `<meta>` tag where expected contains
   `Jekyll v` and actual contains `Jekyll v` with a different version number.

2. **Add `is_acceptable_github_pages_url_diff()` filter** -- filter diffs where
   the difference is between GitHub Pages infrastructure URL patterns. Pattern:
   `jsonld_value_differs` or `attribute_differs` where expected contains
   `github.com/pages/{owner}/{repo}` and actual contains
   `{owner}.github.io/{repo}` (or vice versa), representing the same logical
   site URL in different GitHub Pages environments.

3. **Add `is_acceptable_build_timestamp_diff()` enhancement** -- ensure the
   existing build time filter covers all timestamp formats encountered in
   choosealicense (ISO 8601 with timezone offsets).

4. **Integrate filters into `filter_acceptable_diffs()`** -- add the new
   filters to the existing acceptable diff filtering pipeline.

5. **Add tests for all new filters** in `scripts/test_dom_compare.py`.

### Out of scope

- Fixing the real remaining diffs on choosealicense (IAL class, entity escaping,
  whitespace) -- those are tracked by issue 303 follow-ups
- Rebuilding the cached Jekyll site locally -- that would also solve the problem
  but is a different approach

## Dependencies

- None. This is independent tooling work.

## Key Files to Modify

- `scripts/dom_compare.py` -- add new filter functions, integrate into
  `filter_acceptable_diffs()`
- `scripts/test_dom_compare.py` -- add unit tests for new filters

## Acceptance Criteria

- [ ] `python3 scripts/test_dom_compare.py` passes with all existing tests
      plus new tests below
- [ ] Jekyll version string diffs (`Jekyll v3.10.0` vs `Jekyll v4.4.1`) are
      filtered as acceptable
- [ ] GitHub Pages URL pattern diffs (`github.com/pages/org/repo` vs
      `org.github.io/repo`) are filtered as acceptable
- [ ] The filter correctly handles both directions (expected=GitHub Pages,
      actual=github.io and vice versa)
- [ ] The filter does NOT match URL diffs that are not GitHub Pages patterns
      (e.g., two completely different domains)
- [ ] choosealicense DOM comparison shows 62+/72 matched (up from 17/72)
      after filtering
- [ ] No regressions on DTC comparison results (filters must not incorrectly
      accept real diffs)
- [ ] No regressions on muan-blog or mlwiki comparison results
- [ ] Tests include non-ASCII content (URLs with Unicode path segments in
      GitHub Pages patterns)

## Test Scenarios

### Unit: Jekyll version filter

- Diff with `expected: "content='Jekyll v3.10.0'"` and
  `actual: "content='Jekyll v4.4.1'"` on a `<meta>` tag -> filtered
- Diff with `expected: "content='Jekyll v3.10.0'"` and
  `actual: "content='Rustkyll v1.0'"` -> NOT filtered (different generator)
- Diff with `expected: "content='Some text'"` and
  `actual: "content='Other text'"` on a `<meta>` tag -> NOT filtered
  (not a Jekyll version diff)
- Diff on a non-meta element with Jekyll version text -> NOT filtered
  (must be specifically the generator meta tag)

### Unit: GitHub Pages URL filter

- `jsonld_value_differs` with expected `https://github.com/pages/github/choosealicense.com/`
  and actual `https://github.github.io/choosealicense.com/` -> filtered
- `jsonld_value_differs` with expected `https://github.com/pages/github/choosealicense.com/about/`
  and actual `https://github.github.io/choosealicense.com/about/` -> filtered
- `attribute_differs` with expected URL containing `github.com/pages/owner/repo`
  and actual URL containing `owner.github.io/repo` -> filtered
- Two completely different URLs (`example.com` vs `other.com`) -> NOT filtered
- Same domain but different paths (not a GitHub Pages pattern) -> NOT filtered
- GitHub Pages URL with Unicode path segment -> filtered correctly

### Unit: Build timestamp filter (existing + enhancement)

- Verify existing timestamp filtering still works
- ISO 8601 timestamp with `+01:00` timezone offset is filtered
- ISO 8601 timestamp with `Z` timezone is filtered

### Integration: choosealicense comparison

- Run DOM comparison on choosealicense with the new filters active
- Verify match count is 62+/72
- Verify the unmatched pages have REAL diffs (IAL class, entity escaping),
  not env-dependent diffs
- Run DOM comparison on DTC, verify no change in match count (filters
  should not affect DTC results)

### Regression: Other sites

- Run DOM comparison on muan-blog, verify no change in match count
- Run DOM comparison on mlwiki, verify no change in match count
- Run full test suite: `python3 scripts/test_dom_compare.py`

## Output Verification

```bash
# Run choosealicense comparison before and after
python3 scripts/dom_compare.py \
  --jekyll-dir websites/choosealicense.com/_site_jekyll_cached \
  --rustkyll-dir /tmp/rustkyll_choosealicense

# Expected: 62+/72 matched (up from 17/72)
# The ~6-10 remaining diffs should be real issues (IAL, entity, whitespace)

# Verify DTC is not affected
python3 scripts/dom_compare.py \
  --jekyll-dir websites/DataTalksClub/datatalksclub.github.io/_site_jekyll_cached \
  --rustkyll-dir /tmp/dtc_test

# Expected: same match count as before (745+/790)
```

## Notes

The filter must be precise. It should only match diffs where the ONLY semantic
difference is the environment-dependent value. It must NOT accidentally filter
real bugs where a URL or version happens to appear in the diff text.

Pattern matching approach:
- Jekyll version: match `attribute_differs` where both expected and actual
  contain `Jekyll v` followed by a semver-like pattern
- GitHub Pages URL: match diffs where the URL paths are identical but the
  domain differs between `github.com/pages/{owner}/{repo}` and
  `{owner}.github.io/{repo}` patterns. Extract the path suffix and compare.

## Log

### [SWE] 2026-03-23
- Wrote 24 new tests across 4 test classes (TDD: tests written first, verified import fails)
  - TestJekyllVersionFilter: 6 tests for version string filtering
  - TestGitHubPagesUrlFilter: 9 tests including Unicode path segments and both directions
  - TestBuildTimestampFilterEnhancement: 6 tests for ISO 8601 with +offset and Z
  - TestFilterIntegrationWithNewFilters: 3 tests verifying pipeline integration
- Ran tests: FAILS as expected (ImportError for missing functions)
- Implemented `is_acceptable_jekyll_version_diff()` -- filters attribute_differs where both sides have `content='Jekyll vX.Y.Z'`
- Implemented `is_acceptable_github_pages_url_diff()` -- filters jsonld_value_differs and attribute_differs where URLs differ only by github.com/pages/{owner}/{repo} vs {owner}.github.io/{repo} pattern
- Verified existing `_is_build_time_only_diff()` already handles ISO 8601 with +01:00 and Z timezones
- Integrated new filters into `filter_acceptable_diffs()` first pass
- Ran tests: ALL 140 PASS
- choosealicense comparison: 62/72 matched (up from 17/72), 255 acceptable diffs filtered
- DTC comparison: 745 matched (unchanged), no regressions
- muan-blog comparison: 2174 matched (unchanged), no regressions
- Files modified: scripts/dom_compare.py, scripts/test_dom_compare.py
