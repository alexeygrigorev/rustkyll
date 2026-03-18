# Issue 199: Fix markdown block structure differences (335 pages)

## Problem

335 pages have HTML element structure differences from markdown rendering. Largest: mlwiki.org (235), DTC (26), muan-blog (53), plus smaller counts on theme sites.

Root causes: image references with markdown links, nested lists, definition lists, complex block structures rendered differently by pulldown-cmark vs kramdown.

## Goal

Fix markdown block structure to match kramdown output.

## Approach (TDD)

1. Categorize diffs by specific markdown pattern
2. Fix each pattern in src/kramdown.rs post-processing
3. Focus on DTC and muan-blog patterns first (most impactful)

## Acceptance Criteria

- [ ] DTC block structure diffs fixed
- [ ] muan-blog block structure diffs categorized and fixed where feasible
- [ ] mlwiki.org: categorize and fix what's feasible
