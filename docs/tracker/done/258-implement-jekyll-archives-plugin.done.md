# Issue 258: Implement jekyll-archives plugin

## Problem

The `jekyll-archives` plugin is not implemented. This plugin generates individual archive pages for each category and tag found in posts. It is a hard blocker for the Chirpy theme (issue #236) and is used by minimal-mistakes, academicpages, and other popular themes.

When a site has this in `_config.yml`:

```yaml
jekyll-archives:
  enabled: [categories, tags]
  layouts:
    category: category
    tag: tag
  permalinks:
    tag: /tags/:name/
    category: /categories/:name/
```

Jekyll generates one page per unique tag and one page per unique category. Each generated page uses the specified layout and has `page.title` set to the tag/category name and `page.posts` set to the list of posts with that tag/category (newest first).

## Background: How jekyll-archives works

### Config structure

The `jekyll-archives` key in `_config.yml` has these sub-keys:

- **`enabled`**: Array of archive types to generate. Valid values: `categories`, `tags`. (Also `years`, `months`, `days` for date archives, but those are rare and can be descoped to a follow-up.)
- **`layouts`**: Map of archive type to layout name. E.g., `category: category` means category archive pages use `_layouts/category.html`.
- **`tags`**: Map of archive type to layout name (alternative to `layouts` for tag-specific layout).
- **`permalinks`**: Map of archive type to URL pattern. The `:name` placeholder is replaced with the slugified tag/category name.

### Generated page context

Each archive page gets these template variables:

- `page.title` -- the tag or category name (e.g., "Machine Learning")
- `page.type` -- either `"tag"` or `"category"`
- `page.posts` -- array of post objects belonging to this tag/category, sorted newest-first (same structure as `site.posts` items)
- `page.url` -- the permalink for this archive page

### Permalink resolution

The `:name` placeholder in permalinks is replaced with the **slugified** version of the tag/category name. Jekyll's slug behavior for archives:
- Lowercase the name
- Replace spaces with hyphens
- URL-encode special characters if needed

Example: category "Machine Learning" with permalink `/categories/:name/` produces `/categories/machine-learning/`.

### Interaction with site.tags / site.categories

`site.tags` and `site.categories` already exist in rustkyll (see `build_categories_and_tags` in `generator.rs`). The archives plugin does NOT modify these -- it just reads them to determine which archive pages to generate.

## Scope

### In scope

- Parse the `jekyll-archives` config section from `_config.yml`
- Generate one HTML page per enabled category (when `categories` is in `enabled`)
- Generate one HTML page per enabled tag (when `tags` is in `enabled`)
- Each generated page uses the layout specified in the config
- Each generated page has `page.title`, `page.type`, `page.posts`, and `page.url` in the template context
- Permalink resolution with `:name` placeholder replaced by slugified tag/category name
- Integration into the main `build_site` pipeline (similar to how pagination is integrated at step 10b)

### Descoped (follow-up issues)

- Date-based archives (`years`, `months`, `days` in `enabled`) -- rare feature, can be a separate issue
- The `type` value `"year"`, `"month"`, `"day"` for date archives

## Dependencies

- No blocking dependencies. `site.categories` and `site.tags` already work. The pagination module (`pagination.rs`) provides a pattern for plugin-generated pages.

## Acceptance Criteria

- [ ] `_config.yml` with a `jekyll-archives` section is parsed correctly into a typed config struct
- [ ] When `enabled` contains `categories`, one HTML file is generated per unique post category
- [ ] When `enabled` contains `tags`, one HTML file is generated per unique post tag
- [ ] Generated pages use the layout specified in `jekyll-archives.layouts.category` / `jekyll-archives.layouts.tag`
- [ ] Generated pages have `page.title` set to the category/tag name
- [ ] Generated pages have `page.type` set to `"category"` or `"tag"`
- [ ] Generated pages have `page.posts` as an array of post objects (newest first), matching the posts that have that tag/category
- [ ] Generated pages are written to the correct output path based on `jekyll-archives.permalinks` with `:name` replaced by the slugified name
- [ ] Slugification lowercases the name and replaces spaces with hyphens
- [ ] When `jekyll-archives` is not present in config, no archive pages are generated (no-op)
- [ ] When `enabled` is empty or missing, no archive pages are generated
- [ ] Building the Chirpy theme (`websites/jekyll-theme-chirpy`) produces archive pages under `/tags/` and `/categories/`
- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `./scripts/cargo-safe test` passes with no regressions
- [ ] Tests cover the feature with 8+ new tests

## Test Scenarios

### Unit: Config parsing
- Parse a `_config.yml` with full `jekyll-archives` config (enabled, layouts, permalinks) -- verify all fields extracted
- Parse a `_config.yml` with no `jekyll-archives` key -- verify returns None/default (no archive generation)
- Parse a `_config.yml` with `enabled: []` -- verify no archive types enabled
- Parse a `_config.yml` with only `categories` enabled (not `tags`) -- verify only categories are enabled

### Unit: Slug generation
- Slugify "Machine Learning" -> "machine-learning"
- Slugify "C++" -> appropriate URL-safe slug
- Slugify a name that is already lowercase with no spaces -> unchanged
- Slugify a name with non-ASCII characters (e.g., "Programacao") -> verify graceful handling

### Unit: Permalink resolution
- Permalink `/categories/:name/` with category "Web Development" -> `/categories/web-development/`
- Permalink `/tags/:name/` with tag "rust" -> `/tags/rust/`

### Integration: Archive page generation
- Create a minimal site with 3 posts across 2 categories and 3 tags, with `jekyll-archives` enabled for both -- verify correct number of HTML files generated at correct paths
- Verify each generated HTML file contains the tag/category name (rendered through the layout)
- Verify `page.posts` in the generated output lists the correct posts for each tag/category
- Verify posts within `page.posts` are in reverse chronological order (newest first)

### Integration: No-op when disabled
- Build a site with no `jekyll-archives` config -- verify no extra pages generated
- Build a site with `jekyll-archives` but `enabled: []` -- verify no extra pages generated

### Output verification
- Build `websites/jekyll-theme-chirpy` with rustkyll and verify archive pages exist under `_site/tags/` and `_site/categories/`
- Verify the generated archive pages contain post listings (not empty/broken HTML)

## Log

### [SWE] 2026-03-20
- TDD cycle:
  1. Wrote 17 tests covering config parsing (4 tests), slug generation (5 tests), permalink resolution (2 tests), integration archive page generation (5 tests), and no-op when disabled (1 test)
  2. Ran tests -- FAIL: compilation error due to mismatched types when injecting page.posts into context
  3. Added `render_with_extra_page_fields` method to LayoutEngine to properly inject Liquid values into the page object
  4. Ran tests -- PASS: all 17 tests pass
  5. Fixed `crate::archives::` to `rustkyll::archives::` in main.rs binary crate
  6. Ran full test suite: all 2270+ tests pass, 0 failures
- Files created: `src/archives.rs` (new module)
- Files modified: `src/lib.rs` (added `pub mod archives`), `src/main.rs` (added step 10d), `src/template/layout.rs` (added `render_with_extra_page_fields`)
- Build: clean, no warnings in new code
- `cargo fmt --check`: clean
- clippy: pre-existing vendor warnings only, no new warnings
