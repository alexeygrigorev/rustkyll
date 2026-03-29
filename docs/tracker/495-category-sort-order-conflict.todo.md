# Issue 495: Category sort order conflict (first-encounter vs alphabetical)

## Problem

#354 changed __key_order for site.categories to alphabetical (matching
hydeout). But #399 set it to first-encounter order (matching large-blog-3000).

large-blog-3000 regressed from 3001→3000 (index.html has 54 diffs from
wrong category order).

## Root Cause

Jekyll's category iteration order depends on context:
- In for loops: first-encounter order (order seen in posts)
- In sidebar nav: alphabetical
- Different themes expect different ordering

## Scope

Investigate what Jekyll's actual behavior is and fix. May need to keep
first-encounter as the default __key_order but sort alphabetically in
specific template contexts.

## Baseline

DTC 790/790. large-blog-3000 was 3001/3001, now 3000/3001.
