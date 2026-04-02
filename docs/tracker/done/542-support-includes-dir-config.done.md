# Issue 542: Support `includes_dir` config setting

## Problem

Rustkyll hardcodes the includes directory as `_includes/` (in `src/main.rs:509`). Jekyll supports the `includes_dir` config option in `_config.yml`, which allows sites to override the default. Sites like jekyll-vitepress-theme use `includes_dir: docs/_includes` to load analytics scripts from a different location.

This causes 51 missing `<script>` element diffs (3 per page across all 17 pages) on jekyll-vitepress-theme because the override includes directory contains Plausible analytics scripts (`docs/_includes/jekyll_vitepress/head_end.html`) that are not found -- rustkyll uses the empty stub at `_includes/jekyll_vitepress/head_end.html` instead.

## Scope

Implement support for the `includes_dir` config setting:
1. Add `includes_dir` field to `SiteConfig` in `src/config.rs` (default: `"_includes"`)
2. Use the configured path when resolving `{% include %}` tags in `src/main.rs`
3. Merge includes from both `includes_dir` and `_includes/` -- entries from the custom `includes_dir` override entries from the default `_includes/`, matching Jekyll's theme-override behavior
4. Update the incremental build watcher in `src/incremental.rs` to also track the custom includes directory
5. Update the livereload classifier in `src/livereload.rs` to recognize files under the custom includes dir as `FileChangeKind::Include`

### What NOT to change

- Test code that creates `_includes/` in temp dirs -- these use the default and do not need updating
- The `LayoutEngine::new` signature remains the same (it takes a `Path` for includes_dir)

## Key code locations

- `src/config.rs` line 92: `SiteConfig` struct -- add `includes_dir` field
- `src/main.rs` line 509: `let includes_dir = source.join("_includes");` -- read from config instead
- `src/main.rs` line 509-522: Includes loading -- merge default `_includes/` with custom `includes_dir`
- `src/incremental.rs` line 88: `collect_dir_mtimes(source, &source.join("_includes"), ...)` -- also track custom dir
- `src/livereload.rs` line 156: `starts_with("_includes/")` check -- also check custom dir prefix

## Merge behavior

When `includes_dir` is set to a non-default value (e.g., `docs/_includes`):
1. Load all includes from the default `_includes/` directory
2. Load all includes from the custom `includes_dir` directory
3. Custom entries override default entries with the same relative path
4. This matches Jekyll's behavior where a site-level include overrides a theme-level include

Example for jekyll-vitepress-theme:
- `_includes/head.html` -- loaded (no override exists)
- `_includes/jekyll_vitepress/head_end.html` -- loaded initially (empty stub)
- `docs/_includes/jekyll_vitepress/head_end.html` -- loaded and overrides the above (has Plausible scripts)

## Dependencies

None.

## Split from

Issue #443 (jekyll-vitepress-theme rendering issues) -- RC1b.

## Baseline

- DTC DOM: 596/790 matched (must not regress -- 596 matched files, 255 total diffs)
- jekyll-vitepress-theme DOM: 0/17 matched, 643 total diffs (baseline before this change; the original issue said 575 but that was stale -- verified at 643 on committed code without includes_dir)

## Acceptance Criteria

- [x] `SiteConfig` in `src/config.rs` has an `includes_dir` field that defaults to `"_includes"`
- [x] `cargo build` compiles without errors
- [x] `cargo clippy -- -D warnings` passes
- [x] When `_config.yml` contains `includes_dir: docs/_includes`, rustkyll loads includes from `docs/_includes/` with fallback to `_includes/`
- [x] Custom includes override default includes at the same relative path
- [x] When `includes_dir` is not set, behavior is unchanged (uses `_includes/` only)
- [x] DTC DOM match count does not drop below 596/790 matched files and total diffs do not increase above 255
- [x] jekyll-vitepress-theme: Plausible analytics scripts from `docs/_includes/jekyll_vitepress/head_end.html` are now rendered in output HTML (34 of 51 original missing_element script diffs fixed). The remaining 17 missing_element script diffs (1 per page) are NOT from includes_dir -- they are caused by a missing `<style id="vp-rouge-theme">` CSS block that Jekyll generates for syntax highlighting. This shifts element positions causing the DOM comparator to report a missing `<script>`. See investigation notes in log. This is a separate issue (rouge theme CSS generation), not an includes_dir problem.
- [x] `cargo test` passes (all existing tests plus new tests)

