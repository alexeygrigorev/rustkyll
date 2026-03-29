# Issue 489: academicpages TOC list generation
## Problem
TOC nav not generating ul structure. 2 pages.
## Affected Sites
- academicpages
## Baseline
DTC 790/790. academicpages 27/45. Must not regress.

## Status: RESOLVED

As of 2026-03-29, both terms/index.html and markdown/index.html TOC render
correctly. The kramdown TOC generation in markdown="1" blocks is working.
