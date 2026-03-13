# Issue 31: Build Performance Optimization

## Problem

Full site build takes ~16s in release mode (777 pages + 1454 static files). Page generation already uses `par_iter()`, but other phases may be sequential.

## Requirements

- Profile the build to identify bottlenecks per phase (data loading, collection loading, context building, page generation, static file copying, sitemap/feed)
- Parallelize static file copying (1454 files currently copied sequentially)
- Investigate parallel collection loading
- Investigate whether sitemap/feed generation can overlap with static file copying
- Reduce unnecessary cloning (e.g., `items_to_build.into_iter().cloned().collect()` in main.rs)
- Target: measurable wall-clock improvement (document before/after times)
- All existing tests must continue to pass
