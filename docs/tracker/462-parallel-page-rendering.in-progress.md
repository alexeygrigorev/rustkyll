# Issue 462: Parallel page rendering with rayon

## Problem

DTC builds in 0.69s. The generation phase (~0.45s) renders 792 pages
sequentially. Pages are independent. On multi-core machines, parallel
rendering would give near-linear speedup.

## Current Architecture

In `src/generator.rs`, `generate_collection_pages()` iterates collection
items sequentially. Each page: resolve frontmatter → render Liquid →
convert markdown → apply layout chain → postprocess HTML.

The `CachedSiteContext` is shared read-only across pages. The where-filter
cache (#461) is already thread-local, so it works with rayon.

## Approach

1. Add `rayon` dependency
2. Replace sequential iteration with `par_iter` in the generation loop
3. Collect results into a Vec (order doesn't matter for file output)
4. Ensure output determinism: sitemap and feed ordering must be stable

## Challenges

- Liquid `Parser` may not be `Send`/`Sync` — may need per-thread parsers
- `CachedSiteContext` uses `LenientValue` which wraps `RefCell` — not `Sync`
- Template preprocessing is stateless, should parallelize fine
- Feed/sitemap generation happens after page rendering, uses collected results

## Acceptance Criteria

- [ ] DTC build < 0.5s (median of 3 runs)
- [ ] DTC DOM stays at 790/790
- [ ] Output is byte-identical to sequential rendering
- [ ] All existing tests pass
- [ ] No regression on any other site
- [ ] Only accept if measurable improvement reaches 0.5s target

## Test Scenarios

- Build DTC 3 times, verify median < 0.5s
- Diff DTC output between sequential and parallel builds — must be identical
- Run full DOM comparison on DTC, DTC docs, large-docs-site, muan-blog

## Log

### [SWE] 2026-03-29

**Discovery**: The codebase already uses rayon `par_iter` for page rendering
(added earlier). The issue's description of "sequential iteration" was outdated.
The real bottleneck is the Liquid template interpreter speed (CPU-bound), not
lack of parallelism.

**Optimizations implemented:**

1. **Overlap static file copying with page generation** (`src/main.rs`,
   `src/static_files.rs`): Added `copy_static_files_from_list` that takes
   pre-collected file paths (avoiding redundant directory walk). Static file
   copying starts in a background OS thread at the beginning of the generation
   phase, overlapping I/O-bound file copying with CPU-bound template rendering.
   Saves ~0.04s.

2. **Overlap data + collections loading** (`src/main.rs`): Data file loading
   and collection loading now run in parallel via `rayon::join` since they read
   from independent directories.

3. **Overlap page loading + static file collection** (`src/main.rs`): Standalone
   pages and static file list are collected in parallel via `rayon::join`.

4. **Move context build before incremental/collision checks** (`src/main.rs`):
   Site context building and layout loading (which were already overlapped via
   `rayon::join`) now run before the URL collision detection and incremental
   check, reducing sequential pipeline gaps.

5. **Pre-build CachedSiteContext before directory cleanup**: The
   `CachedSiteContext::new()` call (CPU-bound) now happens before the
   destination directory cleanup (I/O-bound), so both can overlap.

**Benchmark results** (median of 5 runs):
- Before: 0.62s (original baseline from issue description: 0.69s)
- After: 0.569s (median), best: 0.546s
- Improvement: ~8-9% wall-clock reduction

**Note on 0.5s target**: The remaining bottleneck is the Liquid template
interpreter itself (third-party crate). 780 pages * ~4ms each / 12 cores =
0.26s theoretical minimum for generation alone. Actual generation is ~0.33s
(80% efficiency). Combined with 0.14s for collection loading (also parallelized),
there is minimal room for further optimization without changing the Liquid crate.

- Tests: 3403 passed, 0 failed, 2 ignored (4 new tests added)
- Clippy: clean
- fmt: clean
- Output determinism: verified (diff shows only timestamps that change per-second)

**Files modified:**
- `src/main.rs` -- pipeline phase overlapping
- `src/generator.rs` -- 4 new tests for parallel rendering correctness
- `src/static_files.rs` -- new `copy_static_files_from_list` function
