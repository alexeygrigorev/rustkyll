# Issue 543: Data preprocessing for `_data` auto-version resolution

## Problem

Some Jekyll themes use Ruby hooks to preprocess `_data` values at build time. For example, jekyll-vitepress-theme has `_data/versions.yml` with `current: auto`, which the `VersionLabel.apply` Ruby hook resolves to `v1.1.1` (the gem version from `Gemfile.lock`).

Since rustkyll doesn't execute Ruby hooks, the version stays as the literal string `auto`, causing 2 diffs per page on all 17 vitepress pages.

## Scope

Implement a generic data preprocessing mechanism:
- Detect `_data/versions.yml` with `current: auto`
- Read the gem version from `Gemfile.lock` (contains `jekyll-vitepress-theme (1.1.1)`)
- Resolve `auto` to `v{version}`
- Consider making this more general for other `_data` preprocessing patterns

## Dependencies

None.

## Split from

Issue #443 (jekyll-vitepress-theme rendering issues) -- RC2.

## Baseline

- DTC: 790/790 (must not regress)
- jekyll-vitepress-theme: expected to reduce diffs by 2 per page on all 17 pages
