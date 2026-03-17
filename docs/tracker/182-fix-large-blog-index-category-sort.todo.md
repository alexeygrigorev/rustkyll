# Issue 182: Fix large-blog-3000 index page category sort order

## Problem

large-blog-3000's index.html lists posts grouped by category. Jekyll and rustkyll produce different category ordering, causing 128 diffs on the single index page. All 3000 individual post pages match perfectly.

Sample diff:
```
body > main > h2: text_differs
  expected: 'Technology (300 posts)'
  actual:   'Education (300 posts)'
```

## Goal

Match Jekyll's category iteration order so the index page is identical.

## Affected Sites

- large-blog-3000: 1 page (index.html) - currently 3000/3001, would become 3001/3001 (100%)

## Approach (TDD)

1. Write a test that verifies category ordering matches Jekyll's behavior
2. Verify the test fails
3. Fix category sort order in template context or generator
4. Verify the test passes
5. Run `./scripts/recount-all-dom.sh --site large-blog-3000` to confirm 3001/3001

## Acceptance Criteria

- [ ] Category listing on index pages matches Jekyll's order
- [ ] large-blog-3000 reaches 3001/3001 (100%) DOM match
