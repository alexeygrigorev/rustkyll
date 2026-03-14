# Issue 71: Fix missing sidebar/related content in DTC site

## Problem

Structural comparison shows Jekyll includes related course headings from sidebar and related content widgets that rustkyll does not render. These come from complex Liquid includes that list other courses, related posts, etc.

This means rustkyll pages are missing content that Jekyll pages have — navigation sidebars, related content sections, course listings in sidebars, etc.

## Goal

rustkyll must render the same sidebar and related content as Jekyll. If Jekyll shows a sidebar with related courses, rustkyll must show the same sidebar with the same courses.

## Approach

1. Identify which Liquid includes produce the sidebar/related content (likely in _includes/)
2. Compare the rendered output of these includes between Jekyll and rustkyll
3. Find what's failing — could be missing Liquid features, incorrect variable resolution, or template rendering bugs
4. Fix the root causes
5. Re-run structural comparison and verify the heading differences are resolved

## Sites to verify

- DataTalksClub/datatalksclub.github.io

## Dependencies

- Issue 61 (structural comparison) done

## Acceptance criteria

- Sidebar/related content sections render the same as Jekyll
- Cross-page headings from related content widgets appear in rustkyll output
- Structural comparison heading diffs reduced to 0 (or near 0 with documented exceptions)
- All existing tests still pass
