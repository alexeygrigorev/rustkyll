# Issue 425: Push all other sites toward 100% DOM match

## Problem

DTC is at 100%. Other sites have varying match rates. We want to push
them all toward 100%.

## Current State (recount 2026-04-03)

Total: 9123/17065 across 57 sites

### Already at 100%
- alexeygrigorev/aihero: 2/2
- alexeygrigorev/alexeygrigorev.github.io: 8/8
- alexeygrigorev/kids-horror-stories-ru: 1344/1345 (1 only-Jekyll)
- alexeygrigorev/little-book-of-metals-ru: 48/48
- DataTalksClub/courses: 5/5
- lanyon: 6/6
- large-blog-3000: 3001/3001
- large-docs-site: 801/801
- jekyll-docs/lib/blank_template: 1/1

### Close to 100% (quick wins)
- muan-blog: 2178/2254 (97%) -- 36 total diffs, mostly syntax highlighting + 35 only-Jekyll/36 only-rustkyll
- type-theme: 7/8 (88%) -- 1 text_differs in search.html JS
- mojombo-blog: 14/17 (82%) -- syntax highlighting class diffs + img rendering
- DTC main: 596/790 (75%) -- ALL diffs are JSON-LD podcast dates -> #551
- beautiful-jekyll: 5/7 (71%) -- #548
- DTC docs: 38/57 (67%) -- ALL diffs are img p-wrapping -> #550
- hyde: 4/6 (67%) -- highlight tag code block rendering
- hydeout: 19/38 (50%) -- mixed issues

### False positives in comparison
- choosealicense.com: 25/72 (35%) -- ALL diffs are build timestamps -> #552

### Medium effort
- mlwiki.org: 534/645 (83%) -- kramdown tag diffs, text diffs
- homebrew-site: 85/134 (63%) -- missing elements, text diffs, liquid leaks
- text-theme: 5/11 (45%) -- attribute diffs, 5 only-rustkyll
- academicpages: 10/45 (22%) -- text/attribute/missing element diffs
- snippets: 8/25 (32%) -- attribute diffs, text diffs

### Complex (theme support needed)
- al-folio: 2/123 (2%) -- missing/extra attributes, tag diffs, 64 liquid leaks
- chirpy/jekyll-theme-chirpy: 0/17 (0%) -- structural diffs
- just-the-docs: 16/47 (34%) -- mixed diffs
- opensource-guide: 23/390 (6%) -- 3254 missing elements
- bitcoin-org: 1/3577 (0%) -- massive, 2602 only-Jekyll pages
- documentation-theme-jekyll: 3/98 (3%) -- text/element diffs, 31 liquid leaks
- many theme-sample sites at 0% (architect, cayman, dinky, hacker, leap-day, merlot, midnight, primer, slate, time-machine)

## Scope

This is a parent tracking issue. Create individual issues for each site
as we prioritize them. Follow the same micro-issue decomposition approach
that worked for DTC.

## Approach

1. Pick the closest-to-100% site
2. Analyze its diffs
3. Decompose into micro-issues by root cause
4. Implement one at a time with zero-DTC-regression rule
5. Once a site reaches 100%, add it to CI DOM checks
