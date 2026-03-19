# Issue 229: Fix theme sites remaining diffs

## Problem

10 GitHub Pages theme sites still have diffs. Analysis of `docs/comparison/dom-details/` reveals three distinct root causes:

### Root Cause 1: site.github.repository_url populated when it should be empty (7 sites)

Affected: hacker, dinky, midnight, merlot, leap-day, time-machine (on both pages), primer (not affected by this -- has explicit `github:` config).

In `src/generator.rs`, `resolve_repository_url()` falls back to `git remote get-url origin` even when the site does NOT have `jekyll-github-metadata` in its `plugins` list. Jekyll only populates `site.github.*` when the github-metadata plugin is explicitly activated. Without it, `site.github.repository_url` should be nil/empty.

Currently, `has_github_metadata_plugin()` is only used to gate `build_revision`, but the same gate should also apply to `repository_url` and potentially the entire `site.github` object.

Expected behavior: When `jekyll-github-metadata` is NOT in the plugins list, `site.github.repository_url` should be nil (producing `href=''` in templates), not resolved from git remote.

Concrete diffs:
- `href=''` (expected/Jekyll) vs `href='https://github.com/pages-themes/hacker'` (actual/rustkyll)
- Same pattern for dinky, midnight, merlot, leap-day, time-machine

### Root Cause 2: site.github config override is clobbered (primer only)

Affected: primer-theme (2 pages).

Primer's `_config.yml` has a top-level `github:` key with manual values (`repository_url`, `private`, `license`, `source`). In `src/generator.rs`, extras are inserted first (line ~104), then `site.github` is unconditionally overwritten (line ~178). The manual `github:` config from extras is lost.

Jekyll treats `github:` in `_config.yml` as the authoritative `site.github` object. Rustkyll should merge its computed fields (like `build_revision`, `url`) INTO the config-provided `github:` object rather than replacing it.

This causes two diffs in primer:
1. `style.css?v=be021deb...` (expected) vs `style.css?v=''` (actual) -- because the config `github:` object gets replaced, and `build_revision` is empty since github-metadata plugin is not listed
2. `missing_element` -- a `<div>` that depends on `site.github` values being correctly populated

### Root Cause 3: JavaScript syntax highlighting token class differences (all 10 sites)

All 10 theme sites share the same `index.md` content with a JavaScript code block:
```js
var fun = function lang(l) {
  dateformat.i18n = require('./lang/' + l)
  return true;
}
```

Differences (61 per site in index.html):
- `nf` (expected/Rouge) vs `nx` (actual/rustkyll) for function name `lang` -- the JS override in `src/syntax.rs` line 60 maps `entity.name.function` to `nx`, but Rouge actually uses `nf` for function names in JS declarations
- `dl` (expected) vs `s1` (actual) for string delimiters (single quotes) -- Rouge splits `'./lang/'` into `'` (dl) + `./lang/` (s1) + `'` (dl), while syntect emits the entire quoted string as one s1 token
- Cascading text_differs from the string tokenization differences (different span boundaries cause different text content per span)

## Scope

This issue focuses on **Root Causes 1 and 2** (the `site.github` issues). Root Cause 3 (syntax highlighting) is explicitly descoped to a separate issue because:
- It affects the tokenizer's span boundary logic, not just class mapping
- The string delimiter splitting (dl vs s1) requires changes to `accumulate_and_emit` in syntax.rs
- It is the same code block across all 10 sites, so fixing it once fixes all 10
- The fix is orthogonal to the site.github fixes

### What to fix

1. Gate `repository_url` resolution behind `has_github_metadata_plugin()` -- when the plugin is not active, `site.github.repository_url` should be nil
2. When `_config.yml` has a top-level `github:` key in extras, merge computed github fields into it instead of replacing it
3. Ensure `site.github.build_revision` is populated when the config provides an explicit `github:` block (primer case -- the SHA should come from git even without the plugin, because the config explicitly sets up the github object)

## Acceptance Criteria

