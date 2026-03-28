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
