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

### [SWE] 2026-04-02

**Fix 1: Add is_acceptable_meta_timestamp_only_diff filter**
- Wrote 12 tests in TestMetaTimestampOnlyDiff class (scripts/test_dom_compare.py)
  - test_different_month_iso8601_timestamps_accepted
  - test_same_month_iso8601_timestamps_accepted
  - test_different_year_timestamps_accepted
  - test_space_separated_datetime_accepted
  - test_non_timestamp_content_not_accepted
  - test_partial_timestamp_not_accepted (date-only, no time)
  - test_non_attribute_diff_not_accepted
  - test_mixed_timestamp_and_text_not_accepted
  - test_unicode_timestamp_content_not_accepted
  - test_filter_integration_with_filter_acceptable_diffs
  - test_full_html_timestamp_meta_diff_filtered
  - test_full_html_real_meta_diff_not_filtered
- Ran tests: FAILS -- ImportError: cannot import name 'is_acceptable_meta_timestamp_only_diff'
- Implemented is_acceptable_meta_timestamp_only_diff in scripts/dom_compare.py:630
  - Checks diff_type == "attribute_differs" and content= in both sides
  - Extracts content values and validates both are full ISO 8601 datetimes
  - Pattern: YYYY-MM-DD[T ]HH:MM:SS[timezone] -- must be the entire content value
  - Added to filter_acceptable_diffs filter chain
- Ran tests: PASSES -- 12/12 new tests pass

**Summary:**
- Files modified: scripts/dom_compare.py, scripts/test_dom_compare.py
- Tests added: 12 (TestMetaTimestampOnlyDiff class)
- Python tests: 174 total, 172 pass, 2 pre-existing failures (unrelated timezone tests)
- Rust tests: 3880 pass, 0 fail
- choosealicense.com: 72/72 (100%) -- was 25/72, all 47 timestamp diffs filtered
- DTC DOM: 596/790 with 255 total differences -- unchanged from baseline
- DTC build time: 0.691s (under 1.0s threshold)
- Clippy/fmt: N/A (Python-only change)

### [QA] 2026-04-02
- Python tests: 174 total, 172 pass, 2 pre-existing failures (unrelated timezone tests)
- New tests (TestMetaTimestampOnlyDiff): 12/12 pass
- Rust tests: all pass, 0 failures
- Clippy: clean (warnings only from upstream liquid-lib crate)
- Fmt: clean
- DTC DOM (via recount script): 596/790, 255 diffs -- matches baseline, no regression
- choosealicense.com DOM: 72/72 (100%) -- up from 25/72, all 47 timestamp diffs correctly filtered
- DTC build time: 0.695s (under 1.0s threshold)
- TDD compliance: SWE log shows tests written first, verified FAILS (ImportError), then implementation, then PASSES
- Acceptance criteria:
  - dom_compare.py identifies timestamp-only meta tag diffs as acceptable: PASS
  - choosealicense.com 72/72 (100%): PASS
  - Filter is generic (regex-based, not site-specific): PASS
  - DTC DOM unchanged at 596/790 with 255 diffs: PASS
  - No false negatives (test_non_timestamp_content_not_accepted, test_partial_timestamp_not_accepted, test_mixed_timestamp_and_text_not_accepted, test_full_html_real_meta_diff_not_filtered): PASS
  - uv run scripts/dom_compare.py --help still works: PASS (verified during DOM runs)
- VERDICT: PASS

### [PM] 2026-04-02 Acceptance Review
- Reviewed diff: 2 files changed (scripts/dom_compare.py, scripts/test_dom_compare.py)
- Output verification: ran 12/12 unit tests (all pass), verified DTC DOM via recount script
- Results verified: DTC 596/790 (matches baseline, no regression), choosealicense 72/72 per QA log
- Acceptance criteria:
  - dom_compare.py identifies timestamp-only meta diffs as acceptable: MET
  - choosealicense.com 72/72 (100%): MET
  - Filter is generic (regex-based, not site-specific): MET
  - DTC DOM unchanged at 596/790: MET (verified independently)
  - No false negatives (4 negative tests confirm): MET
  - uv run scripts/dom_compare.py --help works: MET per QA
- Follow-up issues created: none needed
- VERDICT: ACCEPT
