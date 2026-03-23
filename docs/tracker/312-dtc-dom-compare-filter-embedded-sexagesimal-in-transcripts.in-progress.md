# Issue 312: Filter embedded sexagesimal timestamps in DTC podcast transcript JSON-LD

## Problem

DTC matches 681/790 (86%). Of the 109 remaining diff pages, approximately 60
are podcast pages where the ONLY difference is sexagesimal timestamp formatting
embedded within transcript text in JSON-LD.

### The diff pattern

Podcast JSON-LD includes a `transcript` field with chapter timestamps:

```
Jekyll:   "...Welcome, Stefan. [1.0]\nStefan: Thank you. [36.0]\n..."
Rustkyll: "...Welcome, Stefan. [0:01]\nStefan: Thank you. [0:36]\n..."
```

Jekyll's YAML 1.1 parser interprets `0:36` as sexagesimal notation and
converts it to `36.0` (float). Rustkyll intentionally preserves the
human-readable `0:36` format. This is a known, documented acceptable
difference (see `src/yaml.rs` `test_sexagesimal_short_timestamp_stays_as_string`).

### Why current filters miss these

The existing `is_acceptable_sexagesimal_diff()` filter in `dom_compare.py`
(line 133) only matches when the ENTIRE `expected` and `actual` strings are
a float vs time value. For transcript diffs, the strings are thousands of
characters long with sexagesimal differences scattered throughout. The filter
sees a 5000-char transcript string and doesn't recognize it as a sexagesimal
issue.

### Verification

Tested all 194 DTC podcast pages:
- 134 transcripts: identical between Jekyll and rustkyll
- 60 transcripts: differ ONLY due to embedded sexagesimal timestamps
  (verified by normalizing `[N.N]` to `[M:SS]` format -- all 60 match after
  normalization)
- 0 transcripts: have real (non-sexagesimal) differences

## Scope

### In scope

1. **Add embedded sexagesimal normalization to JSON-LD comparison** -- before
   comparing JSON-LD string values, normalize embedded timestamp patterns so
   that `[36.0]` and `[0:36]` are treated as equivalent.

2. **Implementation approach** (choose one):

   a. **Normalization approach** (preferred): In `_compare_jsonld_values()`,
      before comparing strings, apply a normalization function that converts
      both `[N.N]` (float) and `[M:SS]` (time) formats to a canonical form.
      If the normalized strings match, skip the diff.

   b. **Post-filter approach**: After the diff is generated, check if the
      only differences between expected and actual are sexagesimal timestamp
      patterns. This is harder because the strings may have many differences
      (one per timestamp).

   c. **Pre-compare approach**: Add a new filter in `filter_acceptable_diffs()`
      that checks `jsonld_value_differs` diffs where the path contains
      `transcript`. Normalize both values and compare -- if they match after
      normalization, accept the diff.

3. **Ensure description diffs with trailing newlines are also handled** --
   issue 307 fixed the 200-char truncation bug. Verify that description-only
   diffs are now correctly caught by the existing trailing newline filter.
   If not, fix that too.

### Out of scope

- Changing rustkyll's YAML sexagesimal handling (intentionally preserves
  human-readable format)
- Fixing non-JSON-LD diffs on DTC pages (structural, text content)
- Changes to any Rust code

## Dependencies

- Issue 307 (truncated description filter fix) -- IN PROGRESS. The truncation
  fix from 307 is needed for the trailing newline filter to work on long
  descriptions. This issue extends the filtering to also cover transcript
  sexagesimal diffs.

## Key Files to Modify

- `scripts/dom_compare.py` -- add `_normalize_embedded_sexagesimal()` helper
  and integrate into `_compare_jsonld_values()` or `filter_acceptable_diffs()`
- `scripts/test_dom_compare.py` -- add tests for embedded sexagesimal
  normalization

## Acceptance Criteria

- [ ] Running `dom_compare.py` on DTC produces 740+/790 matched files
      (up from current 681, with ~60 transcript pages now filtered)
- [ ] Podcast pages with ONLY embedded sexagesimal timestamp diffs in
      transcript JSON-LD are counted as matched
- [ ] The filter correctly normalizes `[36.0]` (Jekyll float) to match
      `[0:36]` (rustkyll time)
- [ ] The filter correctly normalizes `[3723.0]` to match `[1:02:03]`
      (hours:minutes:seconds)
- [ ] The filter correctly handles `[0.0]` matching `[0:00]`
- [ ] Pages with real transcript text differences (not just timestamps)
      are NOT filtered
- [ ] No change in match counts for mlwiki, muan-blog, choosealicense,
      or any other non-DTC site
