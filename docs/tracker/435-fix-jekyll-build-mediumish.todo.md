# Issue 435: Fix Jekyll build for mediumish

## Problem

The mediumish site fails to build with Jekyll, preventing DOM comparison.
The recount script marks it as JEKYLL_FAIL.

## Scope

1. Investigate why `bundle exec jekyll build` fails for this site
2. Fix gem dependencies (update Gemfile.lock, pin compatible versions)
3. Build with Jekyll and cache the output in `_site_jekyll_cached/`
4. Build with rustkyll and run DOM comparison
5. Record the initial DOM match rate

## Baseline

Currently: JEKYLL_FAIL (no comparison possible)
Target: Get a DOM comparison baseline, then push toward 100%
