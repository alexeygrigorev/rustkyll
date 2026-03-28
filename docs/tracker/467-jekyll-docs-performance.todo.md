# Issue 467: jekyll-docs build only 2.5x faster than Jekyll

## Problem

jekyll-docs builds in 1.2s vs 3.0s Jekyll (2.5x). Target is 10x.
Bottleneck: Collections 0.7s + Generation 0.6s for 131 pages.

## Scope

Investigate and optimize. Target: < 0.3s (10x).

## Baseline

Current: 1.2s. Jekyll: 3.0s. Target: < 0.3s.
