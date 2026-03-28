# Issue 426: Performance audit — ensure 10x faster than Jekyll across all sites

## Problem

DTC build is at 1.00s (target: <1.0s). Some sites may be slower than
Jekyll. We need a full benchmark and optimization pass.

## Scope

1. Benchmark all sites: rustkyll vs Jekyll build time
2. Identify any sites where rustkyll is slower than Jekyll
3. Fix performance regressions to achieve 10x faster across the board
4. Bring DTC back under 1.0s if the recent syntax changes slowed it

## Baseline

DTC: 1.00s (must be <1.0s)
