# Issue 110: Fix CI -- all tests must run, no skipping

## Priority

HIGH -- tests exist for a reason. Skipping them hides bugs.

## Problem

The CI integration job skips 7+ test categories with `--skip` flags:
```
--skip structural_comparison --skip vs_jekyll --skip kids_ --skip page_count --skip _notes_exist --skip _stories_exist --skip build_time
```

This defeats the purpose of having tests. Tests must always run in CI. If a test needs a site, clone it. If it needs Jekyll, install it. If it's too slow, optimize it -- don't skip it.

## Goal

Remove ALL `--skip` flags from the CI integration test command. Every test must run and pass.

## Dependencies

None. This is a CI infrastructure issue.

## What needs to happen

### 1. Clone all required sites in CI

The following sites must be cloned (shallow `--depth 1`) into `websites/` during the CI integration job:

- `DataTalksClub/datatalksclub.github.io` (already cloned)
- `alexeygrigorev/kids-horror-stories-ru` -- needed by `kids_*` tests in `integration_feed_sitemap.rs` and `integration_performance.rs`
- `muan/muan.github.com` into `websites/muan-blog` -- needed by `page_count`, `_notes_exist`, `_stories_exist` tests
- `tomjoht/documentation-theme-jekyll` into `websites/documentation-theme-jekyll` -- needed by `page_count` test
- `Homebrew/brew.sh` into `websites/homebrew-site` -- needed by `page_count` test

The synthetic benchmark sites (`large-blog-3000` and `large-docs-site`) must also be available in CI. These are locally generated repos without remotes. Options:
- Create a script that generates them (preferred -- reproducible)
- Or host them on GitHub and clone them

### 2. Install Jekyll in CI

The `vs_jekyll` and `structural_comparison` tests need Jekyll installed. The CI job must:
- Install Ruby (use `ruby/setup-ruby` action)
- Install Bundler and Jekyll: `gem install bundler jekyll`
- Run `bundle install` in site directories that have a Gemfile (DTC site, kids-horror-stories-ru)
- Ensure `jekyll` is in PATH so compare-output.sh and the vs_jekyll tests can invoke it

### 3. Fix build_time thresholds for CI

Current thresholds:
- DTC site: 30s (debug mode)
- Kids site: 5s (debug mode)

CI runners (GitHub Actions `ubuntu-latest`) are typically 2-core VMs. The thresholds may need adjustment. Options (pick one):
- Raise thresholds to accommodate CI hardware (e.g., 60s for DTC, 15s for kids)
- Detect CI via `std::env::var("CI")` and apply relaxed thresholds
- Run build_time tests with `--release` in CI (faster but requires release build)

### 4. Remove all --skip flags

Change the CI integration test command from:
```
cargo test -- --ignored --skip structural_comparison --skip vs_jekyll --skip kids_ --skip page_count --skip _notes_exist --skip _stories_exist --skip build_time
```
to:
```
cargo test -- --ignored
```

## Scope

This issue covers ONLY CI workflow changes and any minimal code changes needed to make tests pass on CI hardware (e.g., adjusting build_time thresholds). No new tests are added; all 20+ existing ignored tests must run and pass.

## Acceptance Criteria

- [ ] `.github/workflows/ci.yml` integration job `cargo test` command has ZERO `--skip` flags
- [ ] The command is `cargo test -- --ignored` (or equivalent that runs ALL ignored tests)
- [ ] `test_structural_comparison_kids_horror_stories` passes in CI (Jekyll installed, kids site cloned)
- [ ] `test_structural_comparison_dtc_site` passes in CI (Jekyll installed, DTC site cloned)
- [ ] `test_dtc_feed_vs_jekyll` passes in CI
- [ ] `test_kids_podcast_vs_jekyll` passes in CI
- [ ] `test_dtc_sitemap_vs_jekyll` passes in CI
- [ ] `test_kids_podcast_validation` passes in CI (kids site cloned)
- [ ] `test_kids_sitemap_validation` passes in CI (kids site cloned)
- [ ] `test_kids_site_build_time` passes in CI (threshold appropriate for CI hardware)
- [ ] `test_dtc_site_build_time` passes in CI (threshold appropriate for CI hardware)
- [ ] `test_kids_site_output_count` passes in CI
- [ ] `test_large_blog_3000_page_count` passes in CI (site available)
- [ ] `test_large_docs_site_page_count` passes in CI (site available)
- [ ] `test_documentation_theme_jekyll_page_count` passes in CI (site cloned)
- [ ] `test_muan_blog_page_count` passes in CI (site cloned)
- [ ] `test_homebrew_site_page_count` passes in CI (site cloned)
- [ ] `test_muan_blog_notes_exist` passes in CI
- [ ] `test_muan_blog_stories_exist` passes in CI
- [ ] `cargo build` and `cargo test` (non-ignored) still pass in CI
- [ ] The CI workflow completes successfully end-to-end (push or PR trigger)

