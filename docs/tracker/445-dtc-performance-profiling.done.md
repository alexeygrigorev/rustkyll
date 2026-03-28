# Issue 445: DTC performance profiling — understand the bottlenecks

## Problem

DTC builds in ~1.0s. Target is 0.5s. The previous attempt (#427) added
complexity without meaningful improvement because it optimized without
profiling.

## Approach

Profile first, optimize later. Only implement changes that give >10%
improvement.

## Scope

1. Profile DTC build with `cargo flamegraph` or `perf`
2. Identify the top 5 hotspots by CPU time
3. For each hotspot: what function, what it does, how much time, can it
   be optimized and by how much
4. Write up findings — no code changes unless a clear >10% win is obvious
5. If a clear win exists, implement it and measure before/after

## Phase Timing (current)

- Generation: 0.62s (62%) — template rendering
- Collections: 0.14s (14%) — loading/parsing
- Static files: 0.03s
- Context: 0.03s
- Pages: 0.02s

The generation phase is the bottleneck. Need to understand:
- How much time in Liquid parsing vs rendering?
- How much in markdown conversion?
- How much in syntax highlighting postprocessing?
- Which pages are slowest? (podcast pages suspected)

## Rules

- Only commit changes that give measurable >10% improvement
- Must maintain DTC 790/790 with 0 diffs
- Prefer simple changes over complex ones

## Log

### [SWE] 2026-03-28

#### Profiling Findings

**Phase timing breakdown (baseline ~1.04s):**
- Generation: 0.59s (57%) -- Liquid template rendering
- Collections: 0.11s (11%) -- I/O + markdown parsing (already parallel)
- Static files: 0.04s (4%) -- hard links (already parallel)
- Context: 0.02s (2%)
- Unaccounted (feed re-render): 0.14s (13%) -- identified as optimization target

**Per-collection CPU time (generation phase, 5.3s total across 8 cores):**
- Podcast (194 pages): 2.84s CPU -- layout rendering, avg 14.6ms/page
- People (428 pages): 1.15s CPU -- layout rendering, avg 2.7ms/page
- Posts (55 pages): 0.99s CPU -- Liquid content + markdown processing
- Books (98 pages): 0.31s CPU -- layout rendering
- Other: 0.06s CPU

**Key finding: per-step breakdown for posts with Liquid tags:**
- Liquid processing: 0.02-0.17s per post (dominant cost)
- Markdown conversion: 0.003-0.047s per post (small)
- Layout rendering: ~0ms per post (pre-compiled)
- Liquid `{% include %}` tags (faq-accordion, related-posts) are the main Liquid cost

**Key finding: feed re-render is redundant 0.135s (13.5% of build):**
Step 12 in main.rs re-rendered 15 posts through Liquid+markdown for feed content.
This duplicated the exact same work already done during generation step 9.
3 runs measured: 0.139s, 0.137s, 0.134s (median 0.137s).

#### Optimization Implemented

Eliminated the redundant feed re-render by caching intermediate rendered content
during generation. When posts with Liquid tags are rendered through the
Liquid+markdown+layout pipeline, the intermediate HTML (post-Liquid, post-markdown,
pre-layout) is now captured and stored in `GenerationResult.rendered_content`.
Step 12 now uses this cache instead of re-processing through Liquid+markdown.

New methods added to LayoutEngine:
- `render_markdown_page_with_content_capture` -- returns (layout_html, intermediate_html)
- `render_markdown_page_with_site_overrides_and_capture` -- same with site overrides

#### Measurements (5 runs each, median)

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| Wall time (median) | 1.043s | 0.851s | -18.4% |
| CPU time (median) | 6.67s | 5.94s | -10.9% |

#### Verification

- DTC DOM comparison: 790/790 (no change)
- Feed.xml: identical to baseline (no raw Liquid tags, 26 accordion/related-posts refs)
- All 3395 tests pass, 0 failures
- Clippy clean, fmt clean

#### Files modified

- `src/generator.rs` -- added `rendered_content` field to `GenerationResult`, capture logic in generation loop
- `src/template/layout.rs` -- added `render_markdown_page_with_content_capture` and `..._with_site_overrides_and_capture` methods, 2 new tests
- `src/main.rs` -- replaced feed re-render step with cache lookup
