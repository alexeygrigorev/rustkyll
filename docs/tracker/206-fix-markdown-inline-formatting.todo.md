# Issue 206: Fix markdown inline formatting not applied (21 pages)

## Problem

21 pages have inline markdown not converted to HTML. Missing em, strong, a elements. DTC (9), mlwiki.org (6), government-github (3), jekyll-docs (2), mojombo-blog (1).

## Goal

Apply markdown formatting in all contexts where Jekyll does.

## Approach (TDD)

1. Sample affected pages to find common pattern
2. Write failing tests
3. Fix - likely content in Liquid output not being passed through markdownify