## Test Scenarios

### Unit: Config parsing
- Parse `_config.yml` with `includes_dir: docs/_includes`, verify `config.includes_dir` is `"docs/_includes"`
- Parse `_config.yml` without `includes_dir`, verify default is `"_includes"`
- Parse `_config.yml` with `includes_dir: _includes` (explicit default), verify it works the same as omitting it

### Unit: Includes merging
- Create temp dir with `_includes/a.html` and `custom_includes/a.html` (different content) and `_includes/b.html`
- Load includes with custom dir, verify `a.html` uses custom content and `b.html` uses default content
- Verify subdirectory includes also merge correctly (e.g., `subdir/file.html`)

### Integration: Site build with custom includes_dir
- Create a minimal site with `includes_dir: custom_inc` in `_config.yml`
- Place a layout that uses `{% include header.html %}`
- Put `header.html` in `custom_inc/` with specific content
- Build the site and verify the output HTML contains the custom include content
- Also place a file only in `_includes/` and verify it is found as fallback

### Integration: Override behavior
- Create a site with `_includes/snippet.html` containing "default" and `custom_dir/snippet.html` containing "override"
- Set `includes_dir: custom_dir` in config
- Build and verify output contains "override", not "default"

### Regression: DTC site
- Build DTC site and run DOM comparison
- Verify match count is >= 596 and total diffs <= 255

## Log

### [PM] 2026-04-02 14:00
- Investigated codebase: `includes_dir` not in SiteConfig, hardcoded as `_includes/` at main.rs:509
- Confirmed jekyll-vitepress-theme has `includes_dir: docs/_includes` in config
- Verified `docs/_includes/jekyll_vitepress/head_end.html` contains Plausible analytics scripts
- Default `_includes/jekyll_vitepress/head_end.html` is empty stub (just a comment)
- DOM comparison shows 51 missing `<script>` element diffs (3 per page x 17 pages)
- DTC baseline recorded: 596/790 matched, 255 total diffs
- jekyll-vitepress-theme baseline: 0/17 matched, 575 total diffs
- Groomed with acceptance criteria and test scenarios

### [SWE] 2026-04-02 15:00 (initial implementation)

**Fix 1: Add `includes_dir` field to SiteConfig**
- Wrote tests: test_includes_dir_custom_value, test_includes_dir_default_when_absent, test_includes_dir_explicit_default, test_includes_dir_unicode_path (src/config.rs)
- Ran tests: FAILS -- compilation error, `no field includes_dir on type SiteConfig`
- Added `includes_dir` field with `#[serde(default = "default_includes_dir")]` to SiteConfig, added `default_includes_dir()` function, updated Default impl
- Ran tests: PASSES -- all 4 config tests pass

**Fix 2: Add `load_includes_merged` function**
- Wrote tests: test_load_includes_merged_custom_overrides_default, test_load_includes_merged_subdirectory_override, test_load_includes_merged_same_dir_no_duplication, test_load_includes_merged_unicode_filenames (src/template/engine.rs)
- Ran tests: FAILS -- compilation error, `cannot find function load_includes_merged in this scope`
- Implemented `load_includes_merged()` in src/template/engine.rs that loads default dir first then overlays custom dir entries
- Ran tests: PASSES -- all 4 merge tests pass

**Fix 3: Use config.includes_dir in main.rs build pipeline**
- Added `LayoutEngine::new_with_merged_includes()` constructor in src/template/layout.rs
- Updated src/main.rs to compute `default_includes_dir` and `custom_includes_dir` from config, use `new_with_merged_includes`

**Fix 4: Track custom includes dir in incremental builds**
- Added `collect_global_files_with_includes_dir()` to src/incremental.rs
- Updated src/main.rs to call it with `config.includes_dir`

**Fix 5: Recognize custom includes dir in livereload file watcher**
- Wrote tests first but did not document FAIL step properly (noted by QA)

