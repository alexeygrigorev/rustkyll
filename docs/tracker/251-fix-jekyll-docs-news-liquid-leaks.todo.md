# Issue 251: Fix jekyll-docs news page liquid leaks

## Problem

`news/index.html` and `news/releases/index.html` in jekyll-docs produce raw Liquid output instead of rendered HTML. These pages DO have front matter and ARE processed through Liquid, but included templates (`news_item.html`, `news_item_archive.html`) fail to render, causing the generator to write raw content as fallback.

## Root Cause

This was originally part of issue 230 RC2, but the SWE investigation found the actual root cause is include rendering errors (not missing front matter as initially hypothesized). When an included template fails to render, the fallback mechanism outputs raw Liquid tags.

## Acceptance Criteria

- [ ] `news/index.html` renders without liquid leaks
- [ ] `news/releases/index.html` renders without liquid leaks
- [ ] All tests pass

## Dependencies

- Issue 230 (done)
