# Issue 462: Parallel page rendering with rayon

## Problem

The generation phase renders 792 DTC pages sequentially. Each page
is independent (different content, same site context). On a multi-core
machine, parallelizing would give near-linear speedup.

## Approach

Use rayon's `par_iter` for the main page rendering loop in
`src/generator.rs`. Key challenges:
- `CachedSiteContext` must be `Send + Sync` (currently uses `RefCell`)
- Liquid template parser may not be thread-safe
- Need to verify output determinism (page order in sitemap/feed)

## Expected Impact

On a 4+ core machine: 2-4x speedup on the generation phase.
Combined with #461 (where indexes): DTC could reach 0.3-0.5s.

## Acceptance Criteria

- [ ] DTC build time < 0.5s (median of 3 runs)
- [ ] DTC DOM stays at 790/790
- [ ] Output is byte-identical to sequential rendering
- [ ] No regression on any other site

## Dependencies

- #461 (where indexes) should land first — it reduces per-page cost,
  making parallelization even more effective