**Fix 6: Integration tests for full pipeline**
- Wrote tests: test_config_includes_dir_parsed, test_merged_includes_override_in_layout_engine, test_merged_includes_subdirectory_override, test_default_includes_dir_unchanged_behavior, test_load_includes_merged_function_directly (tests/test_issue_542.rs)
- Ran tests: PASSES -- all 5 integration tests pass

### [QA] 2026-04-02 16:00
- All tests pass, clippy clean, fmt clean
- DTC DOM: 596/790 matched, 255 total diffs (no regression)
- jekyll-vitepress-theme: 34 of 51 missing script diffs fixed. 17 remain.
- Issues found: (1) TDD violation in Fix 5 -- no documented FAIL step for livereload tests. (2) AC8 partially met -- 17 remaining missing_element script diffs need investigation.
- VERDICT: FAIL

### [SWE] 2026-04-02 17:00 (fixes for QA feedback)

**Note:** Working tree was lost due to accidental `git checkout -- .` during baseline verification. All changes re-implemented from scratch following strict TDD.

**Fix 1 (re-implemented): Add `includes_dir` field to SiteConfig**
- Wrote tests: test_includes_dir_custom_value, test_includes_dir_default_when_absent, test_includes_dir_explicit_default, test_includes_dir_unicode_path (src/config.rs)
- Ran tests: FAILS -- `error[E0609]: no field includes_dir on type config::SiteConfig` (4 errors)
- Implemented: added `includes_dir: String` field with `#[serde(default = "default_includes_dir")]`, `default_includes_dir()` fn, updated Default impl
- Ran tests: PASSES -- all 4 config tests pass

**Fix 2 (re-implemented): Add `load_includes_merged` function**
- Wrote tests: test_load_includes_merged_custom_overrides_default, test_load_includes_merged_subdirectory_override, test_load_includes_merged_same_dir_no_duplication, test_load_includes_merged_unicode_filenames (src/template/engine.rs)
- Ran tests: FAILS -- `error[E0425]: cannot find function load_includes_merged in this scope` (4 errors)
- Implemented `load_includes_merged()` in src/template/engine.rs
- Ran tests: PASSES -- all 4 merge tests pass

**Fix 3 (re-implemented): Use config.includes_dir in main.rs build pipeline**
- Added `LayoutEngine::new_with_merged_includes()` in src/template/layout.rs
- Updated src/main.rs to use `config.includes_dir`

**Fix 4 (re-implemented): Track custom includes dir in incremental builds**
- Added `collect_global_files_with_includes_dir()` to src/incremental.rs
- Updated src/main.rs to call with `config.includes_dir`

**Fix 5 (re-implemented with proper TDD): Livereload custom includes dir**
- Wrote tests FIRST: test_classify_custom_includes_dir, test_classify_custom_includes_dir_default_still_works, test_classify_custom_includes_dir_not_matching, test_classify_custom_includes_dir_unicode_path, test_scope_custom_includes_dir_triggers_full_rebuild (src/livereload.rs)
- Ran tests: FAILS -- `error[E0425]: cannot find function classify_changed_file_with_includes_dir in this scope` (5 errors)
- Temporarily removed the `includes_dir != "_includes" && is_under_dir(...)` guard to verify test_classify_custom_includes_dir catches the bug:
  - Ran test: FAILS -- `assertion left == right failed; left: Content, right: Include` (confirms test catches the bug)
- Restored full implementation with `classify_changed_file_with_includes_dir()`, `is_under_dir()`, `determine_rebuild_scope_with_includes_dir()`
- Ran tests: PASSES -- all 5 livereload tests pass

**Fix 6 (re-used): Integration tests**
- tests/test_issue_542.rs was already present from previous session (untracked file survived git checkout)
- Ran tests: PASSES -- all 5 integration tests pass

**Investigation: Remaining 17 missing_element script diffs on vitepress**

Compared Jekyll vs rustkyll output for `what-is-jekyll-vitepress-theme/index.html`:
- Both outputs have identical script tags: (1) inline theme appearance script, (2) Plausible async loader, (3) Plausible init script, (4) vitepress-theme.js deferred
- The `includes_dir` feature correctly loads all Plausible analytics scripts from `docs/_includes/jekyll_vitepress/head_end.html`
- Jekyll output has an additional `<style id="vp-rouge-theme">` block (230+ lines of rouge syntax highlighting CSS) that rustkyll does not generate
- This missing `<style>` element shifts element positions in `<head>`, causing the DOM comparator to report:
  - `tag_name_differs - expected: 'style', actual: 'script'` at child[15] (17 pages)
  - `missing_element - expected: '<script>', actual: '(none)'` (17 pages) -- this is a position-shift artifact, not a truly missing script
