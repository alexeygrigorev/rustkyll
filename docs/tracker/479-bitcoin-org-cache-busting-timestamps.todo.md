# Issue 479: bitcoin-org cache-busting timestamp diffs

## Problem

141/142 bitcoin-org pages differ only on cache-busting query strings
in URLs (e.g., `?1774771914` vs `?1774779647`). These are build-time
timestamps, not content differences.

## Scope

Add an acceptable-diff filter in dom_compare.py for attribute diffs
where the only difference is a numeric query string parameter.

## Baseline

DTC 790/790. bitcoin-org 1/142. Target: 142/142 with filter.
