# Issue 207: Fix content link href differences (33 pages)

## Problem

33 pages have different link hrefs. Includes URL encoding for non-ASCII, zero-width space handling, link ordering differences. Spread across many sites.

## Goal

Match Jekyll's link href generation exactly.

## Approach (TDD)

1. Categorize by sub-type (URL encoding, ordering, content)
2. Write failing tests
3. Fix each sub-type
