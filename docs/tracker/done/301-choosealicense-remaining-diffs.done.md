# Issue 301: choosealicense remaining diffs (17/72 -> target 63+/72)

## Problem

choosealicense.com matches 17/72 (24%). 55 pages have diffs. Analysis of the
diff file at `docs/comparison/dom-details/choosealicense.com.txt` reveals
the diffs fall into a small number of fixable root causes.

## Diff Analysis

### 55 pages: Missing `github_edit_link` nav link

Every diff page includes:
```
body > div > footer > nav > a: missing_element - expected: '<a>', actual: '(none)'
```

The `{% github_edit_link "Help improve this page" %}` tag in
`_includes/footer.html` produces empty output because `site.github` context
is not being populated. The site has `jekyll-github-metadata` in its plugins
list and the git remote is `https://github.com/github/choosealicense.com.git`,
so rustkyll should be populating `site.github.repository_url`,
`site.github.source.branch` (resolving to `gh-pages`), etc.

Root cause: the `github_edit_link` tag checks
`site.github.repository_url` and `site.github.source.branch`. The
`source.branch` is not currently populated by `build_site_context` in
`src/generator.rs`. Only `repository_url`, `build_revision`, and `url` are
populated. The branch must also be resolved (from git or config).

### 44 pages: JSON key ordering in `site.data.rules | jsonify`

The annotations script on license pages uses `{{ site.data.rules | jsonify }}`.
The YAML source `_data/rules.yml` has key order: `permissions`, `conditions`,
`limitations`. But `jsonify` output has `conditions` first because
`liquid::Object` uses `HashMap` (unordered) and `serde_json::Map` uses
`BTreeMap` (alphabetical).

Jekyll preserves YAML insertion order because Ruby hashes are ordered.

Root cause: `yaml_to_liquid` in `src/generator.rs` converts YAML mappings to
`liquid::Object` which is backed by `HashMap`, losing insertion order. The
`jsonify` filter then serializes via `serde_json::Map` which sorts keys
alphabetically.

Fix approach: The `jsonify` filter must serialize object keys in insertion
order. Since `liquid::Object` is a `HashMap` (from the liquid crate, not
under our control), the fix is in the `jsonify` filter itself: track the
original YAML key order and use it during serialization. One approach is to
store a `__key_order` metadata array alongside the object during
`yaml_to_liquid`, then use it in `liquid_to_json`. Another approach is to
use `serde_json::to_string` with a custom serializer that preserves order,
or to build the JSON string manually using the original key order stashed in
the object.

Alternative simpler approach: Since `liquid::Object` iteration order is
non-deterministic, but Ruby's `Hash#to_json` preserves insertion order, the
`jsonify` filter could accept a hidden `__key_order` key (an array of key
names in insertion order) that gets inserted during `yaml_to_liquid` and
stripped during `jsonify` serialization.

### 7 pages: HTML entity escaping in meta description (`&quot;` / `&#39;`)

Licenses with quotes in their description (bsd-4-clause, cc0-1.0, ecl-2.0,
osl-3.0, no-permission, ncsa, upl-1.0) have description meta tags where
rustkyll HTML-escapes quotes (`&quot;`, `&#39;`) but Jekyll does not. The
description comes from the license YAML front matter and goes through the
SEO tag plugin.

Jekyll uses `| escape` which escapes `<`, `>`, `&` but NOT single/double
quotes in meta content attributes. Rustkyll's `escape` filter or the SEO tag
HTML attribute output is over-escaping.

Root cause: The `escape` filter or the meta tag rendering in
`src/template/seo_tag.rs` is using full HTML entity escaping (including
quotes) instead of the minimal escaping Jekyll uses for meta content.

### 2 pages: `class="bullets"` IAL not applied (about, community)

`about.md` and `community.md` use kramdown IAL `{:.bullets}` to add a class
to the following element. On these pages, the class is being applied to the
wrong element (e.g., `<h3>` or `<p>` gets the class instead of the `<ul>`).

### 2 pages: Whitespace normalization in description (ncsa, upl-1.0)

These license descriptions have double spaces in the YAML source that Jekyll
normalizes to single spaces in meta tags. Rustkyll preserves the double
spaces.

### Environment-dependent diffs (NOT fixable, should be excluded)

- Jekyll version string: `Jekyll v3.10.0` vs `v4.4.1` (every page)
- Build timestamps: differ between builds (48 pages)
- `site.github.url` URL pattern: `github.com/pages/github/` vs
  `github.github.io/` (54 pages). This is because the Jekyll cached site was
  built on GitHub Pages infrastructure which uses a different URL scheme than
  local builds. The rustkyll output matches what Jekyll produces locally.

## Scope

### In scope (3 fixes, target 63+/72 matching)

1. **`site.github.source.branch` population** -- populate `source.branch` in
   `build_site_context` so `github_edit_link` works. Resolve from git
   (`git rev-parse --abbrev-ref HEAD`) or use config default.

2. **`jsonify` key ordering** -- preserve YAML insertion order when
   serializing objects. Attach key order metadata during `yaml_to_liquid` and
   use it during `jsonify` serialization.

