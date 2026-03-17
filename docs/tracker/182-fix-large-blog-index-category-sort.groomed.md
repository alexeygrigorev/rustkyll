# Issue 182: Fix large-blog-3000 index page category sort order

## Checklist Category

**Content text/ordering differences (collection sort, markdown)** -- 358 pages total. This issue addresses the category sort ordering subset (1 page on large-blog-3000). Other content/ordering diffs are separate issues.

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

- large-blog-3000: 1 page (index.html) -- currently 3000/3001, would become 3001/3001 (100%)

## Dependencies

None.

## Approach (TDD)

1. Write a test that creates a site with multiple categories and verifies `site.categories` iterates in the same order as Jekyll (insertion order / alphabetical -- investigate which)
2. Verify the test fails
3. Fix category sort order in template context or generator
4. Verify the test passes
5. Run `./scripts/recount-all-dom.sh --site large-blog-3000` to confirm 3001/3001

## Acceptance Criteria

- [ ] `site.categories` iterates in the same order as Jekyll (investigation needed: Jekyll uses hash insertion order which is alphabetical in Ruby 1.9+)
- [ ] Category listing on index pages matches Jekyll's order
- [ ] large-blog-3000 reaches 3001/3001 (100%) DOM match
- [ ] `cargo test` passes
- [ ] Other sites with category listings are not regressed

## Test Scenarios

### Unit: Category iteration order (write FIRST, must fail before fix)

- **Test `test_categories_alphabetical_order`**: Create a site with posts in categories "Technology", "Education", "Health", "Business". Assert `site.categories` keys iterate in alphabetical order: Business, Education, Health, Technology.
- **Test `test_categories_single_category`**: One category only -- trivially correct.
- **Test `test_categories_case_sensitivity`**: Verify categories with different cases sort correctly (Jekyll is case-sensitive in category names).

### Integration: Full site verification

- Build large-blog-3000 with rustkyll and run DOM comparison.
- Verify index.html category headings appear in the same order as Jekyll output.
- Inspect the HTML source of index.html to confirm category ordering matches.
