# Cross-Site Build Testing Results

## Verified Results (2026-03-14)

**11 Jekyll sites found** across `alexeygrigorev` and `DataTalksClub` GitHub accounts.

- **10 of 11 sites build successfully** (91%)
- **1 of 11 sites fail** (9%)

Previous result (before issues #37-44): 7 of 11 (64%).

## Sites Tested

### alexeygrigorev (8 sites)

| Repository | Status | Pages | Static Files | Time | Notes |
|---|---|---|---|---|---|
| alexeygrigorev.github.io | OK | 8 | 11 | 0.01s | Simple personal site |
| kids-horror-stories-ru | OK | 1345 | 2624 | 73.46s | Large site with 1343+ posts |
| snippets | OK | 2 | 6 | 0.00s | Minimal site |
| data-science-interviews | OK | 0 | 27 | 0.00s | Pages not rendered (no layout specified) |
| mlwiki.org | OK | 2 | 8 | 0.00s | Minimal wiki site |
| mlbookcamp-page | OK | 15 | 77 | 0.02s | `erl_encode` filter handled via passthrough with warning |
| aihero | OK | 2 | 38 | 0.01s | `{% seo %}` tag now supported (#38) |
| little-book-of-metals-ru | OK | 1 | 12 | 0.01s | `normalize_whitespace` filter now supported (#37) |

### DataTalksClub (3 sites)

| Repository | Status | Pages | Static Files | Time | Notes |
|---|---|---|---|---|---|
| datatalksclub.github.io | OK | 784 | 1457 | 621.68s | Primary reference site |
| courses | OK | 5 | 80 | 0.03s | Course listing site |
| docs | FAIL | - | - | 0.01s | Escaped quotes in include parameter values |

## Failure Details

### 1. Escaped quotes in include parameters (DataTalksClub/docs)

```
Build failed: template parse error: liquid:   --> 23:124
   |
23 | {% include "vendor/anchor_headings.html" html=content beforeHeading="true"
   |   anchorBody="<svg viewBox=\"0 0 16 16\" ...>" ... %}
   |                                          ^---
   = expected Value, Range, ...
```

The include tag uses escaped double quotes (`\"`) inside parameter values. The Liquid template parser does not handle backslash-escaped quotes within include tag arguments. This is a different issue from #39 (include subdirectory paths), which was about forward slashes in filenames.

## Improvements Since Last Test

| Repository | Previous Status | Current Status | Fix |
|---|---|---|---|
| mlbookcamp-page | FAIL | OK | Unknown filters now pass through with warning instead of failing |
| aihero | FAIL | OK | `{% seo %}` tag implemented (#38) |
| little-book-of-metals-ru | FAIL | OK | `normalize_whitespace` filter implemented (#37) |
| DataTalksClub/docs | FAIL | FAIL | Include paths with `/` fixed (#39), but new blocker: escaped quotes in include params |

## Page Count Changes

| Repository | Previous Pages | Current Pages | Delta | Notes |
|---|---|---|---|---|
| alexeygrigorev.github.io | 0 | 8 | +8 | Pages now rendered (likely site.pages fix #42) |
| kids-horror-stories-ru | 1344 | 1345 | +1 | Minor |
| snippets | 0 | 2 | +2 | Pages now rendered |
| data-science-interviews | 0 | 0 | 0 | No layouts, expected |
| mlwiki.org | 1 | 2 | +1 | Minor |
| mlbookcamp-page | - | 15 | new | Now builds |
| aihero | - | 2 | new | Now builds |
| little-book-of-metals-ru | - | 1 | new | Now builds |
| datatalksclub.github.io | 779 | 784 | +5 | Minor |
| courses | 0 | 5 | +5 | Pages now rendered |

## New Issues Discovered

| Issue | Description | Affected Site |
|---|---|---|
| (new) | Escaped quotes (`\"`) in include tag parameter values not parsed | DataTalksClub/docs |
