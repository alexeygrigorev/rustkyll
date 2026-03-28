# Issue 456: CI nightly job — DOM check all sites

## Problem

The current CI only checks DTC main (790/790). Changes can regress
other sites without detection (e.g., #449 helped muan-blog but broke
DTC docs). We need a nightly job that checks ALL sites.

## Scope

Add a new GitHub Actions workflow (`.github/workflows/nightly-dom.yml`)
that runs on a schedule (e.g., daily at 2am UTC) and:

1. Checks out rustkyll
2. Builds rustkyll in release mode
3. For each site in websites/ that has a _site_jekyll_cached/:
   - Builds with rustkyll
   - Runs dom_compare.py
   - Records the match count
4. Compares against a stored baseline file (docs/dom-baselines.json)
5. Fails if any site's match count dropped below its baseline
6. Posts results as a workflow artifact

The nightly job doesn't need to build Jekyll (uses cached output).
It only rebuilds with rustkyll and compares.

## NOT in scope

- DTC main stays in the per-push CI (fast, critical)
- The nightly job is for regression detection, not blocking PRs

## Baseline

Current site match counts to use as initial baselines.
