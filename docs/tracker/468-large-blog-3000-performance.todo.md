# Issue 468: large-blog-3000 build only 3.7x faster than Jekyll

## Problem

large-blog-3000 builds in 1.2s vs 4.4s Jekyll (3.7x). Target is 10x.
Bottleneck: template rendering at scale (3001 pages).

## Scope

Investigate and optimize. The where-filter indexing (#461) may have
already helped. Re-benchmark after #461. Target: < 0.44s (10x).

## Baseline

Current: 1.2s. Jekyll: 4.4s. Target: < 0.44s.
