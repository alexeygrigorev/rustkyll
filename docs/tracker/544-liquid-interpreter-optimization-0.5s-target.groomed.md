# Issue 544: Liquid interpreter optimization for sub-0.5s DTC builds

**Descoped from:** #462 (Parallel page rendering with rayon)

## Problem

Issue 462 added rayon parallelization and pipeline overlapping. The remaining bottleneck is the Liquid template interpreter. The generation phase (which is dominated by Liquid rendering) accounts for ~0.36s of a ~0.66s median build (warm cache, release build, 12-core machine).

## Profiling Baseline (2026-04-02)

Measured on the grooming machine (5 warm-cache runs, release build):

| Metric | Value |
|--------|-------|
| Total build (median of 5) | 0.86s |
| Total build (warm, median of 3) | 0.66s |
| Generation phase (warm) | 0.35-0.39s |
| Collections phase (warm) | 0.10s |
| Static files | 0.05s |
| Other (config, data, context, sitemap, feed) | ~0.07s |
| Pages with Liquid tags | 31 of 840 content files |
| Total rendered pages | 792 |
| DOM baseline | 596/790 matched, 194 with diffs, 255 total diffs |

## Feasibility Analysis

### What is already optimized
1. **Layout template ASTs are pre-compiled** -- `compiled_layouts` HashMap caches parsed templates, reused across all 792 page renders
2. **Site context is cached** -- `CachedSiteContext` with lazy `OnceLock`-based `LenientValue` avoids redundant deep cloning
3. **Pages without Liquid tags skip parsing** -- `contains("{{")` / `contains("{%")` early return means ~750 of 780 collection items skip Liquid parsing entirely for their body content
4. **Parallel rendering via rayon** -- pages are rendered in parallel across cores
5. **Layout Objects pre-built** -- `layout_objects` HashMap avoids per-page yaml_to_liquid conversion

### Where remaining time goes (Generation = ~0.36s)
The 792 pages all go through layout rendering even if their body has no Liquid. The DTC site has 2-3 layout templates that are pre-compiled but rendered per-page with different `page.*` context. Each render involves:
- Building per-page `LenientObject` context
- Liquid template interpreter executing the pre-compiled AST
- Post-processing (HTML normalization, kramdown compat)

### What could still be optimized
1. **Consolidate 11 preprocessing passes into 1** -- `engine.rs::parse()` runs 11 sequential string-scanning passes (preprocess_capture_tags, preprocess_jekyll_tags, etc.). These could be merged into a single pass. However, most page bodies skip this entirely (no Liquid tags), and layout templates are only parsed once. **Estimated impact: negligible (<5ms).**

2. **Reduce per-page context construction cost** -- Each page builds a fresh `LenientObject`. The `build_page_object()` + `build_render_context_from_page_object()` chain allocates and converts front matter. Could be optimized with arena allocation or pre-converted page objects. **Estimated impact: 10-30ms.**

3. **Liquid interpreter hot path optimization** -- The vendored `liquid-core` and `liquid-lib` crates could be profiled and optimized (e.g., reducing allocations in the render loop, using SmallVec for short arrays, caching filter chain results). **Estimated impact: 20-50ms.**

4. **Template output pre-sizing** -- Pre-allocate output strings based on expected size (layout size + content size). **Estimated impact: 5-10ms.**

5. **Skip HTML normalization for simple pages** -- Many normalization passes (kramdown compat, boolean attributes) could be skipped for pages that don't need them. **Estimated impact: 10-20ms.**

6. **Replace `liquid` crate with custom interpreter** -- The nuclear option. Would require reimplementing all 40+ custom tags and filters. The liquid crate is vendored with local patches. **Estimated impact: potentially 100-150ms but 2000+ lines of code and high risk.**

### Verdict on 0.5s target

The 0.5s target requires cutting total build time from ~0.66s to 0.50s -- a 24% reduction. The generation phase would need to drop from ~0.36s to ~0.20s -- a 44% reduction. 

**This is not achievable with incremental improvements.** The major optimizations (template caching, context caching, parallel rendering, body-skip) are already implemented. The remaining gains from items 1-5 above total an optimistic 45-110ms, which would bring the build to ~0.55-0.62s -- close but not reliably below 0.5s.

**A full liquid interpreter replacement (item 6) could theoretically reach the target but is a disproportionate effort** -- 2000+ lines of new code, reimplementation of 40+ tags/filters, high regression risk, and months of stabilization. The vendored liquid-core already has local patches; a custom interpreter would need to replicate all of those behaviors.

## Recommendation

**Split this into two phases:**

### Phase A: Low-hanging fruit optimizations (this issue)
Target: bring warm-cache median below 0.60s (from current ~0.66s). Concrete items:
1. Profile generation phase with `perf` or `flamegraph` to identify actual hot spots
2. Optimize per-page context construction (reduce allocations)
3. Pre-size template output buffers
4. Skip unnecessary HTML normalization passes where possible
5. Profile and optimize vendored liquid-core render loop if flamegraph shows hot spots

### Phase B: Custom interpreter (separate future issue, if ever needed)
Only pursue if Phase A results are insufficient AND users report DTC build speed as a real problem. The current ~0.66s is already very fast for 792 pages.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes (all existing tests)
- [ ] DTC full build median (3 warm-cache runs, release) < 0.60s
- [ ] DTC DOM match count does not drop below baseline: 596 files matched, 255 total diffs (must not increase diffs)
- [ ] No site-specific hardcoding -- optimizations must be generic
- [ ] Generated HTML is byte-identical to pre-optimization output for DTC site (diff the output directories)
- [ ] Flamegraph or profiling data included in the issue log showing where time was saved

## Test Scenarios

### Unit: Context construction optimization
- Build page context for a page with 10 front matter keys, verify no regression in output
- Build page context for a page with empty front matter, verify no crash
- Verify LenientValue lazy initialization still works correctly after any refactoring

### Unit: Output buffer pre-sizing
- Render a template with known layout size and content, verify output is correct
- Verify pre-sized buffer does not truncate or corrupt output

### Integration: Full DTC build correctness
- Build DTC site before and after optimization
- Diff all generated HTML files -- must be byte-identical
- Run DOM comparison -- diffs must not increase

### Performance: Timing verification
- Run 3 warm-cache DTC builds, take median
- Median must be < 0.60s
- Generation phase must show measurable improvement over baseline (~0.36s)

## Dependencies

- #462 (parallel rendering -- done)

## Notes

- The `liquid` crate is deeply integrated: vendored `liquid-core` and `liquid-lib` with local patches, 40+ custom tags (seo_tag, include_tag, highlight_tag, avatar_tag, feed_meta_tag, details_tag, file_exists_tag, gist_tag, noop_tags, octicon_tag) and 30+ custom filters
- Current performance is already very good: 792 pages in 0.66s = 0.83ms/page average
- The 0.5s target from the original issue is not realistically achievable without a full interpreter rewrite, which is not worth the effort at this time
- Adjusted target to 0.60s which is achievable with incremental optimizations
- If 0.60s is not reached, the issue should be closed with a note that current performance is sufficient

## Log

### [PM] 2026-04-02 Grooming
- Read issue, profiled DTC build extensively
- Current baseline: 0.66s median (warm), generation 0.36s
- All major optimizations already in place (template caching, context caching, parallel rendering, body-skip)
- Liquid crate deeply integrated: vendored with patches, 40+ custom tags/filters
- Adjusted target from 0.5s to 0.60s (original target infeasible without full rewrite)
- Recommended Phase A (incremental, this issue) + Phase B (rewrite, future issue if needed)
- DOM baseline recorded: 596 matched, 255 diffs
