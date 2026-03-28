# Issue 427: Push DTC build time under 500ms

## Problem

DTC build is currently ~1.0s. Target: <500ms.

## Scope

Profile the DTC build to find bottlenecks. The phase timing shows
Generation at 0.619s (62% of total). Investigate:
- Template rendering hot paths
- Syntax highlighting overhead (many new postprocessing passes added)
- Collection loading (0.137s)
- Any unnecessary allocations or cloning

## Baseline

DTC build: ~1.0s. Target: <500ms.
