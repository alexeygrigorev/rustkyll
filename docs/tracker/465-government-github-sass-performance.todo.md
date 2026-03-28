# Issue 465: government-github SASS compilation 3x slower than Jekyll

## Problem

government-github builds in 13.2s with rustkyll vs 4.3s with Jekyll.
The bottleneck is SASS compilation of 73 primer SCSS files from
node_modules. The grass crate is 3x slower than Jekyll's native sassc.

## Scope

1. Profile grass SASS compilation on the primer SCSS tree
2. Investigate caching compiled SASS output between builds
3. Investigate optimizing grass import resolution for node_modules
4. Target: < 4.3s (at least match Jekyll)

## Baseline

Current: 13.2s. Jekyll: 4.3s. Target: < 4.3s.
