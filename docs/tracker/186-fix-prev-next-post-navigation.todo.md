# Issue 186: Fix previous/next post navigation ordering

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

The previous/next post links show the current post instead of the adjacent one.

## Goal

Fix post navigation to use the same ordering as Jekyll.

## Affected Sites

- mojombo-blog: 6 pages (currently 11/17 match, expected 17/17 after fix)

## Approach (TDD)

1. Write a test that verifies previous/next post variables for a known post ordering
2. Verify the test fails
3. Fix navigation ordering in `src/generator.rs`
4. Verify the test passes
5. Run `./scripts/recount-all-dom.sh --site mojombo-blog` to confirm 17/17

## Acceptance Criteria

- [ ] `page.previous` and `page.next` match Jekyll's ordering
- [ ] mojombo-blog reaches 17/17 (100%) DOM match
