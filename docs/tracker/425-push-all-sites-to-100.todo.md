# Issue 425: Push all other sites toward 100% DOM match

## Problem

DTC is at 100%. Other sites have varying match rates. We want to push
them all toward 100%.

## Current State (from last full recount)

### Already at 100%
- alexeygrigorev.github.io: 8/8
- kids-horror-stories-ru: 1344/1344
- DTC courses: 5/5
- DTC main: 790/790
- large-blog-3000: 3001/3001
- large-docs-site: 801/801
- jekyll blank_template: 1/1
- choosealicense.com: 72/72
- mojombo-blog: 17/17
- little-book-of-metals-ru: 48/48

### Close to 100% (quick wins)
- muan-blog: 2194/2218 (99%)
- lanyon: 5/6 (83%)
- type-theme: 5/8 (62%)
- beautiful-jekyll: 3/5 (60%)
- DTC docs: 47/57 (82%)

### Medium effort
- mlwiki.org: 576/644 (89%)
- mlbookcamp-page: 6/15 (40%)
- academicpages: 10/45 (22%)

### Complex (theme support needed)
- al-folio, chirpy, just-the-docs, etc.

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
