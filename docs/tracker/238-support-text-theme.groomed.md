# Issue 238: Support TeXt Jekyll theme

## Problem

TeXt is a popular Jekyll theme with rich features for content-heavy sites, but it is not currently in our benchmark suite.

## Theme Details

- **GitHub:** https://github.com/kitian616/jekyll-TeXt-theme
- **Stars:** ~3,000
- **Use case:** Personal blogs, documentation, content-heavy sites
- **Notable features:** Skins and color schemes, table of contents, tags/categories, search, charts (Chart.js), mermaid diagrams, math (MathJax), multiple layout options, i18n support

## Scope

1. Clone the TeXt demo site into `websites/text-theme/`.
2. Build the demo site with both Jekyll and rustkyll.
3. Run DOM comparison against the cached Jekyll output and record the real match rate.
4. Identify TeXt-specific rendering blockers and either fix them in this issue or create follow-up issues that reference `#238`.

## Baseline

- DTC DOM baseline: `766/790`

## Acceptance Criteria

- [ ] The TeXt demo site is cloned into `websites/text-theme/` and the repository state is documented in the issue log.
- [ ] Jekyll builds the TeXt demo site successfully and produces a reference `_site` output.
- [ ] rustkyll builds the TeXt demo site successfully without errors and produces HTML output for the same site.
- [ ] The DOM comparison between the Jekyll and rustkyll outputs is run and the issue log records the exact match count, differing-file count, and main diff categories.
- [ ] Representative pages that exercise TeXt features are verified in the output, including the homepage plus pages covering navigation/layout behavior, table of contents, tags/categories, blog content with math, charts or diagrams if present, and multilingual content if present in the demo.
- [ ] Any TeXt-specific rendering issues discovered during comparison are either fixed in this issue or explicitly tracked in follow-up issues that reference `#238`.
- [ ] The DTC DOM match count does not drop below `766/790`.

## Test Scenarios

### Integration: demo site setup
- Clone the upstream TeXt demo site into `websites/text-theme/` and verify the expected theme files and configuration are present.
- Run `jekyll build` in the demo site and confirm the reference HTML output is generated.
- Run `rustkyll build --source websites/text-theme --destination /tmp/text-theme-rustkyll` and confirm the same page set is generated.

### Integration: output comparison
- Run DOM comparison against the Jekyll reference output and record the exact match count, differing-file count, and notable diff categories.
- Inspect representative pages for navigation, table of contents, tags/categories, blog posts with math, charts or diagrams, and multilingual pages if present in the demo.
- Verify any identified rendering blocker is either fixed or captured in a follow-up issue linked to `#238`.

## Dependencies

- None (research/benchmark task)

## Log

### [PM] 2026-03-24
- Groomed the issue into a benchmark-oriented spec for the TeXt demo site.
- Recorded DTC baseline: `766/790`.
- Added explicit output verification requirements, representative TeXt feature coverage, and traceable follow-up handling for any blockers.
