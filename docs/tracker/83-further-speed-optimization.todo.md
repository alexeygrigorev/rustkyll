# Issue 83: Further speed optimization while maintaining pixel-perfect generation

## Problem

rustkyll is already 18.5x faster than Jekyll on the DTC site (1.05s vs 19.4s). But there's room for more — especially on larger sites and for the overall developer experience.

## Goal

Make rustkyll even faster while maintaining pixel-perfect output. Speed must never come at the cost of correctness.

## Potential optimizations to investigate

1. **Parallel page rendering** — use rayon to render pages across CPU cores (currently single-threaded)
2. **Template compilation cache** — compile templates once, reuse across pages
3. **Lazy site context** — only build parts of site context that templates actually access
4. **Incremental rendering** — only re-render pages whose dependencies changed
5. **Profile-guided optimization** — use flamegraph to find remaining hot spots
6. **Custom Liquid renderer** — replace the liquid crate entirely for maximum control (Option B from issue #49)
7. **Memory-mapped file I/O** — reduce syscall overhead for reading source files
8. **SCSS caching** — compile SCSS once, not per-build

## Constraints

- Output must remain pixel-perfect with Jekyll (0% Playwright diff)
- Structural comparison must pass (exact page counts, same elements)
- All existing tests must pass
- Performance improvements must be measured with the benchmark script
- Before and after numbers documented

## Dependencies

- Issue 73 (kramdown compatibility) should be done first
- Issue 82 (expanded test sites) provides the validation suite

## Acceptance criteria

- DTC site builds in under 0.5s (currently 1.05s)
- kids-horror-stories-ru builds in under 0.2s (currently 0.4s)
- All benchmark sites faster or same speed (no regressions)
- Pixel-perfect output maintained (0% Playwright diff on all tested pages)
- Structural comparison passes for all sites
- Profiling results documented (before/after flamegraphs)
- Benchmark results updated with new numbers
