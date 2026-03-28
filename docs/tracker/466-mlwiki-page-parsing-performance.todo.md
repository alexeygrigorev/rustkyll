# Issue 466: mlwiki.org page parsing 2x slower than Jekyll

## Problem

mlwiki.org builds in 1.8s with rustkyll vs 0.98s with Jekyll.
The bottleneck is the Pages phase: 645 standalone pages at ~2.8ms each.
Page parsing (frontmatter extraction, markdown processing) has high
per-page overhead.

## Scope

1. Profile page parsing for mlwiki — where is time spent per page?
2. Is it YAML parsing? Markdown conversion? File I/O?
3. Investigate batching or parallelizing page loading
4. Target: < 0.98s (at least match Jekyll)

## Baseline

Current: 1.8s. Jekyll: 0.98s. Target: < 0.98s.
