# Issue 269: large-blog-3000 index page category sort order

## Problem

large-blog-3000 matches 3000/3001 pages. The only failing page is `index.html`, which has a category listing with a different sort order than Jekyll.

## Impact

Fixes the last 1 page to achieve 100% DOM match on large-blog-3000.

## Acceptance Criteria

- [ ] large-blog-3000 index.html DOM matches Jekyll
- [ ] 3001/3001 pages match
- [ ] No regressions on other sites