- [ ] `python3 -m pytest scripts/test_dom_compare.py` passes
- [ ] Tests include non-ASCII/Unicode content (transcripts with accented
      names, CJK text around timestamps)

## Test Scenarios

### Unit: Sexagesimal normalization in strings

- Normalize `"Hello [36.0] world"` and `"Hello [0:36] world"` -- both
  should produce the same canonical form
- Normalize `"[0.0] Start [3723.0] End"` and `"[0:00] Start [1:02:03] End"`
  -- should match after normalization
- Normalize `"[1.5] text"` and `"[0:01] text"` -- `1.5` is 1.5 seconds,
  `0:01` is 1 second. The YAML 1.1 sexagesimal interpretation of `0:01` is
  `1.0`, so `[1.5]` should NOT match `[0:01]`. Verify non-matching case.
- Normalize `"Price is $36.00"` -- `[36.00]` pattern should only match
  inside square brackets, not in arbitrary text
- Normalize `"[1:30:45]"` and `"[5445.0]"` (1*3600 + 30*60 + 45 = 5445) --
  should match

### Unit: Transcript-specific filtering

- Two transcript strings differing only in `[N.N]` vs `[M:SS]` timestamps
  -- diff should be FILTERED as acceptable
- Two transcript strings differing in actual word content (not timestamps)
  -- diff should NOT be filtered
- Transcript with mix of sexagesimal diffs and real word diffs -- diff
  should NOT be filtered (not all differences are sexagesimal)
- Empty transcript vs non-empty -- should NOT be filtered

### Unit: Edge cases

- `[0.5]` in Jekyll (0.5 seconds, from YAML `0:00.5`?) -- verify handling
- `[120.0]` vs `[2:00]` -- should match (2 minutes = 120 seconds)
- Unicode around timestamps: `"[36.0] cafe\u0301" vs "[0:36] cafe\u0301"` --
  should match (only timestamp differs)

### Integration: DTC full comparison

- Run `python3 scripts/dom_compare.py` on DTC
- Verify matched count is 740+/790
- Spot-check pages:
  - `podcast/ai-in-healthcare-and-digital-therapeutics.html` -- matched
  - `podcast/ai-infrastructure-hybrid-cloud-on-prem-distributed-training.html`
    -- matched
  - `podcast/teaching-reproducible-research-and-open-science-coding-practices-
    for-academia.html` -- check (has both description and transcript diffs)
- Verify acceptable diffs filtered count increases by ~60

### Regression

- Run dom_compare.py on mlwiki -- verify unchanged
- Run dom_compare.py on muan-blog -- verify unchanged
- `python3 -m pytest scripts/test_dom_compare.py` -- all tests pass

## Output Verification

```bash
python3 scripts/dom_compare.py \
  --jekyll-dir websites/DataTalksClub/datatalksclub.github.io/_site_jekyll_cached \
  --rustkyll-dir websites/DataTalksClub/datatalksclub.github.io/_site_rustkyll \
  --output /tmp/dtc_after_312.txt

# Summary line must show >= 740 files matched (up from 681)
# Acceptable diffs filtered should increase by ~60

# Spot-check: podcast pages should not appear as DIFF
grep "podcast/ai-in-healthcare" /tmp/dtc_after_312.txt
# Should NOT appear as DIFF (or if it does, only for non-transcript reasons)

# Regression
python3 scripts/dom_compare.py \
  --jekyll-dir websites/alexeygrigorev/mlwiki.org/_site_jekyll_cached \
  --rustkyll-dir websites/alexeygrigorev/mlwiki.org/_site_rustkyll
# Must show same count as before
```

## Log

### [SWE] 2026-03-23
- Wrote 20 tests first: TestNormalizeEmbeddedSexagesimal (16 tests) and TestEmbeddedSexagesimalJsonLDFiltering (4 tests)
- Ran tests: FAILS as expected -- ImportError: cannot import name '_normalize_embedded_sexagesimal'
- Implemented `_seconds_to_canonical_time()` and `_normalize_embedded_sexagesimal()` in dom_compare.py
- Integrated normalization into `_compare_jsonld_values()`: before reporting a diff, normalize both strings with `_normalize_embedded_sexagesimal()` and skip if they match
- Ran tests: 20 new tests PASS
- DTC comparison: 739 files matched, 51 with differences (up from 681, +58 pages)
- 273 acceptable diffs filtered out
- Files modified: scripts/dom_compare.py, scripts/test_dom_compare.py
- Note: 1 pre-existing test failure in TestPageLevelMathBugFilter from issue 311 (not related to this issue)
