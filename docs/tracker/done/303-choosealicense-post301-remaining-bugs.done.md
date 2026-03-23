# Issue 303: choosealicense post-301 remaining bugs (17/72 -> 60+/72)

## Problem

Issue 301 committed fixes for choosealicense (source.branch, jsonify key
order, meta escaping) but the score stayed at 17/72. Investigation reveals:

1. **The jsonify key order fix introduced a regression**: a `__key_order`
   metadata array is leaking into rendered HTML on 38 pages, appearing as a
   visible list item in "Using this license" project lists.
2. **The `github_edit_link` fix works on regular pages but not on collection
   item pages** (licenses). 24 license pages are missing the edit link.
3. **Project sort order differs** from Jekyll on 23 license pages.
4. **Double-slash in breadcrumb URLs** on every page with JSON-LD breadcrumbs.
5. **Environment-dependent diffs** (version string, timestamps, GitHub Pages
   URL pattern) affect all 55 diff pages but are not real bugs.

Fixing items 1-4 should bring choosealicense from 17/72 to approximately
60+/72 (with the remaining ~12 pages having env-dependent diffs or minor
edge cases).

## Diff Analysis

### Bug A: `__key_order` metadata leaking into HTML -- 38 pages -- CRITICAL

The issue 301 jsonify fix stores a `__key_order` array as a property on
`liquid::Object` instances during `yaml_to_liquid`. This metadata is meant
to be consumed only by the `jsonify` filter. However, when templates iterate
over objects (e.g., `{% for project in site.data.rules.permissions %}`), the
`__key_order` key appears as a real data item and renders in the HTML.

On choosealicense, this manifests as an extra `<li>` containing
`__key_order` in the "Using this license" project lists.

**Root cause:** `__key_order` is stored as a regular key in the
`liquid::Object` HashMap. Any `{% for %}` loop or `.size` filter will see it.

**Fix:** Either:
- Strip `__key_order` during iteration (modify the Liquid for-loop to skip it)
- Move key order tracking out of the object itself (e.g., use a side channel
  like a global registry keyed by object identity)
- Use a naming convention that is guaranteed not to conflict and filter it
  from all iteration points

The cleanest approach is to strip `__key_order` from objects before they
enter the template context, and only use it inside the `jsonify` filter by
looking it up from a separate registry.

### Bug B: `github_edit_link` not resolving collection item source path -- 24 pages

The `github_edit_link` tag generates an edit URL from
`site.github.repository_url`, `site.github.source.branch`, and the page's
source file path. For regular pages (about.md, index.html), the source path
is correctly resolved. For collection items (e.g., `_licenses/mit.txt`), the
source path is empty or not set, so the tag produces no output.

Jekyll resolves the source path for collection items using `page.path`, which
for a license like MIT would be `_licenses/mit.txt`.

**Root cause:** The `page.path` variable for collection items is either not
set or does not include the collection directory prefix (`_licenses/`).

**Fix:** Ensure `page.path` for collection items is set to the relative path
including the collection directory (e.g., `_licenses/mit.txt`). Check how
`page.path` is populated in `src/generator.rs` or `src/collection.rs` for
collection items.

### Bug C: Project sort order in "Using this license" lists -- 23 pages

License pages have a "Using this license" section listing notable projects.
The project data comes from YAML front matter (or data files) as a hash/
object. Jekyll iterates hashes in insertion order (Ruby Hash preserves
insertion order). Rustkyll iterates in HashMap order (non-deterministic) or
BTreeMap order (alphabetical).

Example on MIT license page:
- Jekyll: Babel, .NET, Rails (insertion order)
- Rustkyll: .NET, Babel, Rails (alphabetical)

**Root cause:** Same as the jsonify ordering issue -- `liquid::Object` uses
HashMap. The `__key_order` fix addressed jsonify serialization but not
template iteration order.

**Fix:** This is conceptually the same problem as jsonify key order. If
`__key_order` metadata exists on an object, `{% for %}` loops should iterate
in that order. This requires modifying the Liquid for-loop implementation to
respect `__key_order` when present.

Note: This fix naturally addresses Bug A if `__key_order` is used to control
iteration order but filtered out from the visible items.

### Bug D: Double-slash in breadcrumb JSON-LD URLs -- every page with breadcrumbs

The SEO tag breadcrumb JSON-LD generates `@id` URLs by concatenating
`site.github.url` (or `site.url`) with `page.url`. When `site.github.url`
ends with `/` and `page.url` starts with `/`, the result has `//`:
`https://github.github.io/choosealicense.com//about/`

