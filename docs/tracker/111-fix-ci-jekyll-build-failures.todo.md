# Issue 111: Fix CI Jekyll build failures in vs_jekyll tests

## Problem

CI integration tests fail because Jekyll crashes when building sites. The `bundle install` step doesn't install all required gems, or the gem versions are incompatible.

Failing tests: test_dtc_feed_vs_jekyll, test_dtc_sitemap_vs_jekyll, test_kids_podcast_vs_jekyll

## Goal

Make Jekyll builds work in CI so all vs_jekyll comparison tests pass.

## Approach

1. Check the Jekyll error output in CI logs
2. Fix Gemfile/gem installation issues
3. Ensure bundle exec jekyll build works for DTC and kids sites in CI
4. All vs_jekyll tests must pass

## Acceptance criteria

- All 3 vs_jekyll tests pass in CI
- Jekyll builds DTC site in CI without errors
- Jekyll builds kids-horror-stories-ru in CI without errors
- CI is fully green (zero failures)
