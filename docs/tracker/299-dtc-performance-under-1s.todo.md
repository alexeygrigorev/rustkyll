# Issue 299: DTC build performance < 1.0s

## Problem

DTC builds in 1.23s, target is < 1.0s. Liquid template rendering is the bottleneck (0.75s for 789 pages). Issue 295 brought it from 1.7s to 1.23s.

## Acceptance Criteria

- [ ] DTC builds in < 1.0s (release mode)
- [ ] No regressions
