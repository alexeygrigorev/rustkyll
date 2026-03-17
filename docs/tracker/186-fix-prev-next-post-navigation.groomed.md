# Issue 186: Fix previous/next post navigation ordering

## Checklist Category

**Content text/ordering differences (collection sort, markdown)** -- 358 pages total. This issue addresses the prev/next navigation subset (6 pages on mojombo-blog).

## Problem

mojombo-blog has 6 pages where previous/next post links point to wrong posts. The navigation ordering doesn't match Jekyll's.

Sample diff:
```
body > div > div > ul > li > span: text_differs
  expected: '22 Nov 2011'
  actual:   '21 Apr 2014'
body > div > div > ul > li > a: attribute_differs
  expected: "href='/2011/11/22/open-source-everything.html'"
  actual:   "href='/2014/04/21/farewell-github-hello-immersive-computing.html'"
```

The previous/next post links show the wrong adjacent post, suggesting the post ordering used for navigation differs from Jekyll's.

## Goal

Fix post navigation to use the same ordering as Jekyll. In Jekyll, `page.previous` is the post published before the current one (earlier date), and `page.next` is the post published after (later date). Posts are sorted by date ascending.

## Affected Sites

- mojombo-blog: 6 pages (currently 11/17 match, expected 17/17 after fix)

## Dependencies

None.

## Approach (TDD)

1. Write a test that creates 3 posts with known dates and verifies `page.previous` and `page.next` point to the correct adjacent posts
2. Verify the test fails
3. Fix navigation ordering in `src/generator.rs` (investigate whether previous/next are swapped or the sort order is wrong)
4. Verify the test passes
5. Run `./scripts/recount-all-dom.sh --site mojombo-blog` to confirm 17/17

## Acceptance Criteria

- [ ] `page.previous` points to the post with the immediately earlier date (the post before in chronological order)
- [ ] `page.next` points to the post with the immediately later date (the post after in chronological order)
- [ ] First post in chronological order has `page.previous` as nil/null
- [ ] Last post in chronological order has `page.next` as nil/null
- [ ] When multiple posts share the same date, ordering is consistent with Jekyll (alphabetical by slug or path as tiebreaker)
- [ ] mojombo-blog reaches 17/17 (100%) DOM match
- [ ] `cargo test` passes

## Test Scenarios

### Unit: Previous/next ordering (write FIRST, must fail before fix)

- **Test `test_post_previous_is_earlier_date`**: Create posts A (2023-01-01), B (2023-02-01), C (2023-03-01). Assert B.previous == A and B.next == C.
- **Test `test_first_post_has_no_previous`**: Assert A.previous is nil.
- **Test `test_last_post_has_no_next`**: Assert C.next is nil.
- **Test `test_prev_next_with_same_date_posts`**: Create posts X and Y both dated 2023-06-15. Verify they have a stable, deterministic ordering matching Jekyll.

### Regression: Other sites unaffected

- **Test `test_prev_next_dtc_blog_posts`**: Verify DTC blog posts still have correct prev/next (regression check).

### Integration: Full site verification

- Build mojombo-blog with rustkyll and run DOM comparison.
- Inspect at least 2 affected pages to verify previous/next links point to the correct posts.
- Compare the navigation links against Jekyll output for those pages.
