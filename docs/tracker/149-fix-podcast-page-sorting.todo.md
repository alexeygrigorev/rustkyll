# Issue 149: Fix podcast page sorting to match Jekyll

## Priority

CRITICAL — user-reported. Podcast episodes appear in wrong order on the podcast listing page.

## Problem

The podcast page (/podcast.html) shows episodes in a different order than Jekyll. This is visible to users and breaks the site experience.

## Goal

Podcast episodes must appear in the exact same order as Jekyll on the podcast listing page. Investigate how Jekyll sorts them (by date? by filename? by episode number?) and match exactly.

## Approach

1. Build DTC site with both Jekyll and rustkyll
2. Compare the podcast.html page — extract episode order from both
3. Identify the sorting difference
4. Fix the sort to match Jekyll
5. Verify with DOM comparison and Playwright

## Acceptance criteria

- Podcast episodes appear in same order as Jekyll on /podcast.html
- DOM comparison shows 0 diffs for podcast episode ordering
- Playwright pixel diff for /podcast.html is 0%
- No regressions on other pages
- Test: write a failing test that checks episode order, fix, test passes
