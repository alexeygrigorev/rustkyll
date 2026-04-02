# Issue 543: Data preprocessing for `_data` auto-version resolution

## Problem

jekyll-vitepress-theme uses a Ruby `post_read` hook (`VersionLabel.apply`) that detects `current: auto` in `_data/versions.yml` and replaces it with `v{VERSION}` where VERSION comes from the gem's `version.rb` (currently `1.1.1`). The hook lives at `lib/jekyll/vitepress_theme/hooks.rb:86-106`.

Since rustkyll does not execute Ruby hooks, the version string stays as the literal `auto`, causing 2 text_differs diffs on every page (one in the nav version button, one in the mobile nav version label). That is 34 diffs across all 17 pages.

### How it works in Jekyll

1. `_data/versions.yml` contains `current: auto`
2. The `site:post_read` hook fires `VersionLabel.apply(site)`
3. It checks `site.data['versions']['current']` -- if it case-insensitively equals `"auto"`, it replaces the value with `"v#{Jekyll::VitePressTheme::VERSION}"`
4. The VERSION constant is `"1.1.1"` (from `lib/jekyll/vitepress_theme/version.rb`)
5. The same version appears in `Gemfile.lock` as `jekyll-vitepress-theme (1.1.1)`

### Where the version appears in output

- `_includes/nav.html:92` -- `{{ versions.current | default: 'Version' }}`
- `_includes/nav.html:253` -- `{{ versions.current | default: 'Version' }}`

## Scope

After `load_data()` returns the data tree, apply a post-processing step that detects the `auto` pattern and resolves the version from `Gemfile.lock`.

### Implementation approach

1. Add a `postprocess_data()` function in `src/data.rs` that takes the mutable data tree and the site source path
2. If `data["versions"]` is a mapping and `data["versions"]["current"]` is a string that case-insensitively equals `"auto"`:
   a. Parse `Gemfile.lock` in the site source directory
   b. Look for the theme gem name from `_config.yml`'s `theme` key (e.g., `jekyll-vitepress-theme`)
   c. Extract the version with a regex like `^\s+{gem_name}\s+\((\d+\.\d+\.\d+)\)` from the SPECS section
   d. Replace `"auto"` with `"v{version}"`
3. If `Gemfile.lock` is missing or the gem is not found, leave the value as-is (log a warning)

### Why this is not site-specific

This approach is generic: it works for any Jekyll theme that uses the `current: auto` pattern with a gem version from `Gemfile.lock`. The theme name comes from `_config.yml`, not hardcoded. Currently only jekyll-vitepress-theme uses this pattern, but the implementation should work for any theme.

## Dependencies

None.

## Split from

Issue #443 (jekyll-vitepress-theme rendering issues) -- RC2.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes (including new tests)
- [ ] When `_data/versions.yml` has `current: auto` and a `Gemfile.lock` with a matching theme gem, the data tree resolves `auto` to `v{version}` (e.g., `v1.1.1`)
- [ ] When `Gemfile.lock` is missing or the theme gem is not listed, `auto` is left as-is (no crash)
- [ ] When `current` is not `"auto"` (e.g., `"v2.0.0"`), no replacement happens
- [ ] DTC DOM match count must not drop below 790/790 (baseline: 596 matched + 194 with diffs = 790 total)
- [ ] jekyll-vitepress-theme DOM total diffs must decrease by 34 (from 643 to 609 or fewer)
- [ ] The version button in nav renders `v1.1.1` instead of `auto` on all 17 vitepress pages

## Test Scenarios

### Unit: auto-version detection

- `_data/versions.yml` with `current: auto` and `Gemfile.lock` containing `jekyll-vitepress-theme (1.1.1)` with theme `jekyll-vitepress-theme` in config -- resolves to `v1.1.1`
- `_data/versions.yml` with `current: AUTO` (case-insensitive) -- resolves correctly
- `_data/versions.yml` with `current: auto` but no `Gemfile.lock` -- left as `auto`, no error
- `_data/versions.yml` with `current: auto` but gem not found in `Gemfile.lock` -- left as `auto`, no error
- `_data/versions.yml` with `current: v2.0.0` (not auto) -- no replacement
- `_data/versions.yml` missing entirely -- no crash
- No `versions` key in data tree -- no crash

### Unit: Gemfile.lock version extraction

- Parse `Gemfile.lock` with `jekyll-vitepress-theme (1.1.1)` -- extracts `1.1.1`
- Parse `Gemfile.lock` with `some-other-theme (0.5.2)` -- extracts `0.5.2`
- Parse `Gemfile.lock` where gem appears in both SPECS and DEPENDENCIES -- extracts from SPECS (indented with spaces)
- Empty `Gemfile.lock` -- returns None

### Integration: vitepress site build

- Build `websites/jekyll-vitepress-theme` and verify the generated HTML contains `v1.1.1` in the nav version spans (not `auto`)
- Run DOM comparison and verify total diffs decreased by 34

## Baseline

- DTC: 790/790 (596 exact match + 194 with differences, 255 total diffs)
- jekyll-vitepress-theme: 0/17 matched, 643 total diffs (34 of which are the `auto` diffs this issue fixes)
