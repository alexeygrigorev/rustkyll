# Issue 359: SCSS `@import` with `sass_dir` load path not supported by grass compiler

## Problem

Jekyll's SCSS pipeline respects the `sass_dir` configuration (default: `_sass`) as a load path for `@import` directives. When a SCSS file uses `@import "file"`, Jekyll resolves it relative to the configured `sass_dir`. Rustkyll's grass-based SCSS compilation does not pass `sass_dir` as a load path, causing `@import` resolution failures.

Discovered in the Type theme (`websites/type-theme/`), where `css/main.scss` imports partials from `_sass/`. The build produces a warning and falls back to no CSS compilation for the affected stylesheet.

Related to issue #244 (Type theme support).

## Impact

Any Jekyll site that uses `@import` in SCSS files with partials in `_sass/` (the vast majority of Jekyll sites with custom SCSS) may fail to compile stylesheets correctly.

## Possible Fix

Pass the site's `sass_dir` (resolved to an absolute path) as a load path to the grass compiler's `Options`.
