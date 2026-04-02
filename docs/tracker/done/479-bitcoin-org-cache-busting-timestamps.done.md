# Issue 479: bitcoin-org cache-busting timestamp diffs

## Problem

bitcoin-org templates use `{{ site.time | date: '%s' }}` extensively to append Unix epoch
cache-busting query strings to CSS, JS, image, and favicon URLs (e.g.,
`/css/main.css?1774771914`). Since `site.time` is the build timestamp, Jekyll and
rustkyll builds always produce different values (e.g., `?1774771914` vs `?1774898774`).

These are NOT content bugs -- both outputs are correct. The timestamps are build
artifacts that create noise in the DOM comparison, making it harder to identify real
diffs.

## Root Cause

The templates in `_includes/layout/base/html-head.html`, `_includes/layout/base/*.html`,
`_layouts/*.html`, and content markdown files all use `{{ site.time | date: '%s' }}`
which produces a 10-digit Unix epoch timestamp. Jekyll and rustkyll both implement this
correctly -- they just produce different timestamps because they were built at different
times.

## Scope

Add an `is_acceptable_cache_busting_timestamp_diff` filter in `scripts/dom_compare.py`
that marks `attribute_differs` diffs as acceptable when both the expected and actual
values are identical after stripping numeric query string suffixes (i.e., `?\d+` at
the end of a URL-like value).

This is a comparison tooling change only -- no rustkyll Rust code changes needed.

## Constraints

- The filter must only match `attribute_differs` diffs where both sides contain
  URL-like values with `?\d+` suffixes
- Stripping the `?\d+` suffix from both sides must make them identical -- if the
  base URLs differ, the diff is real and must NOT be filtered
- The filter must NOT match diffs where the query string is not purely numeric
  (e.g., `?v=2.1` is not a cache-busting timestamp)
- Must not affect DTC DOM match count (DTC does not use this pattern)

## Baselines

- DTC DOM: 596/790 matched (must not drop below 596)
- bitcoin-org DOM: 1/127 matched, 11094 total diffs
- Expected after filter: 1/127 matched (no page flips -- all 125 pages with timestamp
  diffs also have other real diffs), but ~1400 fewer total diffs reported

**Important note:** The original issue suggested 142/142 target. Investigation shows
that zero bitcoin-org pages have ONLY timestamp diffs -- every page with timestamp
diffs also has other real diffs (missing_text, tag_name_differs, etc.). The filter
will reduce noise but will not increase the page match count. The true value is
cleaner reports and lower total diff counts.

## Dependencies

None.

## Acceptance Criteria

- [ ] `is_acceptable_cache_busting_timestamp_diff(diff)` function exists in `scripts/dom_compare.py`
- [ ] Function is called in `filter_acceptable_diffs` alongside existing filters
- [ ] Filter matches `attribute_differs` diffs where expected and actual differ only
      in a trailing `?\d+` query string (e.g., `href='/css/main.css?1774771914'` vs
      `href='/css/main.css?1774898774'`)
- [ ] Filter does NOT match when base URLs differ (e.g., `src='/js/base.js?123'` vs
      `src='/js/main.js?456'` -- different base paths)
- [ ] Filter does NOT match non-numeric query strings (e.g., `?v=2.1`)
- [ ] Filter does NOT match `text_differs`, `missing_text`, `tag_name_differs`, or
      other diff types -- only `attribute_differs`
- [ ] DTC DOM match count remains at or above 596/790
- [ ] bitcoin-org total diff count drops by at least 1000 (from ~11094)
- [ ] bitcoin-org accepted diff count increases correspondingly
- [ ] `cargo build` still compiles (no Rust changes expected, but verify)

## Test Scenarios

### Unit: filter function
- `attribute_differs` with `href='/css/main.css?1774771914'` vs `href='/css/main.css?1774898774'` -> accepted
- `attribute_differs` with `src='/js/base.js?1774771914'` vs `src='/js/base.js?1774898774'` -> accepted
- `attribute_differs` with `content='https://bitcoin.org/img/icons/opengraph.png?1774771914'` vs `content='https://bitcoin.org/img/icons/opengraph.png?1774898774'` -> accepted
- `attribute_differs` with `src='/js/base.js?1774771914'` vs `src='/js/main.js?1774898774'` -> NOT accepted (different base paths)
- `attribute_differs` with `href='/css/main.css?v=2.1'` vs `href='/css/main.css?v=2.2'` -> NOT accepted (non-numeric query)
- `text_differs` with any timestamp content -> NOT accepted (wrong diff type)
- `attribute_differs` with `class='foo'` vs `class='bar'` -> NOT accepted (no query string)
- `attribute_differs` with `href='/css/main.css'` vs `href='/css/main.css?1774898774'` -> accepted (one side has no timestamp, one does -- still a build artifact)

### Integration: DOM comparison
- Run `dom_compare.py` on bitcoin-org and verify total diff count drops by 1000+
- Run `dom_compare.py` on DTC and verify match count stays at 596/790 or above
- Verify the filter appears in the "acceptable diffs filtered out" count in the summary line

