# Issue 544: Liquid interpreter optimization for sub-0.5s DTC builds

**Descoped from:** #462 (Parallel page rendering with rayon)

## Problem

Issue 462 added rayon parallelization and pipeline overlapping, bringing DTC builds from ~0.68s to ~0.56s (median). The remaining bottleneck is the Liquid template interpreter (third-party `liquid` crate), which accounts for ~85% of the generation phase. The theoretical minimum with current Liquid is ~0.26s for generation alone (780 pages × 4ms / 12 cores), plus ~0.3s fixed overhead = ~0.56s practical minimum.

To reach the original 0.5s target from issue 462, the Liquid interpreter needs to be replaced or augmented.

## Approach Options

1. **Template bytecode compilation** — Pre-compile Liquid templates to a bytecode representation, avoiding re-parsing and string matching on each render. Could reduce per-page render time by 2-3x.
2. **Faster Liquid interpreter** — Replace the `liquid` crate with a custom interpreter optimized for Jekyll's subset of Liquid (no need for full Liquid spec).
3. **Template caching** — Cache parsed ASTs across pages that share the same layout template (most posts share 2-3 layouts).
4. **Incremental builds** — Already partially done; for full rebuilds, the Liquid overhead remains.

## Acceptance Criteria

- [ ] DTC full build < 0.5s (median of 3 runs, release build)
- [ ] DTC DOM stays at 790/790
- [ ] Output is byte-identical to current rendering
- [ ] All existing tests pass

## Dependencies

- #462 (parallel rendering — done)

## Notes

This is a significant undertaking. The `liquid` crate is deeply integrated. Consider whether the effort is justified given that incremental builds already achieve sub-0.1s for partial rebuilds.