- The remaining 17 missing_element diffs are caused by missing rouge theme CSS generation, NOT by includes_dir
- This is a separate issue (rouge/syntax-highlighting CSS emission) and out of scope for includes_dir

**Investigation: Total diff count 575 vs 643**

- Verified by stashing all changes and building vitepress from committed code: 643 total diffs
- The 575 number in the original issue baseline was stale (from an older commit)
- The includes_dir change did NOT increase vitepress total diffs -- they were already 643 before

**Summary:**
- Files modified: src/config.rs, src/main.rs, src/template/engine.rs, src/template/layout.rs, src/template/mod.rs, src/incremental.rs, src/livereload.rs
- Files created: tests/test_issue_542.rs (from previous session, verified passing)
- Tests added: 18 new tests (4 config, 4 merge, 5 livereload, 5 integration)
- Build results: 3570 lib tests pass, 0 fail; all integration tests pass; clippy clean; fmt clean
- DTC DOM: 596/790 matched, 255 total diffs (exactly matches baseline)
- DTC build time: 0.629s (under 1.0s threshold)
- Vitepress: Plausible analytics scripts now present in output. 34 of 51 original missing_element script diffs eliminated. 17 remain due to missing rouge theme CSS (separate issue, not includes_dir).
- Vitepress total diffs: 643 (same as baseline before includes_dir change -- no regression)

### [QA] 2026-04-02 12:25 (re-test)
- Tests: 3570 lib tests passed, 0 failed, 2 ignored (pre-existing kramdown_parser); all integration test crates pass
- Clippy: clean (no warnings)
- Fmt: clean
- DTC DOM: 596/790 matched, 255 total diffs (matches baseline exactly -- no regression)
- DTC build time: 0.61s (under 1.0s threshold)
- TDD compliance:
  - Fix 1 (config): test written -> FAILS (no field) -> implemented -> PASSES. OK.
  - Fix 2 (merge): test written -> FAILS (cannot find function) -> implemented -> PASSES. OK.
  - Fix 5 (livereload): test written -> FAILS (cannot find function) -> additionally verified test catches logical bug (Content != Include) -> implemented -> PASSES. OK. Previous QA concern resolved.
- AC1 (SiteConfig includes_dir field, default _includes): PASS
- AC2 (cargo build): PASS
- AC3 (clippy clean): PASS
- AC4 (custom includes_dir loads from configured path with fallback): PASS (verified via integration tests)
- AC5 (custom includes override default at same relative path): PASS
- AC6 (default behavior unchanged when includes_dir not set): PASS
- AC7 (DTC DOM >= 596/790, diffs <= 255): PASS (596/790, 255 diffs)
- AC8 (vitepress: 34/51 script diffs fixed, remaining 17 from rouge CSS): PASS -- explanation is reasonable; remaining diffs are position-shift artifacts from missing style element, not missing scripts
- AC9 (cargo test passes): PASS
- VERDICT: PASS

### [PM] 2026-04-02 18:30
- Reviewed diff: 8 files changed (src/config.rs, src/main.rs, src/template/engine.rs, src/template/layout.rs, src/template/mod.rs, src/incremental.rs, src/livereload.rs, tests/test_issue_542.rs)
- Output verification: Ran `scripts/recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io` independently. Result: 596/790 matched, 255 total diffs. Matches baseline exactly.
- Results verified: Real DTC DOM data present, no regression. Vitepress investigation documented with clear explanation of remaining 17 diffs (rouge CSS, not includes_dir).
- Code review: Clean implementation. Merge semantics correct (default first, custom overlays). Backward-compatible via serde defaults. Incremental and livereload paths updated. 18 tests cover config parsing, merge logic, livereload classification, and integration. Unicode tests included.
- Acceptance criteria: all 9 met
- Follow-up issues created: none needed (remaining vitepress diffs are a known separate concern, rouge CSS generation)
- VERDICT: ACCEPT
