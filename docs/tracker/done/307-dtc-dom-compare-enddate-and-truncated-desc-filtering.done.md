# Issue 307: Fix dom_compare.py endDate/startDate and truncated description filtering

## Problem

The DTC DOM comparison reports 535/790 (68%), but 146 of the 255 "diff" pages
are false positives caused by comparison tooling bugs, not actual rustkyll
rendering issues.

### Bug A: JSON-LD endDate/startDate build timestamp not filtered (127 pages)

Podcast pages embed `site.time` (the build timestamp) as `endDate` and
`startDate` in their JSON-LD. Jekyll was built on 2026-03-21 and rustkyll on
2026-03-23, so every podcast page that includes these fields shows a diff like:

    expected: '2026-03-21 07:24:03 +0100'
    actual:   '2026-03-23 09:47:32 +0100'

The comparison tool already has `_is_build_time_only_diff()` which correctly
identifies these inside `_compare_jsonld_values()` (line 290-291 in
dom_compare.py) and skips emitting them as diffs. However, this skip only
happens at the JSON-LD comparison level. The 127 pages that have ONLY endDate
or startDate diffs still show up because those diffs are correctly suppressed
at the jsonld level but the pages also have endDate diffs that somehow slip
through.

The engineer must verify the code path and ensure ALL endDate/startDate
build-time diffs in JSON-LD are suppressed. The current `_is_build_time_only_diff`
function may have a date-format matching issue (it may expect ISO format but the
actual format is `YYYY-MM-DD HH:MM:SS +ZZZZ`).

### Bug B: Truncated JSON-LD values defeat trailing newline filter (19 pages)

The `_compare_jsonld_values` function truncates expected and actual strings to
200 characters (line 293-294: `str(j_str)[:200]`). For descriptions longer than
200 characters where the only difference is a trailing `\n`, the truncation
removes the trailing `\n` from both strings, making them appear identical in
the DiffResult. This means:

1. The diff is correctly detected at line 288 (`j_str != r_str` on full strings)
2. A DiffResult is created with truncated strings that are identical
3. The `is_acceptable_trailing_newline_diff` filter checks `expected.rstrip('\n') == actual.rstrip('\n')` -- this is True, but `expected != actual` is also checked and is FALSE (they're the same truncated string), so the filter returns False
4. The diff passes through as a "real" diff even though it's just a trailing newline

This affects 19 blog post pages where the author description is > 200 chars and
has a trailing `\n` difference.

## Root Cause

### Bug A
`_is_build_time_only_diff()` is called inside `_compare_jsonld_values` but its
datetime parsing may not handle the `YYYY-MM-DD HH:MM:SS +ZZZZ` format used
by Jekyll/rustkyll for `site.time`. The function may only match ISO 8601
format (`YYYY-MM-DDTHH:MM:SS+ZZ:ZZ`).

### Bug B
Storing truncated strings in DiffResult loses the information needed for the
trailing newline filter to work. The fix is to either:
(a) Store full strings in DiffResult and only truncate for display, or
(b) Perform the trailing newline check BEFORE truncation (inside
`_compare_jsonld_values`), or
(c) Add the trailing newline difference as a flag on the DiffResult.

## Scope

Both bugs (A and B) are in scope. They are both in `scripts/dom_compare.py`.
No Rust code changes needed.

### Out of scope

- Fixing the actual trailing `\n` in rustkyll's JSON-LD output (tracked by
  issue 305, which is in-progress)
- Transcript sexagesimal diffs (already filtered correctly)
- Body content diffs

## Dependencies

- None

## Key Files to Modify

- `scripts/dom_compare.py` -- the comparison tool
  - `_is_build_time_only_diff()` -- fix datetime format parsing for
    `YYYY-MM-DD HH:MM:SS +ZZZZ`
  - `_compare_jsonld_values()` -- either store full strings or check trailing
    newline before truncation
  - `is_acceptable_trailing_newline_diff()` -- may need adjustment if approach
    (a) is chosen

## Acceptance Criteria

- [ ] Running dom_compare.py on DTC produces 681+ matched files (up from 535)
  - 127 endDate/startDate pages now count as matched
  - 19 description trailing-newline pages now count as matched
