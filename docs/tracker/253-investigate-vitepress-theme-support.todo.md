# Issue 253: Investigate jekyll-vitepress-theme support

## Problem

The jekyll-vitepress-theme (https://github.com/crmne/jekyll-vitepress-theme) is a modern Jekyll theme that mimics VitePress styling. We need to evaluate how difficult it would be to support this theme and identify what rustkyll features are missing.

## Goal

1. Clone the theme and attempt to build it with rustkyll
2. Identify what works and what breaks
3. Categorize missing features (Liquid tags, filters, layout patterns, plugins)
4. Estimate effort to support it
5. Generalize findings: what common patterns block new theme support?

## Acceptance Criteria

- [ ] Theme cloned into `websites/` benchmark directory
- [ ] Build attempted with rustkyll, errors documented
- [ ] List of missing features/unsupported patterns identified
- [ ] Effort estimate (easy/medium/hard) for each gap
- [ ] Comparison with other theme gaps (from issues #235-#244) to identify common blockers
- [ ] Summary of what percentage of the theme renders correctly vs broken

## Dependencies

- None (investigation issue)
