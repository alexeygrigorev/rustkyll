# Issue 232: Fix `site.github.url` to match `jekyll-github-metadata` behavior

## Problem

choosealicense.com has 155 `jsonld_value_differs` diffs in breadcrumb JSON-LD (`itemListElement`) because `site.github.url` resolves differently between Jekyll and rustkyll.

The choosealicense.com templates use `{{ site.github.url }}` (populated by the `jekyll-github-metadata` plugin) for breadcrumb base URLs. Jekyll's plugin derives `site.github.url` from the git remote, constructing a GitHub Pages URL. Rustkyll currently sets `site.github.url` to `config.url` (the `url:` value from `_config.yml`), which produces a different value.

## Root Cause Analysis

### How Jekyll's `jekyll-github-metadata` resolves `site.github.url`

When the `jekyll-github-metadata` plugin is active, it populates `site.github.url` with the **GitHub Pages URL** for the repository:

1. **With GitHub API access** (`JEKYLL_GITHUB_TOKEN` set): Fetches the repo's Pages URL from the API, which includes custom domain configuration (CNAME).
2. **Without API access** (local builds): Derives the Pages URL from the git remote's owner/repo name (the "nwo" -- name with owner), constructing `https://{OWNER}.github.io/{REPO}/`. For organization repos where the repo name matches `{ORG}.github.io`, the URL is just `https://{ORG}.github.io/`.

### Current rustkyll behavior

In `src/generator.rs` (line 211-212), `site.github.url` is set to `config.url` when not already present from explicit `github:` config. This is incorrect -- it should replicate the `jekyll-github-metadata` URL resolution logic.

### Why this causes diffs

The comparison builds both Jekyll and rustkyll from the site directory. Jekyll resolves `site.github.url` from the git remote and produces a GitHub Pages URL. Rustkyll uses `config.url`. When the git remote's Pages URL differs from `config.url`, all breadcrumb URLs differ.

For choosealicense.com (remote: `github/choosealicense.com`), the Pages URL pattern would be `https://github.github.io/choosealicense.com/`. The `config.url` is `https://choosealicense.com`. These differ, causing 155 diffs across all 72 pages (each page has 2-3 breadcrumb items).

### What this issue fixes

**Only RC-A: `site.github.url` resolution (155 diffs)**

When `jekyll-github-metadata` is listed in `plugins`, derive `site.github.url` from the git remote NWO using the same GitHub Pages URL pattern that Jekyll uses, instead of falling back to `config.url`.

### Out of scope (other diff categories)

These are NOT addressed by this issue and remain as-is or require separate issues:

- **Timestamp diffs (94 diffs)**: Build-time differences (`datePublished`, `article:published_time`). Different build moments produce different timestamps. Not a bug.
- **CSS hash diffs (55 diffs)**: Expected -- different file content produces different hashes.
- **Sort order diffs (~86 diffs)**: Notable project lists appear in different order. This is a data sort stability issue, not related to `site.github.url`.
- **Smart quote encoding (10 diffs)**: Typography differences in descriptions containing curly quotes. Separate concern.
- **Structural diffs (47 diffs)**: Markdown rendering differences (about.html, community.html body element ordering) and footer nav link differences. Separate concern.
- **`window.annotations` (25 diffs)**: JSON key ordering in script tags. Separate concern.

## Scope

Modify the `site.github.url` resolution in `build_site_context()` so that when `jekyll-github-metadata` is active, the URL is derived from the git remote using the GitHub Pages URL pattern, matching Jekyll's local-build behavior.

The implementation must:

1. Extract the owner/repo NWO from the git remote (reuse existing `resolve_repository_url` logic)
2. Construct the GitHub Pages URL: `https://{OWNER}.github.io/{REPO}/`
3. Handle the special case where repo name equals `{OWNER}.github.io` (user/org site): URL is `https://{OWNER}.github.io/`
4. Only apply this when `jekyll-github-metadata` is in the plugins list
5. If explicit `github.url` is set in `_config.yml`, that takes priority (existing behavior)
6. If no git remote is available, fall back to `config.url` (current behavior)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all new and existing tests
- [ ] When `jekyll-github-metadata` is in `plugins` and a git remote is available, `site.github.url` is derived from the git remote using the GitHub Pages URL pattern (`https://{OWNER}.github.io/{REPO}/`)
- [ ] When repo name matches `{OWNER}.github.io`, `site.github.url` is `https://{OWNER}.github.io/` (no repo suffix)
- [ ] When `jekyll-github-metadata` is NOT in `plugins`, `site.github.url` remains `config.url` (unchanged behavior)
- [ ] When explicit `github: { url: "..." }` is in `_config.yml`, that value takes priority over git-derived URL
- [ ] When no git remote is available, `site.github.url` falls back to `config.url`
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes

