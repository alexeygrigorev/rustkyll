# Issue 552: choosealicense.com -- false positive timestamp diffs in DOM comparison

## Problem

choosealicense.com shows 25/72 (35%) DOM match, but ALL 47 diffs are false positives caused by build-time timestamps in meta tags. Every diff is identical:

```
head > meta > meta > link > meta: attribute_differs
  expected: "content='2026-03-20T23:47:36+01:00'"
  actual:   "content='2026-04-04T00:48:27+02:00'"
```

The Jekyll cache was built on 2026-03-20, and rustkyll was built on 2026-04-04. The sites are functionally identical -- the only difference is the `<meta>` tag with the build timestamp (likely `og:updated_time` or similar SEO tag).

This is NOT a rustkyll rendering bug. It is a DOM comparison methodology issue.

## Scope

Fix the DOM comparison script (`scripts/dom_compare.py`) to treat build-time timestamp meta tags as acceptable differences, similar to how it already filters "acceptable diffs." Specifically:

1. Meta tags with `property="article:modified_time"` or similar temporal meta tags where the content is a timestamp should be treated as acceptable when the only difference is the timestamp value.
2. Alternatively, add a timestamp normalization step that ignores ISO 8601 timestamp differences in meta content attributes.

This will correctly push choosealicense.com from 25/72 to 72/72 (100%).

## Dependencies

None.

## Acceptance Criteria

- [ ] `dom_compare.py` correctly identifies timestamp-only meta tag diffs as acceptable
- [ ] choosealicense.com DOM comparison shows 72/72 (100%) after the fix
- [ ] The filter is generic (applies to any site with build-time meta timestamps, not hardcoded to choosealicense)
- [ ] DTC main DOM match count does not change (596/790 -- DTC does not have this pattern)
- [ ] No false negatives: real attribute diffs in meta tags are still reported
- [ ] `uv run scripts/dom_compare.py --help` still works

## Test Scenarios

### Unit: Timestamp meta tag filtering
- Two HTML docs identical except for `<meta content="2026-03-20T23:47:36+01:00">` vs `<meta content="2026-04-04T00:48:27+02:00">` -- should report 0 diffs
- Two HTML docs with different meta content that is NOT a timestamp -- should still report diff
- Two HTML docs with timestamp diff AND a real structural diff -- should report only the structural diff

### Integration: choosealicense.com
- Run `dom_compare.py` on choosealicense.com Jekyll vs rustkyll output
- Verify 72/72 match (all 47 timestamp diffs filtered as acceptable)

## DOM Baseline

- DTC main: 596/790 (must not change)
- choosealicense.com: 25/72 (target: 72/72)

## Log

### [PM] 2026-04-02 grooming
- All 47 diffs are identical: build timestamp in meta tag content attribute
- Jekyll cache built 2026-03-20, rustkyll built 2026-04-04
- No actual rendering difference -- purely a comparison methodology false positive
- Fix in dom_compare.py, not in rustkyll itself
