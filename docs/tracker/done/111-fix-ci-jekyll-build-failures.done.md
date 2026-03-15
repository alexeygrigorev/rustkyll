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

## Log

### [SWE] 2026-03-15

- Analyzed CI logs from run 23107970167
- Root cause: `build_with_jekyll()` in `tests/integration_feed_sitemap.rs` called `Command::new("jekyll")` directly, bypassing bundler. The DTC site requires `jekyll-theme-cayman` which is installed via `bundle install` into a local vendor path. Without `bundle exec`, the system Jekyll cannot find site-specific gems.
- Error message: `The jekyll-theme-cayman theme could not be found. (Jekyll::Errors::MissingDependencyException)`
- Fix: Changed `build_with_jekyll()` to use `Command::new("bundle").arg("exec").arg("jekyll")` with `.current_dir(source)` so the site's Gemfile and bundled gems are loaded correctly.
- No other test files invoke Jekyll directly (confirmed via grep).
- Verified locally: `bundle exec jekyll build` succeeds for both DTC and kids sites.
- Tests: all 1070+ tests pass (9 unit tests in integration_feed_sitemap, full suite passes)
- Clippy: clean (no warnings)
- Fmt: clean
- Files modified: `tests/integration_feed_sitemap.rs`
