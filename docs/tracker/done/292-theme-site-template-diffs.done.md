# Issue 292: Fix remaining theme site template/layout DOM diffs

## Problem

After issue 290 eliminated all syntax highlighting diffs across theme sites, 7 of 10 theme sites still do not reach a perfect 2/2 DOM match. The remaining diffs are all template/layout issues caused by `site.github.*` variable handling.

## Root Cause Analysis

### href attribute diffs (6 sites: dinky, hacker, leap-day, merlot, midnight, time-machine)

These theme sites do NOT list `jekyll-github-metadata` in their `_config.yml` plugins, nor do they have an explicit `github:` key. Their layouts reference `{{ site.github.repository_url }}`, `{{ site.github.zip_url }}`, `{{ site.github.tar_url }}`, etc.

**Jekyll behavior (local build without plugin):** All `site.github.*` variables resolve to nil/empty, producing `href=""` in the HTML.

**Rustkyll behavior (current bug):** `build_site_context()` in `src/generator.rs` line 207 unconditionally populates `site.github.repository_url` from the git remote, even when there is no `jekyll-github-metadata` plugin and no explicit `github:` config key. This produces `href="https://github.com/pages-themes/dinky"` instead of `href=""`.

The `zip_url` and `tar_url` fields are not populated by rustkyll either way, so those correctly render as empty. The only mismatch is `repository_url`.

**Specific evidence** (dinky-theme):
- Jekyll cached: `<a class="buttons github" href="">View On GitHub</a>`
- Rustkyll: `<a class="buttons github" href="https://github.com/pages-themes/dinky">View On GitHub</a>`

### Text/element diffs (primer-theme)

Primer-theme has an explicit `github:` key in its config with `repository_url`, `source.branch`, `license`, and `private` fields. Its layout (`_layouts/default.html` line 22) uses `{% github_edit_link "Improve this page" %}`, which is a custom Liquid tag from the `jekyll-github-metadata` plugin.

**Jekyll behavior:** `jekyll-github-metadata` plugin resolves the edit URL from `site.github.repository_url` and `site.github.source.branch`, producing `<a href="https://github.com/pages-themes/primer/edit/master/index.md">Improve this page</a>`.

**Rustkyll behavior (current bug):** The `{% github_edit_link %}` tag is implemented as a no-op in `src/template/noop_tags.rs`, producing empty output. This results in `This site is open source. .` instead of `This site is open source. <a href="...">Improve this page</a>.`

## Key Files

- `src/generator.rs`: `build_site_context()` (lines 180-238) -- site.github population logic
- `src/generator.rs`: `resolve_repository_url()` (line 338) -- unconditional git remote fallback
- `src/template/noop_tags.rs`: `GithubEditLinkTag` -- currently a no-op, needs real implementation
- Theme configs (none have `jekyll-github-metadata` in plugins):
  - `websites/dinky-theme/_config.yml`
  - `websites/hacker-theme/_config.yml`
  - `websites/leap-day-theme/_config.yml`
  - `websites/merlot-theme/_config.yml`
  - `websites/midnight-theme/_config.yml`
  - `websites/time-machine-theme/_config.yml`
- `websites/primer-theme/_config.yml` -- has explicit `github:` key with `repository_url` and `source.branch`

## Fix Strategy

### Part A: Gate repository_url behind plugin/explicit-config check

In `build_site_context()`, change the `repository_url` population so that:
- If `jekyll-github-metadata` is in plugins OR an explicit `github:` key exists in config: resolve from git remote as currently done
- Otherwise: do NOT populate `repository_url` (leave it nil, which renders as empty string)

This aligns with Jekyll's behavior where `site.github.*` is empty without the metadata plugin.

### Part B: Implement github_edit_link tag

