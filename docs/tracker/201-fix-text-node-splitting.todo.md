# Issue 201: Fix text node splitting differences (138 pages)

## Problem

138 pages have text split differently across child nodes. Text after br tags appears in wrong node, inline elements have different text boundaries. Mostly mlwiki.org (114), DTC (22), mlbookcamp-page (1), mojombo-blog (1).

## Goal

Match Jekyll's text node placement in HTML output.

## Approach (TDD)

1. Sample DTC diffs - text after br tags
2. Fix br tag text placement in kramdown post-processing
3. Handle mlwiki.org cases

## Acceptance Criteria

- [ ] Text after br tags placed correctly (sibling, not child)
- [ ] DTC text node diffs fixed