3. **Meta description quote escaping** -- fix `escape` filter or SEO tag
   rendering to not HTML-escape quotes in meta content attributes, matching
   Jekyll behavior.

### Out of scope (track as follow-up)

- IAL class application order (2 pages: about, community) -- kramdown IAL
  edge case, file follow-up issue
- Description whitespace normalization (2 pages: ncsa, upl-1.0) -- minor
- Environment-dependent diffs (version string, timestamps, GitHub Pages URL)
  -- not bugs, should be excluded from comparison or accepted as known diffs

### Impact estimate

Fixing all 3 in-scope items should bring choosealicense from 17/72 to
approximately 63/72 (excluding env-dependent diffs and 9 remaining edge
case pages). The DOM comparison tool may report ~22/72 matching due to
version/timestamp/URL diffs that are environment-dependent.

## Dependencies

- None. All fixes are independent of other in-progress issues.

## Key Files to Modify

- `src/generator.rs` -- populate `site.github.source.branch` in
  `build_site_context` (around line 200-268)
- `src/generator.rs` -- `yaml_to_liquid` function: attach key order metadata
  to objects converted from YAML mappings
- `src/template/filters/jsonify.rs` -- `liquid_to_json` for objects: use
  stored key order instead of HashMap iteration order
- `src/template/seo_tag.rs` -- meta description rendering: use Jekyll-
  compatible escaping (no quote escaping in content attributes)
- `src/template/filters/mod.rs` or `src/template/filters/escape.rs` -- if
  the `escape` filter itself over-escapes quotes

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes
- [ ] `github_edit_link` produces correct `<a>` tags on choosealicense pages
      when `site.github` is populated from git metadata
- [ ] `site.github.source.branch` is populated (from git HEAD or config) when
      `jekyll-github-metadata` plugin is listed
- [ ] `site.data.rules | jsonify` preserves YAML key order: `permissions`
      before `conditions` before `limitations`
- [ ] Meta description content attributes do NOT HTML-escape quotes (`"` and
      `'` appear as literal characters, not `&quot;`/`&#39;`)
- [ ] choosealicense DOM match count improves (target: at least 22/72
      matching with the DOM comparison tool, acknowledging env-dependent
      diffs; or 63+/72 when excluding version/timestamp/URL diffs)
- [ ] No regressions on DTC, muan-blog, lanyon, or any of the 13+ sites
      currently at 100%
- [ ] Non-ASCII content in license descriptions (curly quotes, em-dashes)
      renders correctly

## Test Scenarios

### Unit: site.github.source.branch population

- Configure a site with `jekyll-github-metadata` plugin listed, build site
  context with a git directory on `gh-pages` branch, verify
  `site.github.source.branch` == "gh-pages"
- Configure a site without the plugin, verify `site.github.source` is not
  populated (or has sensible default)
- Test `github_edit_link` tag with `site.github.repository_url` and
  `site.github.source.branch` set to "gh-pages", verify correct URL like
  `https://github.com/github/choosealicense.com/edit/gh-pages/index.html`

### Unit: jsonify key order preservation

- Create a YAML mapping with keys `permissions`, `conditions`, `limitations`
  (in that order), convert through `yaml_to_liquid`, apply `jsonify` filter,
  verify JSON output has keys in same order
- Create a nested YAML structure (array of objects), verify all levels
  preserve key order through `jsonify`
- Verify that non-YAML objects (programmatically created `liquid::Object`)
  still serialize without error (graceful fallback to alphabetical or
  iteration order)

### Unit: meta description escaping

- Render a page with description containing double quotes: `A "test" license`,
  verify the `<meta content='...'>` attribute contains literal `"` not
  `&quot;`
- Render a page with description containing single quotes/apostrophes:
  `you've created`, verify output contains literal `'` not `&#39;`
- Render a page with description containing `<` and `&`, verify those ARE
  still escaped (only quotes should be unescaped)

### Integration: choosealicense site build

- Build choosealicense.com with rustkyll
- Run DOM comparison against Jekyll cached output
- Verify `index.html` footer nav contains the "Help improve this page" link
- Verify `licenses/mit/index.html` has `window.annotations` JSON with
  `permissions` key before `conditions` key
- Verify `licenses/bsd-4-clause/index.html` meta description contains
  literal `"advertising clause"` (not `&quot;advertising clause&quot;`)
- Verify no regressions: run comparison on DTC and muan-blog sites

### Regression: Unicode content

- License descriptions with curly apostrophes (cc0-1.0: "you've"),
  em-dashes, and non-ASCII characters render correctly in both meta tags
  and JSON-LD

## Output Verification

Build and inspect:
```bash
./scripts/cargo-safe build --release
./target/release/rustkyll build \
  --source websites/choosealicense.com/ \
  --destination /tmp/choosealicense_test
python3 scripts/dom_compare.py \
  --jekyll-dir websites/choosealicense.com/_site_jekyll_cached \
  --rustkyll-dir /tmp/choosealicense_test
```

Spot-check files:
- `/tmp/choosealicense_test/index.html` -- footer nav has edit link
- `/tmp/choosealicense_test/licenses/mit/index.html` -- annotations JSON key order
- `/tmp/choosealicense_test/licenses/bsd-4-clause/index.html` -- unescaped quotes in meta
