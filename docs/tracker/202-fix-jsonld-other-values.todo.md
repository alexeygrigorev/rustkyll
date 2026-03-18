# Issue 202: Fix JSON-LD other value differences (212 pages)

## Problem

212 pages have various JSON-LD field value differences. Mostly DTC (202), plus theme sites (1 each). Includes: description truncation, headline formatting, special character handling ($ getting stripped).

## Goal

Match jekyll-seo-tag JSON-LD output for all field values.

## Approach (TDD)

1. Sample DTC JSON-LD diffs to categorize sub-types
2. Fix description truncation logic
3. Fix special character handling in JSON-LD values
4. Fix headline formatting

## Acceptance Criteria

- [ ] JSON-LD description matches jekyll-seo-tag truncation
- [ ] Special characters preserved correctly
- [ ] DTC JSON-LD values match Jekyll