- [ ] `site.github.repository_url` is nil when `jekyll-github-metadata` is NOT in the plugins list AND no explicit `github:` config exists
- [ ] When `_config.yml` has a top-level `github:` key, its values are preserved in `site.github` (not overwritten)
- [ ] When `_config.yml` has a top-level `github:` key, computed fields (`build_revision`, `url`, `repository_url`) are merged in as defaults (config values take priority)
- [ ] The 7 sites with `href=''` diffs (hacker, dinky, midnight, merlot, leap-day, time-machine, and the non-syntax diffs in architect/cayman/slate if any) no longer show that diff
- [ ] Primer's `style.css?v=` link includes the git SHA (from `build_revision`)
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all new and existing tests
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` reports no changes needed

## Test Scenarios

### Unit: site.github gating

- Config with NO `jekyll-github-metadata` plugin and NO explicit `github:` key: `site.github.repository_url` should be nil (or at least not resolved from git remote)
- Config WITH `jekyll-github-metadata` plugin: `site.github.repository_url` should be resolved normally
- Config with explicit `github:` key containing `repository_url`: that value should be preserved

### Unit: site.github config merging

- Config with `github: { private: false, repository_url: "https://..." }`: all keys should appear in `site.github`, plus computed defaults for missing keys
- Config with `github: { repository_url: "custom" }` and also a `repository` field: the explicit `github.repository_url` should win over the computed one
- Config with `github: {}` (empty map): computed fields should fill in as defaults

### Integration: theme site output

- Build hacker-theme (or a minimal reproduction) and verify the GitHub link href is empty (matching Jekyll)
- Build primer-theme (or a minimal reproduction) and verify the CSS link includes the git SHA

## Output Verification

- Build at least one affected theme site (e.g., hacker-theme) and inspect the generated HTML for correct `href=''` on the GitHub link
- Build primer-theme and verify `style.css?v=<sha>` in the HTML output
- Compare against Jekyll reference output in `_site_jekyll_recount` directories if available

## Dependencies

- Issue 213 (theme site fixes) -- already done

## Descoped (to be tracked separately)

- **JavaScript syntax highlighting token classes** (Root Cause 3): The `nf` vs `nx` mapping and string delimiter splitting (`dl` vs `s1`) affect all 10 theme sites equally (same code block). This requires changes to both the scope mapping table and the `accumulate_and_emit` function in `src/syntax.rs`. Should be tracked as a separate issue.

## Log

- 2026-03-18: Created from cross-site comparison analysis.
- 2026-03-19: [PM] Groomed. Analyzed DOM diffs for all 10 theme sites. Identified 3 root causes. Scoped issue to site.github fixes only (root causes 1 and 2). Descoped syntax highlighting token differences to a separate issue.

### [SWE] 2026-03-19
- Wrote 5 failing tests (TDD step 1):
  - test_github_repo_url_nil_without_plugin_and_no_explicit_github_config: FAILS -- got Scalar("https://github.com/alexeygrigorev/rustkyll"), expected Nil
  - test_github_config_preserved_when_explicit_github_key: FAILS -- got Scalar("https://github.com/alexeygrigorev/rustkyll"), expected Scalar("https://github.com/custom/repo")
  - test_github_config_build_revision_populated_with_explicit_github_key: FAILS -- got empty string, expected git SHA
  - test_github_repo_url_resolved_with_plugin: expected to pass (verifies existing behavior)
  - test_github_empty_map_gets_computed_defaults: FAILS -- computed defaults not merged into empty map
- Implemented fix in src/generator.rs (build_site_context function, lines 162-215):
  - Root Cause 1: Gated resolve_repository_url behind has_github_metadata_plugin() -- when plugin absent and no explicit github: config, repository_url is Nil
  - Root Cause 2: When _config.yml has explicit github: key, start from those values and merge computed fields as defaults (config wins)
  - Root Cause 2b: populate build_revision from git when explicit github: config exists (primer case)
- All 5 new tests PASS, all 119 generator tests PASS
- 8 pre-existing failures in kramdown.rs (from issue 246 in-progress changes, not related)
- Clippy clean, fmt clean
- Files modified: src/generator.rs, docs/tracker/229-fix-theme-sites-remaining-diffs.in-progress.md
- QA fix: Updated integration test `test_site_context_github_url_resolves` in tests/integration_context.rs to assert repository_url IS Nil (DTC config lacks jekyll-github-metadata plugin), verifying gating behavior at integration level. All tests pass.