## Test Scenarios

All tests follow TDD: write the test FIRST, verify it FAILS, implement the fix, verify it PASSES.

### Unit: GitHub Pages URL derivation

**Test 1: Standard repo produces correct Pages URL**
- Configure: `jekyll-github-metadata` in plugins, git remote resolves to `github/choosealicense.com`
- Assert: `site.github.url` equals `https://github.github.io/choosealicense.com/`
- Note: This may require mocking or a helper function that converts NWO to Pages URL

**Test 2: User/org site repo (name matches `{OWNER}.github.io`)**
- Configure: git remote resolves to `DataTalksClub/datatalksclub.github.io`
- Assert: `site.github.url` equals `https://datatalksclub.github.io/`

**Test 3: Regular user repo**
- Configure: git remote resolves to `alexeygrigorev/mlbookcamp-page`
- Assert: `site.github.url` equals `https://alexeygrigorev.github.io/mlbookcamp-page/`

**Test 4: No `jekyll-github-metadata` plugin -- uses config.url**
- Configure: `jekyll-github-metadata` NOT in plugins, `url: "https://example.com"` in config
- Assert: `site.github.url` equals `https://example.com`

**Test 5: Explicit `github.url` in config takes priority**
- Configure: `jekyll-github-metadata` in plugins AND explicit `github: { url: "https://custom.example.com" }` in `_config.yml`
- Assert: `site.github.url` equals `https://custom.example.com`

**Test 6: No git remote available -- falls back to config.url**
- Configure: `jekyll-github-metadata` in plugins, but site directory has no git remote
- Assert: `site.github.url` equals `config.url` value

### Unit: NWO extraction helper

**Test 7: Extract NWO from HTTPS URL**
- Input: `https://github.com/github/choosealicense.com`
- Assert: NWO is `("github", "choosealicense.com")`

**Test 8: Extract NWO from SSH URL**
- Input: `git@github.com:alexeygrigorev/rustkyll.git`
- Assert: NWO is `("alexeygrigorev", "rustkyll")`

**Test 9: Non-ASCII/Unicode repo name**
- Input: `https://github.com/user/projet-francais`
- Assert: NWO is `("user", "projet-francais")`

## Dependencies

- None

## Notes

- The `resolve_repository_url` function already extracts the git remote and converts it to an HTTPS URL. A new helper function should extract the owner/repo from the remote URL and construct the Pages URL.
- The function `nwo_to_pages_url(owner, repo) -> String` should be straightforward to implement and test independently.
- This fix only affects the `site.github.url` field. `site.github.repository_url` and `site.github.build_revision` remain unchanged.

## Log

### [SWE] 2026-03-19

TDD Cycle:

1. Wrote 10 tests first in `src/generator.rs` (tests module):
   - `test_extract_nwo_from_https_url` -- extract owner/repo from HTTPS URL
   - `test_extract_nwo_from_ssh_url` -- extract owner/repo from SSH URL
   - `test_extract_nwo_unicode_repo_name` -- extract NWO with non-ASCII repo name
   - `test_nwo_to_pages_url_standard_repo` -- standard project repo Pages URL
   - `test_nwo_to_pages_url_org_site` -- user/org site (repo == owner.github.io)
   - `test_nwo_to_pages_url_regular_user_repo` -- regular user repo Pages URL
   - `test_github_url_without_metadata_plugin_uses_config_url` -- no plugin = config.url
   - `test_github_url_with_explicit_github_url_takes_priority` -- explicit config wins
   - `test_github_url_with_plugin_no_git_remote_falls_back` -- plugin but no git = config.url
   - `test_github_url_with_plugin_derives_from_git_remote` -- plugin + git = Pages URL

2. Ran tests: FAILS as expected -- `extract_nwo_from_remote` and `nwo_to_pages_url` not found (compilation error)

3. Implemented three new functions in `src/generator.rs`:
   - `extract_nwo_from_remote(remote_url) -> Option<(String, String)>` -- extracts owner/repo from GitHub remote URL (HTTPS or SSH)
   - `nwo_to_pages_url(owner, repo) -> String` -- converts owner/repo to GitHub Pages URL, with special case for org sites
   - `resolve_github_pages_url(config, site_dir) -> String` -- resolves Pages URL from git remote or falls back to config.url

4. Modified `build_site_context()` to use `resolve_github_pages_url()` when `jekyll-github-metadata` plugin is active, instead of always using `config.url`

5. Ran tests: ALL 10 PASS
   - Full test suite: 1847 lib tests + all integration tests pass (0 failures)
   - Clippy: clean (no warnings/errors in rustkyll code)
   - Format: clean

Files modified: `src/generator.rs`
