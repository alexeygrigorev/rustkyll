# Issue 457: CI — add DTC docs to per-push DOM check

## Problem

DTC docs (48/57) should be checked in CI alongside DTC main (790/790)
to prevent regressions. It uses the same DTC repo.

## Scope

Extend the existing dom-check CI job to also:
1. Build DTC docs site with Jekyll and rustkyll
2. Run dom_compare.py on DTC docs
3. Assert >= 48/57 matched

The DTC docs site is at websites/DataTalksClub/docs/ — a subdirectory
of the DTC repo (already cloned in CI).

## Baseline

DTC docs: 48/57.