Replace the no-op `GithubEditLinkTag` with a real implementation that:
1. Reads the link text from the tag argument (e.g., `"Improve this page"`)
2. Constructs the edit URL from `site.github.repository_url` and `site.github.source.branch` and the current page's source path
3. Produces `<a href="{repository_url}/edit/{branch}/{path}">{link_text}</a>`
4. Produces empty output when `site.github.repository_url` is nil/empty (matching Jekyll)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with no regressions
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` is clean
- [ ] **dinky-theme**: `href=""` on "View On GitHub" link (matches Jekyll cached output)
- [ ] **hacker-theme**: `href=""` on "View on GitHub" link (matches Jekyll cached output)
- [ ] **leap-day-theme**: `href=""` on GitHub link (matches Jekyll cached output)
- [ ] **merlot-theme**: `href=""` on GitHub link (matches Jekyll cached output)
- [ ] **midnight-theme**: `href=""` on GitHub link (matches Jekyll cached output)
- [ ] **time-machine-theme**: `href=""` on all GitHub/repository links (matches Jekyll cached output)
- [ ] **primer-theme**: `{% github_edit_link "Improve this page" %}` produces `<a href="https://github.com/pages-themes/primer/edit/master/index.md">Improve this page</a>`
- [ ] Sites WITH `jekyll-github-metadata` plugin or explicit `github:` config still get `repository_url` populated correctly
- [ ] No regressions on DTC or other sites that currently pass

## Test Scenarios

### Unit: repository_url gating

- Test that `build_site_context()` with no plugin and no explicit github key produces nil `repository_url`
- Test that `build_site_context()` with `jekyll-github-metadata` in plugins produces a real `repository_url`
- Test that `build_site_context()` with explicit `github:` config key (like primer-theme) preserves `repository_url`

### Unit: github_edit_link tag

- Test that `{% github_edit_link "Improve this page" %}` with `site.github.repository_url` and `site.github.source.branch` in context produces correct `<a>` element
- Test that `{% github_edit_link "Edit" %}` with empty/nil `repository_url` produces empty output
- Test with non-ASCII link text (e.g., `{% github_edit_link "Seite verbessern" %}`)

### Integration: theme site output verification

- Build dinky-theme and verify `href=""` on the "View On GitHub" link in `_site_rustkyll/index.html`
- Build primer-theme and verify the "Improve this page" link appears with correct edit URL in `_site_rustkyll/index.html`
- Build time-machine-theme and verify all `repository_url` references render as `href=""`

## Dependencies

- Issue 290 (done) -- syntax highlighting diffs resolved

## Log

### [SWE] 2026-03-21

**TDD Cycle - Part A: Gate repository_url**
- Wrote test `test_github_repo_url_nil_without_plugin_or_explicit_config` (src/generator.rs)
- Updated existing tests that expected unconditional repository_url population
- Updated integration test `test_site_context_github_url_nil_without_plugin` (tests/integration_context.rs)
- Ran tests: existing tests FAIL as expected with old behavior assertions
- Implemented fix: gated `repository_url` population in `build_site_context()` behind `has_plugin || has_explicit_github` check
- Ran tests: ALL PASS

**TDD Cycle - Part B: Implement github_edit_link tag**
- Wrote 7 tests for github_edit_link (src/template/noop_tags.rs):
  - `test_github_edit_link_no_output_without_context`
  - `test_github_edit_link_with_string_arg_no_github`
  - `test_github_edit_link_produces_link_with_context`
  - `test_github_edit_link_empty_repository_url`
  - `test_github_edit_link_unicode_link_text`
  - `test_github_edit_link_default_branch_master`
  - `test_feed_meta_in_layout_context` (existing, preserved)
- Implemented `GithubEditLinkRenderable` that reads `site.github.repository_url`, `site.github.source.branch`, and `page.path` from runtime context
- Ran tests: ALL PASS

**Final verification:**
- Full test suite: 2,651 tests (2,636 run + 15 ignored), 0 failures
- Clippy: clean (no warnings from our code)
- Fmt: clean

**Files modified:**
- `src/generator.rs` -- gated repository_url behind plugin/explicit-config check, updated unit tests
- `src/template/noop_tags.rs` -- replaced no-op GithubEditLinkTag with real implementation
- `tests/integration_context.rs` -- updated DTC integration test to expect nil repository_url