## Test Scenarios

### CI Workflow: Site cloning

- CI clones `alexeygrigorev/kids-horror-stories-ru` into `websites/alexeygrigorev/kids-horror-stories-ru` (shallow)
- CI clones `muan/muan.github.com` into `websites/muan-blog` (shallow)
- CI clones `tomjoht/documentation-theme-jekyll` into `websites/documentation-theme-jekyll` (shallow)
- CI clones `Homebrew/brew.sh` into `websites/homebrew-site` (shallow)
- CI makes `large-blog-3000` and `large-docs-site` available (either generated or cloned)

### CI Workflow: Jekyll installation

- Ruby and Jekyll are installed in the CI integration job
- `bundle install` succeeds for DTC site and kids-horror-stories-ru
- `jekyll build` can be invoked by compare-output.sh and vs_jekyll tests

### CI Workflow: All ignored tests pass

- Running `cargo test -- --ignored` in CI produces zero failures
- The `structural_comparison` tests run compare-output.sh which invokes Jekyll and produces a passing comparison
- The `vs_jekyll` tests build both Jekyll and rustkyll output and compare feeds/sitemaps
- The `page_count` tests build benchmark sites and verify HTML file counts match expected values
- The `build_time` tests complete within thresholds (adjusted for CI if needed)
- The `_notes_exist` and `_stories_exist` tests verify muan-blog output directories

### Local development: No regression

- `cargo test` (without `--ignored`) still passes locally with no changes
- `cargo test -- --ignored` still passes locally for developers who have the sites cloned

## Implementation Notes

- The synthetic sites `large-blog-3000` and `large-docs-site` are locally generated git repos with no remote. The engineer needs to either (a) create a generation script that CI can run, or (b) push them to GitHub so CI can clone them. Option (a) is preferred for reproducibility.
- The CI job will be slower (cloning sites, installing Ruby/Jekyll). Consider caching Ruby gems and site clones across runs.
- The `compare-output.sh` script has a hardcoded Jekyll path (`/home/alexey/.rvm/gems/ruby-3.3.7/bin/jekyll`) in `benchmark.sh` -- the CI step must ensure `jekyll` is in PATH so the scripts find it.
- If any existing test genuinely cannot work in CI (e.g., it depends on local-only resources that cannot be replicated), the engineer must raise this during implementation -- do NOT silently skip it.

## Log

### [SWE] 2026-03-15 implementation

- Created `scripts/generate-synthetic-sites.sh` to generate `large-blog-3000` (3000 posts) and `large-docs-site` (800 pages across 10 sections) deterministically. Script is idempotent (skips if sites already exist with correct counts).
- Updated `.github/workflows/ci.yml`:
  - Renamed integration job from "Integration Tests (DTC Site)" to "Integration Tests (All Sites)"
  - Added shallow clones for all 5 required sites: DTC, kids-horror-stories-ru, muan-blog, documentation-theme-jekyll, homebrew-site
  - Added `scripts/generate-synthetic-sites.sh` step for synthetic benchmark sites
  - Added Ruby 3.3 setup via `ruby/setup-ruby@v1` action
  - Added gem caching via `actions/cache@v4` for `vendor/bundle`
  - Added `gem install bundler jekyll`
  - Added `bundle install` for DTC site and kids-horror-stories-ru (the two sites with Gemfiles)
  - Removed ALL `--skip` flags: command is now `cargo test -- --ignored`
  - Set `CI=true` env var explicitly for the test step
  - Added `scripts/**` to path triggers so CI workflow changes are tested
- Updated `tests/integration_performance.rs`:
  - Added `is_ci()` helper that checks `std::env::var("CI")`
  - DTC build_time threshold: 30s local, 90s CI (2-core VM is ~3x slower)
  - Kids build_time threshold: 5s local, 30s CI
- Verified: all 1054+ non-ignored tests pass, clippy clean, fmt clean
- Files modified: `.github/workflows/ci.yml`, `tests/integration_performance.rs`, `scripts/generate-synthetic-sites.sh` (new)
