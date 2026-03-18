# Issue 210: Fix choosealicense.com JSON-LD and CSS version hash (57 pages)

## Problem

choosealicense.com has 15/72 (21%) DOM match. Two systematic issues affect nearly every page:

### 1. CSS version hash empty (every page)

Jekyll output: `href='/assets/css/application.css?v=5772b493422a39e20df7b265eab77bb1c79ee9cf'`
Rustkyll output: `href='/assets/css/application.css?v=''`

The template at `_includes/header.html` line 7 uses `{{ site.github.build_revision }}`. Jekyll's `jekyll-github-metadata` plugin populates this with the git HEAD SHA. Rustkyll's `build_site_context()` in `src/generator.rs` builds `site.github` but only includes `repository_url` -- it does not include `build_revision`.

**Fix**: In `build_site_context()`, resolve the git HEAD SHA (via `git rev-parse HEAD`) and insert it as `site.github.build_revision`.

### 2. JSON-LD breadcrumb URLs are empty/relative (every page)

Jekyll output: `"@id": "https://github.com/pages/alexeygrigorev/rustkyll/"`
Rustkyll output: `"@id": "/"`

The template at `_includes/breadcrumbs.html` uses `{{ site.github.url }}` for breadcrumb URLs (lines 19, 26, 33). Jekyll's `jekyll-github-metadata` plugin sets `site.github.url` to the GitHub Pages URL for the site. Rustkyll does not populate `site.github.url` at all, so it renders as empty string, leaving only the path portion.

**Fix**: In `build_site_context()`, populate `site.github.url`. The value should come from `site.url` in `_config.yml` (which is `https://choosealicense.com` for this site). This matches what Jekyll's github-metadata plugin returns for sites with a custom domain configured via `url:` in config.

### 3. License sort order in appendix table (OUT OF SCOPE)

Licenses appear in different order (e.g., `bsd-2-clause-patent` before `bsd-2-clause`). This is a data iteration order issue. **Not addressed in this issue** -- the goal is to fix issues 1 and 2 only. Sort order is tracked separately or can be filed as a follow-up if needed.

## Implementation Location

The changes are concentrated in `src/generator.rs`, specifically in `build_site_context()` (around line 149-153) where the `site.github` object is constructed. Two new fields need to be added:

1. `site.github.build_revision` -- git HEAD SHA from `git rev-parse HEAD`
2. `site.github.url` -- from `config.url` (the site's configured URL)

## Dependencies

None. This issue is self-contained.

## Goal

Fix issues 1-2 to bring choosealicense.com from 15/72 to 50+/72 DOM match.

## TDD Test Scenarios

### Unit Test 1: site.github.build_revision from git SHA

```rust
#[test]
fn test_github_build_revision_populated_from_git_head() {
    // 1. Create a temp directory, `git init`, create a file, commit with
    //    non-ASCII message: "Исправление · fix"
    // 2. Get expected SHA via `git rev-parse HEAD`
    // 3. Create a minimal SiteConfig
    // 4. Call build_site_context() with site_dir pointing to the temp dir
    // 5. Assert site["github"]["build_revision"] == expected SHA (40-char hex)
}
```

### Unit Test 2: site.github.build_revision is empty for non-git directory

```rust
#[test]
fn test_github_build_revision_empty_for_non_git_dir() {
    // 1. Create a temp directory (no git init)
    // 2. Call build_site_context() with site_dir pointing to that dir
    // 3. Assert site["github"]["build_revision"] is empty string or nil
    //    (should not panic or error)
}
```

### Unit Test 3: site.github.url populated from config url

```rust
#[test]
fn test_github_url_from_config() {
    // 1. Create SiteConfig with url = "https://example.com"
    // 2. Call build_site_context()
    // 3. Assert site["github"]["url"] == "https://example.com"
}
```

### Integration Test 4: CSS version hash renders in HTML output

```rust
#[test]
fn test_css_version_hash_renders_in_template() {
    // 1. Create a minimal site in a git repo with a template containing:
    //    <link href="style.css?v={{ site.github.build_revision }}">
    // 2. Build the site
    // 3. Assert output HTML contains `style.css?v=<40-char-hex>`
    // 4. Assert it does NOT contain `?v=''` or `?v=`(empty)
}
```

### Integration Test 5: JSON-LD breadcrumb renders absolute URLs

```rust
#[test]
fn test_jsonld_breadcrumb_renders_absolute_urls() {
    // 1. Create a minimal site with url: "https://example.com"
    //    and a template containing:
    //    "@id": "{{ site.github.url }}{{ page.url }}"
    // 2. Build the site with a page at /about/
    // 3. Assert output contains "@id": "https://example.com/about/"
    // 4. Assert output does NOT contain "@id": "/about/" (relative)
    // 5. Include non-ASCII page title: "О проекте" to test Unicode in JSON-LD
}
```

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all new tests (minimum 4 new tests)
- [ ] `site.github.build_revision` is populated from `git rev-parse HEAD` when the site directory is inside a git repository
- [ ] `site.github.build_revision` is empty string or nil (not an error) when the site is not in a git repository
- [ ] `site.github.url` is populated from `config.url`
- [ ] Existing tests continue to pass (no regressions)
- [ ] Tests include non-ASCII/Unicode content (per project convention)

## Output Verification Criteria

After building choosealicense.com with rustkyll:

- [ ] Every generated HTML page's `<link>` tag for `application.css` has a non-empty `?v=` parameter containing a 40-character hex SHA (not `?v=''` or `?v=`)
- [ ] Every generated HTML page's JSON-LD `<script type="application/ld+json">` block contains absolute URLs in `@id` fields (starting with `https://`), not relative paths like `/` or `/licenses`
- [ ] The JSON-LD breadcrumb structure is valid JSON (parseable)
- [ ] Pages without `hide_breadcrumbs: true` have the JSON-LD breadcrumb block; pages with it do not

## Log

### [SWE] 2026-03-18

- **Root cause 1**: `build_site_context()` in `src/generator.rs` only populated `site.github.repository_url` but not `site.github.build_revision` (git HEAD SHA needed for CSS cache busting)
- **Root cause 2**: `site.github.url` was not populated at all, causing JSON-LD breadcrumb URLs to be relative instead of absolute
- **Fix**: Added two new fields to the `site.github` object in `build_site_context()`:
  - `build_revision`: resolved via `git rev-parse HEAD` (empty string for non-git dirs)
  - `url`: populated from `config.url`
- Added helper function `resolve_build_revision()` following the same pattern as existing `resolve_repository_url()`
- **Tests added**: 6 new tests in `tests/integration_github_metadata.rs`:
  1. `test_github_build_revision_populated_from_git_head` - SHA matches git HEAD
  2. `test_github_build_revision_empty_for_non_git_dir` - empty string, no panic
  3. `test_github_url_from_config` - matches config URL
  4. `test_github_url_empty_when_config_url_empty` - empty string when no URL
  5. `test_css_version_hash_renders_in_template` - end-to-end template rendering
  6. `test_jsonld_breadcrumb_renders_absolute_urls` - end-to-end JSON-LD rendering
- All tests include non-ASCII/Unicode content (Russian, French, Spanish characters)
- **Build**: 6/6 new tests pass, clippy clean, fmt clean
- **Files modified**: `src/generator.rs`
- **Files created**: `tests/integration_github_metadata.rs`
- Pre-existing test failures (10 in lib tests) are from parallel issue 209 work, not related to this change
