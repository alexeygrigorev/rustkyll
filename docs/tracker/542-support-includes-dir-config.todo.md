# Issue 542: Support `includes_dir` config setting

## Problem

Rustkyll hardcodes the includes directory as `_includes/` (in `src/main.rs:509`). Jekyll supports the `includes_dir` config option in `_config.yml`, which allows sites to override the default. Sites like jekyll-vitepress-theme use `includes_dir: docs/_includes` to load analytics scripts from a different location.

This causes missing `<script>` tags (3 diffs per page) on jekyll-vitepress-theme because the override includes directory contains Plausible analytics scripts that aren't found.

## Scope

Implement support for the `includes_dir` config setting:
- Parse `includes_dir` from `_config.yml`
- Use the configured path when resolving `{% include %}` tags
- Default to `_includes/` if not set (current behavior)

## Dependencies

None.

## Split from

Issue #443 (jekyll-vitepress-theme rendering issues) -- RC1b.

## Baseline

- DTC: 790/790 (must not regress)
- jekyll-vitepress-theme: expected to reduce diffs by 3 per page on all 17 pages
