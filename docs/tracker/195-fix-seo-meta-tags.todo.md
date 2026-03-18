# Issue 195: Fix SEO meta tag differences (og:, twitter:, description)

## Problem

2236 pages have SEO meta tag differences. This is the #1 category by page count. Most are on muan-blog (2163), plus choosealicense (16), government-github (11), opensource-guide (28), jekyll-docs (14), DTC (2), aihero (2).

The diffs include: meta tag ordering, og:type values, twitter:card values, description content, og:url, og:image differences.

## Goal

Match jekyll-seo-tag meta tag output exactly: same tags, same order, same content.

## Approach (TDD)

Break into sub-tasks by site:
1. Analyze muan-blog meta tag diffs specifically (it uses a custom SEO setup, not jekyll-seo-tag)
2. Fix og:type, twitter:card, description for sites using jekyll-seo-tag
3. Fix meta tag ordering to match Jekyll

## Acceptance Criteria

- [ ] Meta tag content matches Jekyll for DTC pages
- [ ] Meta tag content matches Jekyll for theme sites
- [ ] muan-blog meta tags match (may need custom handling)
