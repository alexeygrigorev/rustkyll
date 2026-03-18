# Issue 204: Fix extra HTML elements in rustkyll output (90 pages)

## Problem

90 pages have extra HTML elements not in Jekyll output. DTC (17), mlwiki.org (56), muan-blog (16), mlbookcamp-page (1).

## Goal

Remove extra wrapper elements to match Jekyll output.

## Approach (TDD)

1. Write failing tests with sample HTML showing extra elements
2. Fix kramdown post-processing or markdown rendering
3. Verify tests pass
