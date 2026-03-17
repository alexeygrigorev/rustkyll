# Issue 179: Analyze all DOM differences and create comprehensive checklist

## Problem

rustkyll matches 5448/9767 (56%) HTML files across 35 benchmark sites. We need a comprehensive, categorized list of every type of DOM difference, so we can methodically eliminate them one by one using a TDD approach.

## Goal

Create a file `docs/dom-differences-checklist.md` that:

1. Lists every distinct category of DOM difference found across all benchmark sites
2. For each category: describes the root cause, which sites are affected, how many pages, and a sample diff
3. Uses checkboxes (`- [ ]` / `- [x]`) to track which categories have been fixed
4. Orders categories by impact (most pages affected first)

This checklist becomes the master tracking document. From it, we create individual tracker issues for each fix, following TDD: write a failing test first, then implement the fix.

## Approach

1. Parse all per-site diff detail files in `docs/comparison/dom-details/`
2. Categorize every diff by root cause (not just diff type)
3. Group related diffs (e.g., all "datePublished timezone" diffs across sites are one category)
4. For each category, include:
   - Description of the difference
   - Root cause
   - Sites affected and page count
   - Sample diff (one example)
   - Checkbox for tracking fix status
5. Write to `docs/dom-differences-checklist.md`

## Acceptance Criteria

- [ ] `docs/dom-differences-checklist.md` exists with all difference categories
- [ ] Every category has: description, root cause, affected sites, page count, sample diff
- [ ] Categories ordered by total page impact (descending)
- [ ] Checkboxes for each category (unchecked = not yet fixed)
- [ ] Total count of affected pages per category is accurate
- [ ] The sum of all category impacts accounts for all 4319 non-matching files

## Test Scenarios

1. Run `./scripts/recount-all-dom.sh` and verify the checklist covers all sites with diffs
2. Verify no diff category is missing by cross-checking against per-site detail files
3. Verify page counts add up
