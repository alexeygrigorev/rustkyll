# Issue 200: Fix markdown table rendering failure (109 pages)

## Problem

109 pages have tables not rendered as HTML table elements. Mostly mlwiki.org (108) plus 1 DTC page. Tables inside list items or with non-standard formatting (wiki-style, leading pipes) fail to render.

## Goal

Render markdown tables correctly in all contexts.

## Approach (TDD)

1. Sample failing tables from mlwiki.org and DTC
2. Write tests reproducing each table format
3. Fix pulldown-cmark table handling or add post-processing in kramdown.rs
4. Tables inside list items may need special handling

## Acceptance Criteria

- [ ] Standard pipe tables render correctly
- [ ] Tables inside list items render correctly
- [ ] DTC table case fixed
- [ ] mlwiki.org tables: fix what's feasible with standard markdown