**Root cause:** URL joining does not normalize double slashes.

**Fix:** When building breadcrumb `@id` URLs, strip trailing `/` from the
base URL or leading `/` from the page URL before concatenation. Or use a
URL join function that normalizes.

Note: The `github.com/pages/github/` vs `github.github.io/` part of the URL
is environment-dependent (the cached Jekyll site was built on GitHub Pages
infrastructure) and NOT a bug.

### Environment-dependent diffs -- NOT bugs, 55 pages

These affect every diff page and cannot be "fixed" because they depend on
build environment:

- **Jekyll version string**: `Jekyll v3.10.0` (cached) vs `Jekyll v4.4.1`
  (rustkyll reports current Jekyll version). All 55 diff pages.
- **Build timestamps**: `2026-03-20T23:47:36+01:00` vs current build time.
  45 pages.
- **GitHub Pages URL pattern**: `github.com/pages/github/choosealicense.com/`
  vs `github.github.io/choosealicense.com/`. The cached site was built on
  GitHub Pages infrastructure which uses a different URL scheme. 149 diffs.

**Action:** These should be excluded from the comparison. The DOM comparison
tool should either be configured to ignore these patterns or the cached
Jekyll site should be rebuilt locally. For scoring purposes, pages where ALL
diffs are environment-dependent should count as matching.

### Minor real diffs -- out of scope

- **IAL `{:.bullets}` class** (3 pages: about, community, no-permission) --
  the `{:.bullets}` kramdown IAL is applied to `<p>` instead of `<ul>`. This
  is a kramdown IAL edge case.
- **JSON-LD `&#39;` escaping** (2 pages: no-permission, osl-3.0) -- JSON-LD
  description has HTML-escaped quotes. The meta tag fix from 301 does not
  cover JSON-LD `jsonify` output.
- **Meta extra attributes** (1 page: ecl-2.0) -- description containing
  quotes parsed as HTML attributes by the DOM comparison tool (this is a
  Jekyll bug where the description breaks the HTML attribute).
- **Description whitespace normalization** (2 pages: ncsa, upl-1.0) -- double
  spaces not normalized.

## Scope

### In scope (4 fixes, target 60+/72)

1. **Fix `__key_order` leak** -- prevent `__key_order` from appearing in
   template iteration. Either filter it from for-loops or move it to a side
   channel.

2. **Fix `github_edit_link` for collection items** -- set `page.path` for
   collection items to include the collection directory prefix (e.g.,
   `_licenses/mit.txt`).

3. **Fix iteration order for objects with `__key_order`** -- make `{% for %}`
   loops iterate in insertion order when `__key_order` metadata is present.

4. **Fix double-slash in breadcrumb URLs** -- normalize URL joining in
   breadcrumb `@id` generation.

### Out of scope (minor, track as follow-up)

- IAL `{:.bullets}` class application (3 pages)
- JSON-LD `&#39;` escaping (2 pages)
- Meta extra attributes / ecl-2.0 (1 page)
- Description whitespace normalization (2 pages)
- Environment-dependent diffs (all pages)

## Dependencies

- Issue 301 (choosealicense remaining diffs) -- DONE (committed). This issue
  fixes regressions and remaining bugs from that work.

## Key Files to Modify

- `src/generator.rs` -- `yaml_to_liquid` function: change how `__key_order`
  metadata is stored (side channel instead of in-object, or filtered during
  iteration)
- `src/template/filters/jsonify.rs` -- `liquid_to_json`: adapt to new
  key order storage mechanism
- `vendor/liquid-lib/` or `src/template/engine.rs` -- for-loop iteration:
  respect `__key_order` when iterating objects, filter out the metadata key
- `src/collection.rs` or `src/generator.rs` -- set `page.path` for collection
  items to include collection directory prefix
- `src/template/seo_tag.rs` -- breadcrumb `@id` URL generation: normalize
  double slashes
- `src/template/tags/github_edit_link.rs` or equivalent -- verify it uses
  `page.path` correctly

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes
- [ ] `__key_order` does NOT appear anywhere in rendered HTML output for any
      site (grep all output for `__key_order` must return 0 results)
- [ ] `{% for %}` loops over YAML-sourced objects iterate in YAML insertion
      order (not alphabetical, not random)
- [ ] `site.data.rules | jsonify` still preserves YAML key order (the
      issue 301 fix is not regressed)
- [ ] `github_edit_link` produces correct `<a>` tags on license pages: e.g.,
      `https://github.com/github/choosealicense.com/edit/gh-pages/_licenses/mit.txt`