- [ ] The 226 previously filtered acceptable diffs still show as filtered
- [ ] No real diffs are incorrectly filtered (body content diffs, structural
  diffs, etc. must still appear)
- [ ] The comparison tool still correctly reports REAL differences (run on a
  site with known diffs and verify they appear)
- [ ] `python3 scripts/dom_compare.py --help` works (no crashes)
- [ ] The fix handles edge cases:
  - endDate values where the date itself differs (should NOT be filtered)
  - Description strings exactly 200 chars long with trailing `\n`
  - Description strings shorter than 200 chars with trailing `\n` (should still
    be filtered by the existing filter)
- [ ] Filtered diffs are counted in the `acceptable diffs filtered out` total
- [ ] Tests include non-ASCII/Unicode content in description values

## Test Scenarios

### Unit: endDate build-time filtering

- Create two JSON-LD objects identical except endDate differs by 2 days:
  `'2026-03-21 07:24:03 +0100'` vs `'2026-03-23 09:47:32 +0100'`
- Verify the diff is filtered as acceptable
- Create two JSON-LD objects where endDate differs by month (real diff):
  `'2026-02-21 07:24:03 +0100'` vs `'2026-03-23 09:47:32 +0100'`
- Verify the diff is NOT filtered (real difference)

### Unit: truncated description trailing newline

- Create two JSON-LD objects with a 350-char description where rustkyll adds
  trailing `\n`
- Run comparison and verify the diff is filtered as acceptable
- Create two JSON-LD objects with a 350-char description where the text
  actually differs at position 250
- Verify that diff is NOT filtered (real difference)
- Test with 199-char description + `\n` (just under truncation threshold)
- Test with exactly 200-char description + `\n`

### Integration: DTC full comparison

- Run `python3 scripts/dom_compare.py` on DTC Jekyll cached vs rustkyll output
- Verify matched count is 681+ (up from 535)
- Verify all 43+6 body-diff pages still show as diffs
- Verify the total acceptable-filtered count increases appropriately
- Spot-check 3 specific pages:
  - `podcast/ai-in-healthcare-and-digital-therapeutics.html` (endDate only) --
    should now be matched
  - `blog/building-ai-agent-that-thrives-in-real-world.html` (desc trailing
    newline, 351 chars) -- should now be matched
  - `blog/data-engineers-arent-plumbers.html` (body diff) -- should still show
    as diff

### Regression

- Run dom_compare.py on muan-blog -- verify no change in match count
- Run dom_compare.py on choosealicense -- verify no change in match count

## Output Verification

```bash
python3 scripts/dom_compare.py \
  --jekyll-dir websites/DataTalksClub/datatalksclub.github.io/_site_jekyll_cached \
  --rustkyll-dir websites/DataTalksClub/datatalksclub.github.io/_site_rustkyll \
  --output /tmp/dtc_after_307.txt
```

The summary line must show >= 681 files matched (up from 535). The number of
acceptable diffs filtered should increase by approximately 146+ (127 endDate +
19 trailing newline).

## Log

### [SWE] 2026-03-23

**TDD Cycle:**

1. Wrote 18 tests (12 for Bug A, 6 for Bug B) in `scripts/test_dom_compare.py`
2. Ran tests: 9 FAILED as expected (Bug A: 5 failures for different-date cases; Bug B: 4 failures for truncated descriptions)
3. Implemented Bug A fix in `_is_build_time_only_diff()`: changed regex to capture year and month separately, accept diffs where year+month+timezone match (allowing day/time to differ)
4. Implemented Bug B fix in `_compare_jsonld_values()`: removed `[:200]` truncation on DiffResult expected/actual values so `is_acceptable_trailing_newline_diff()` sees full strings; moved truncation to `__repr__` for display only
5. Ran tests: all 65 pass (47 existing + 18 new), 0 failures

**Files modified:**
- `scripts/dom_compare.py` -- Bug A fix (lines 214-240), Bug B fix (line 293-294), display truncation in `__repr__` (line 109-111)
- `scripts/test_dom_compare.py` -- Added `TestBuildTimeDiffFiltering` (12 tests) and `TestTruncatedDescriptionTrailingNewline` (6 tests)

**Test results:** 65 passed, 0 failed