## Implementation Notes

Follow the pattern of existing filters like `is_acceptable_build_time_diff`. The new
function should:

1. Check `diff.diff_type == "attribute_differs"`
2. Extract the attribute values from `diff.expected` and `diff.actual`
3. Strip trailing `?\d+` from both values using regex
4. Return `True` if the stripped values are equal (and at least one had a `?\d+` suffix)

## Log

### [PM] 2026-04-02 grooming
- Investigated bitcoin-org templates: `site.time | date: '%s'` used in 30+ include/layout files
- Confirmed this is a comparison artifact, not a rustkyll bug
- Analyzed full diff report: ~1414 timestamp diffs out of ~11100 total
- Zero pages have ONLY timestamp diffs -- all also have other real diffs
- Corrected target: filter reduces noise but does not increase page match count
- DTC baseline recorded: 596/790
- bitcoin-org baseline recorded: 1/127 matched, 11094 total diffs

### [SWE] 2026-04-02

**Fix 1: Add is_acceptable_cache_busting_timestamp_diff filter**
- Wrote 11 tests in TestCacheBustingTimestampFilter class (scripts/test_dom_compare.py)
- Ran tests: FAILS -- ImportError: cannot import name 'is_acceptable_cache_busting_timestamp_diff'
- Implemented is_acceptable_cache_busting_timestamp_diff() in scripts/dom_compare.py
- First run: 7/11 tests FAIL -- regex matched `?\d+$` but values have trailing `'` (format is `href='/path?123'`)
- Fixed regex to `\?\d+(\'?)$` with backreference to preserve trailing quote
- Ran tests: ALL 11 PASS
- Added function call to filter_acceptable_diffs() alongside existing filters

**Verification:**
- Python tests: 154 passed, 2 failed (pre-existing timezone failures, unrelated)
- Rust tests: all pass
- DTC DOM: 596/790 matched, 255 total diffs (unchanged from baseline)
- bitcoin-org DOM: 1/127 matched, 9691 total diffs (down from 11094 -- 1403 timestamp diffs filtered)
- DTC build time: 0.665s (under 1.0s threshold)
- Clippy/fmt: N/A (Python-only change)

**Summary:**
- Files modified: scripts/dom_compare.py, scripts/test_dom_compare.py
- Tests added: 11 unit tests for cache-busting timestamp filter
- bitcoin-org total diffs reduced by 1403 (11094 -> 9691)
- DTC unaffected (596/790, 255 diffs)

### [QA] 2026-04-02
- Python tests: 156 ran, 154 passed, 2 failed (pre-existing timezone failures, confirmed by running on stashed code)
- 11 new tests in TestCacheBustingTimestampFilter: all pass
- Rust tests: all pass
- Clippy: clean (only dependency warnings)
- Fmt: clean
- TDD compliance: PASS -- SWE log shows tests written first (ImportError), then implementation, then regex fix after 7/11 failures
- DTC DOM: 596/790, 255 total diffs (matches baseline of 596, no regression)
- bitcoin-org DOM: 1/127, 9691 total diffs (down from 11094 baseline, reduction of 1403)
- DTC build time: N/A (Python-only change, no rendering pipeline impact)
- Code follows existing patterns (import re inside function, same filter structure)
- Unicode test included (test_unicode_path_with_timestamps)
- Acceptance criteria:
  1. Function exists: PASS
  2. Integrated in filter_acceptable_diffs: PASS
  3. Matches attribute_differs with trailing ?\d+: PASS
  4. Does NOT match different base URLs: PASS
  5. Does NOT match non-numeric query strings: PASS
  6. Does NOT match non-attribute_differs types: PASS
  7. DTC DOM >= 596/790: PASS (596/790)
  8. bitcoin-org diffs drop by 1000+: PASS (1403 reduction)
  9. bitcoin-org accepted diffs increase: PASS (4074 filtered)
  10. cargo build compiles: PASS
- VERDICT: PASS

### [PM] 2026-04-02 16:30
- Reviewed diff: 2 files changed (scripts/dom_compare.py, scripts/test_dom_compare.py)
- Output verification: ran DTC DOM recount via recount-all-dom.sh, confirmed 596/790 (no regression); ran all 11 new unit tests, all pass
- Results verified: bitcoin-org diffs reduced 11094 -> 9691 (1403 filtered), DTC 596/790 unchanged
- Acceptance criteria: all 10 met
  1. Function exists: PASS
  2. Integrated in filter_acceptable_diffs: PASS
  3. Matches attribute_differs with trailing ?\d+: PASS
  4. Does NOT match different base URLs: PASS
  5. Does NOT match non-numeric query strings: PASS
  6. Does NOT match non-attribute_differs types: PASS
  7. DTC DOM >= 596/790: PASS (596/790)
  8. bitcoin-org diffs drop by 1000+: PASS (1403)
  9. bitcoin-org accepted diffs increase: PASS
  10. cargo build compiles: PASS
- Code quality: clean, follows existing filter patterns, regex handles trailing quote correctly
- Follow-up issues created: none needed
- VERDICT: ACCEPT