- [ ] `page.path` for collection items includes the collection directory
      prefix (e.g., `_licenses/mit.txt`, not `mit.txt`)
- [ ] Breadcrumb JSON-LD `@id` URLs have no double slashes: e.g.,
      `https://choosealicense.com/about/` (not `https://choosealicense.com//about/`)
- [ ] choosealicense "Using this license" project lists match Jekyll's
      insertion order (e.g., MIT shows Babel, .NET, Rails -- not alphabetical)
- [ ] choosealicense DOM match count improves significantly. Target: at least
      22/72 with the current comparison tool (acknowledging env-dependent
      diffs), or 60+/72 when env-dependent diffs are excluded
- [ ] No regressions on DTC, muan-blog, lanyon, mlwiki, or any of the 13+
      sites currently at 100%
- [ ] Non-ASCII content in license descriptions and project names renders
      correctly

## Test Scenarios

### Unit: `__key_order` not leaked

- Create a YAML object with keys `a`, `b`, `c`, convert through
  `yaml_to_liquid`, iterate with `{% for pair in obj %}`, verify the output
  contains only `a`, `b`, `c` (no `__key_order`)
- Create a YAML object, apply `.size` filter, verify the count does not
  include `__key_order`
- Create a YAML object, apply `| jsonify`, verify `__key_order` does not
  appear in the JSON string

### Unit: Insertion order iteration

- Create a YAML mapping `{z: 1, a: 2, m: 3}`, convert through
  `yaml_to_liquid`, iterate with `{% for pair in obj %}`, verify output order
  is `z`, `a`, `m` (insertion order, not alphabetical)
- Create nested YAML: `{outer: {z: 1, a: 2}}`, iterate inner object, verify
  insertion order preserved

### Unit: Collection item page.path

- Create a collection item in `_licenses/mit.txt`, build the site, verify
  `page.path` is `_licenses/mit.txt`
- Create a regular page `about.md`, verify `page.path` is `about.md`
  (no regression)
- Create a collection item with Unicode filename, verify path is correct

### Unit: github_edit_link for collection items

- Set `site.github.repository_url` to `https://github.com/org/repo`,
  `site.github.source.branch` to `gh-pages`, and `page.path` to
  `_licenses/mit.txt`. Render `{% github_edit_link "Edit" %}`, verify output
  is `<a href="https://github.com/org/repo/edit/gh-pages/_licenses/mit.txt">Edit</a>`

### Unit: Breadcrumb URL normalization

- Set `site.url` to `https://example.com/`, `page.url` to `/about/`, render
  breadcrumb JSON-LD, verify `@id` is `https://example.com/about/` (no `//`)
- Set `site.url` to `https://example.com` (no trailing slash), `page.url` to
  `/about/`, verify `@id` is `https://example.com/about/`

### Integration: choosealicense site build

- Build choosealicense.com with rustkyll
- Run DOM comparison against Jekyll cached output
- Verify `__key_order` does not appear in any output file:
  `grep -r __key_order /tmp/choosealicense_test/` returns 0 results
- Verify `licenses/mit/index.html` footer nav contains "Help improve this
  page" link pointing to `_licenses/mit.txt`
- Verify `licenses/mit/index.html` "Using this license" lists Babel first
  (not .NET)
- Verify `about/index.html` breadcrumb `@id` has no `//`
- Verify match count improves (target: 22+ with env diffs, 60+ without)

### Regression: Other sites

- Run `cargo test` full suite
- Verify DTC, muan-blog match counts unchanged
- Verify all 13+ sites at 100% remain at 100%
- Run choosealicense comparison and verify no new diff categories introduced

## Output Verification

```bash
./scripts/cargo-safe build --release
./target/release/rustkyll build \
  --source websites/choosealicense.com/ \
  --destination /tmp/choosealicense_test

# Critical check: no __key_order leak
grep -r "__key_order" /tmp/choosealicense_test/
# Must return 0 results

# DOM comparison
python3 scripts/dom_compare.py \
  --jekyll-dir websites/choosealicense.com/_site_jekyll_cached \
  --rustkyll-dir /tmp/choosealicense_test

# Spot checks
grep "Help improve" /tmp/choosealicense_test/licenses/mit/index.html
# Must show: <a href=".../_licenses/mit.txt">Help improve this page</a>

grep "Babel" /tmp/choosealicense_test/licenses/mit/index.html
# Must show Babel before .NET in the list

grep "choosealicense.com//" /tmp/choosealicense_test/about/index.html
# Must return 0 results (no double slash)
```
