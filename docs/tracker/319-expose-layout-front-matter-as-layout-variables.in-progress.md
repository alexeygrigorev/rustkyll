# Issue 319: Expose layout front matter as `layout.*` template variables

## Problem

Jekyll makes all front matter keys from the current layout available to
templates via `layout.<key>`. For example, if `_layouts/base.html` has:

```yaml
---
layout: default
common-css:
  - "/assets/css/beautifuljekyll.css"
common-ext-css:
  - href: "https://cdn.example.com/bootstrap.min.css"
    sri: "sha384-..."
---
```

Then templates can access `layout.common-css` and `layout.common-ext-css` to
iterate over and emit `<link>` elements.

Currently, rustkyll's `extract_layout_front_matter()` in
`src/template/layout.rs` only extracts the `layout` key (for parent layout
chaining) and discards all other front matter. This means `layout.*` variables
are always empty/nil in templates.

### Impact

This affects at least 10 benchmark sites:

- **beautiful-jekyll** (5 pages) -- `layout.common-css`, `layout.common-ext-css`,
  `layout.common-js`, `layout.common-ext-js` used to emit CSS/JS `<link>` and
  `<script>` tags. All 5 pages missing CSS links.
- **chirpy** / **jekyll-theme-chirpy** (13 pages each) -- `layout.panel_includes`,
  `layout.tail_includes`, `layout.script_includes` used to control page
  structure.
- **just-the-docs** (47 pages) -- `layout.nav_enabled` controls nav rendering.
- **documentation-theme-jekyll** (100 pages) -- `layout.comments` controls
  comment sections.
- **academicpages** (17 pages) -- `layout.author_profile` controls sidebar.
- **minimal-mistakes** (1 page) -- `layout.author_profile` for archive pages.
- **bitcoin-org**, **homebrew-site**, **uswds-site** -- various layout-level
  settings.

## Scope

### In scope

1. **Parse and store all layout front matter** -- modify `extract_layout_front_matter()`
   to return the full front matter HashMap, not just the `layout` key.
2. **Store front matter in the Layout struct** -- add a `front_matter` field to
   the `Layout` struct alongside `source` and `parent_layout`.
3. **Populate `layout.*` in template context** -- when rendering a template with
   a layout, insert the layout's front matter values into the Liquid context
   under the `layout` key (as a Liquid Object).
4. **Handle layout chaining** -- when layouts chain (e.g., `post` -> `base` ->
   `default`), the `layout` variable should reflect the front matter of the
   **innermost layout** that the page directly specifies, matching Jekyll's
   behavior. (In Jekyll, `layout.*` always refers to the current layout being
   rendered at each level of the chain.)

### Out of scope

- Fixing all remaining DOM diffs for the affected sites (separate issues)
- Layout inheritance/merging of front matter across the chain
- `layout.layout` (the parent layout name) -- this is already handled via the
  chaining mechanism

## Dependencies

- None. This is an independent infrastructure fix.

## Key Files to Modify

- `src/template/layout.rs` -- `extract_layout_front_matter()`, `Layout` struct,
  layout loading, and layout rendering (where context is built)
- `src/generator.rs` -- where render context is assembled and passed to layout
  rendering, ensure `layout` object is populated

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes
- [ ] Layout front matter (all keys except `layout`) is parsed and stored
- [ ] Template context includes `layout.<key>` for all front matter keys from
      the active layout
- [ ] `layout.common-css` returns an array when the layout defines it as a YAML
      list
- [ ] `layout.common-ext-css` returns an array of objects (with `href`, `sri`
      keys) when defined as such
- [ ] `layout.nav_enabled` returns a boolean `false` when defined as `false`
- [ ] `layout.author_profile` returns a boolean when defined as boolean
- [ ] Layout chaining works: a page using layout `post` (which inherits from
      `base`) correctly sees `layout.*` from `post` when rendering post's
      template, and from `base` when rendering base's template
- [ ] beautiful-jekyll builds and all 5 common pages contain CSS `<link>` tags
      for bootstrap, font-awesome, Google Fonts, bootstrap-social, and
      beautifuljekyll.css
- [ ] beautiful-jekyll DOM comparison improves from 0/5 to 3+/5
- [ ] No regressions: DTC remains 745+/790, muan-blog remains 2174+/2218
- [ ] All sites currently at 100% (architect-theme, cayman-theme, slate-theme,
      hacker-theme, dinky-theme, midnight-theme, merlot-theme, leap-day-theme,
      primer-theme, time-machine-theme, mojombo-blog, large-blog-3000,
      large-docs-site, kids-horror-stories-ru, alexeygrigorev.github.io,
      DataTalksClub/courses, DataTalksClub/docs) remain at 100%
- [ ] Tests include non-ASCII content (layout front matter with Unicode values)

## Test Scenarios

### Unit: Layout front matter parsing

- Parse a layout with `common-css: ["/a.css", "/b.css"]`, verify array returned
- Parse a layout with `common-ext-css` containing objects with `href`/`sri`,
  verify object array returned
- Parse a layout with boolean `nav_enabled: false`, verify boolean value
- Parse a layout with string value `category: "What's new"`, verify string
- Parse a layout with no front matter, verify empty front matter map
- Parse a layout with only `layout: default`, verify empty map (layout key
  itself is excluded from the front matter map)
- Parse a layout with Unicode value: `title: "Bibliotheque"`, verify preserved

### Unit: Layout variable in template context

- Render `{{ layout.common-css | size }}` with a layout that has 2 CSS entries,
  verify output is `2`
- Render `{% for css in layout.common-css %}{{ css }}{% endfor %}`, verify both
  paths emitted
- Render `{{ layout.nav_enabled }}`, verify `false` output
- Render `{{ layout.nonexistent }}`, verify empty/nil output (no error)

### Integration: beautiful-jekyll site build

- Build beautiful-jekyll with rustkyll
- Verify `index.html` contains `bootstrap.min.css` link
- Verify `index.html` contains `beautifuljekyll.css` link
- Verify `index.html` contains `font-awesome` link
- Run DOM comparison, verify 3+/5 pages match

### Integration: Regression check

- Build DTC, verify no regression in page count or DOM matches
- Run `cargo test` full suite
- Verify all 100% sites remain at 100%

## Output Verification

```bash
./scripts/cargo-safe build --release

# beautiful-jekyll should now have CSS links
./target/release/rustkyll build \
  --source websites/beautiful-jekyll/ \
  --destination /tmp/bj_319

grep 'bootstrap.min.css' /tmp/bj_319/index.html
# Must find the link tag

grep 'beautifuljekyll.css' /tmp/bj_319/index.html
# Must find the link tag

grep 'font-awesome' /tmp/bj_319/index.html
# Must find the link tag

# DOM comparison
uv run scripts/dom_compare.py \
  --jekyll-dir websites/beautiful-jekyll/_site_jekyll_cached \
  --rustkyll-dir /tmp/bj_319
# Target: 3+/5 matched

# Regression check
./target/release/rustkyll build \
  --source websites/DataTalksClub/datatalksclub.github.io \
  --destination /tmp/dtc_319
uv run scripts/dom_compare.py \
  --jekyll-dir websites/DataTalksClub/datatalksclub.github.io/_site_jekyll_cached \
  --rustkyll-dir /tmp/dtc_319
# Must remain 745+/790
```
