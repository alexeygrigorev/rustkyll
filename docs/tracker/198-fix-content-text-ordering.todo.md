# Issue 198: Fix content text/ordering differences (358 pages)

## Problem

358 pages have text content or ordering differences. Largest: mlwiki.org (325), DTC (24), mojombo-blog (4), kids-horror (2), large-blog-3000 (1), alexeygrigorev.github.io (1), mlbookcamp-page (1).

Root causes: collection sort differences, markdown text splitting, front matter value processing.

## Goal

Match Jekyll's text output for all affected pages.

## Approach (TDD)

1. mlwiki.org (325): MediaWiki-style markup ('''bold''', ''italic'') not supported. Investigate scope.
2. DTC (24): Zero-width space handling around emphasis, text after br tags.
3. mojombo-blog (4): Post content differences.
4. kids-horror (2): Unicode quote normalization.

## Acceptance Criteria

- [ ] DTC text content matches Jekyll
- [ ] mojombo-blog text diffs fixed
- [ ] kids-horror quote normalization fixed
- [ ] mlwiki.org: categorize and fix what's feasible
