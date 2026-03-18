# Issue 205: Fix other attribute differences (85 pages)

## Problem

85 pages have attribute differences not covered by other issues. Includes: Cyrillic heading IDs (little-book-of-metals-ru 33 pages), alt text whitespace (mlwiki.org 48), other attribute values (mlbookcamp-page 3, mojombo-blog 1).

Key sub-issue: little-book-of-metals-ru heading IDs produce '-1-------' instead of the Cyrillic slug. This is a separate bug in slugify for non-ASCII characters.

## Goal

Fix attribute generation to match Jekyll.

## Approach (TDD)

1. Fix Cyrillic slugify for heading IDs
2. Fix alt text whitespace normalization
3. Write failing tests first for each case
