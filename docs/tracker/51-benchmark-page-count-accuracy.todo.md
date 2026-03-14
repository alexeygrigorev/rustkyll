# Issue 51: Fix benchmark page count accuracy

## Problem

Several sites in the benchmark report suspicious page counts:

- bitcoin-org: "?" (unknown)
- edition-template: "?" (unknown)
- data-science-interviews: 0 pages (builds but produces nothing?)
- academicpages: 1 page (seems too low)
- minimal-mistakes: 1 page (seems too low)
- beautiful-jekyll: 3 pages (might be too low)

The page count should reflect the actual number of HTML files generated, and sites that produce 0 pages should be investigated.

## Goal

Ensure the benchmark script accurately counts pages and that sites producing 0 or suspiciously low page counts are investigated.

## Dependencies

- Issue 49 (large-site performance) -- DTC site needs to build first

## Acceptance criteria

- Benchmark script counts pages correctly (count HTML files in _site/)
- Sites producing 0 pages are either fixed or documented as intentionally empty
- "?" entries are resolved
- Updated benchmark results reflect accurate page counts
