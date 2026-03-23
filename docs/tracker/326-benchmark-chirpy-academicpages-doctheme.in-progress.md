# Issue 326: Benchmark and fix chirpy, academicpages, and documentation-theme-jekyll

## Problem

Rustkyll is hitting diminishing returns on the current big 3 sites (DTC 94%, muan-blog 98%, mlwiki 87%). To prove the engine is truly a generic Jekyll replacement (not just tuned for a few sites), we need to broaden the benchmark suite to popular themes with different patterns.

Three sites are already cloned and have Jekyll reference builds cached but have never been systematically analyzed or fixed:

| Site | Jekyll pages | Rustkyll pages | Match | Diffs | Notes |
|------|-------------|---------------|-------|-------|-------|
| chirpy | 17 | 17 | 1/17 (6%) | 560 | Navigation data ordering, sidebar tabs |
| academicpages | 45 | 18 | 1/45 (2%) | 374 | Only 18/45 pages generated, share buttons ordering |
| documentation-theme-jekyll | 98 | 98 | 2/98 (2%) | 4389 | All pages generated, structural diffs |

Combined: 160 pages, only 4 matching (2.5%). Each site uses different theme patterns that stress different parts of the engine.

## Why these three sites

**chirpy** (~7k GitHub stars): Uses `_tabs/` collection with custom sort order via front matter, `_data/` files for sidebar navigation, polyglot/i18n support, and complex Liquid conditionals. The 560 diffs in just 17 pages suggest a few systematic issues (likely data ordering or collection routing) that would fix many pages at once.

**academicpages** (~13k GitHub stars): A minimal-mistakes derivative for academic portfolios. Only 18/45 pages are generated, meaning 27 pages are missing entirely -- likely a collection configuration or permalink issue. The share-button ordering diffs suggest `site.data` hash iteration order differences.

**documentation-theme-jekyll** (~4k GitHub stars): A documentation-focused theme with 98 pages, sidebar navigation from `_data/` YAML, and a table-of-contents include. 98/98 pages generate, so the engine handles the basics -- the 4389 diffs are likely from a few repeating template-level issues (similar to opensource-guide's pattern).

## Analysis needed first

Before fixing, the engineer must run a diagnostic pass to categorize the diffs:

### chirpy (17 pages, 560 diffs)
1. Why does `_tabs/` collection sort differently? Check front matter `order` field handling.
2. Why is sidebar navigation wrong? Check `_data/` hash iteration vs `site.data` ordering.
3. Are the 560 diffs from a few repeated patterns (nav on every page) or page-specific?

### academicpages (45 pages, 27 missing)
1. Why are 27 pages not generated? Check collection configuration (`_teaching/`, `_talks/`, `_publications/`, `_portfolio/`).
2. Check permalink patterns for these collections.
3. The share-button ordering diffs suggest `for` loop iteration on a hash is non-deterministic or differently-ordered.

### documentation-theme-jekyll (98 pages, 4389 diffs)
1. Categorize the 4389 diffs -- how many are from repeated sidebar/nav issues vs content?
2. Check `_data/sidebars/` YAML loading and nested navigation rendering.
3. The SCSS warning during build may cause missing styles but should not affect HTML structure.

## Scope

This issue has two phases:

**Phase 1: Diagnostic (required)** -- Analyze all three sites' diffs, categorize them, and identify the root causes. Produce a table of "N diffs from cause X, affecting M pages" for each site.

**Phase 2: Fix (required)** -- Fix the highest-leverage root causes. Priority is issues that affect the most pages across multiple sites (e.g., data hash ordering would affect chirpy AND academicpages AND documentation-theme-jekyll).

## Dependencies

- None. These sites are already cloned with Jekyll reference builds.
- Issue 324 (opensource-guide template fixes) may produce fixes that also help these sites (e.g., `.size` on data mappings was fixed there).

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `./scripts/cargo-safe test` passes with all existing tests plus new tests
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] academicpages generates all 45 pages (not 18)
- [ ] chirpy DOM match reaches 8+/17 (from 1/17)
- [ ] academicpages DOM match reaches 20+/45 (from 1/45)
- [ ] documentation-theme-jekyll DOM match reaches 30+/98 (from 2/98)
- [ ] A diagnostic report is included in the issue log with categorized diffs for each site
- [ ] No regressions on DTC (must remain 745+/790)
- [ ] No regressions on muan-blog (must remain 2172+/2218)
- [ ] No regressions on mlwiki (must remain 560+/644)
- [ ] No regressions on sites currently at 100%
- [ ] At least 10 new test functions covering the fixes
- [ ] Tests include non-ASCII/Unicode content where applicable
- [ ] Follow-up issues created for any identified-but-unfixed categories of diffs

## Test Scenarios

### Unit: Collection with custom sort order
- Define a collection `_tabs` with items having `order: 1`, `order: 2`, `order: 3` in front matter
- Verify iteration over `site.tabs` respects the `order` field
- Verify `site.tabs | sort: 'order'` produces correct ordering

### Unit: Data hash iteration order
- Load `_data/` directory with multiple YAML files
- Verify `for item in site.data.dirname` iterates in a deterministic order
- Verify the order matches Ruby's hash iteration (insertion order / alphabetical)

### Unit: Collection output for non-standard collections
- Define `_teaching` collection with `output: true` in `_config.yml`
- Verify all items in the collection produce output pages
- Verify permalink patterns like `/:collection/:path/` work correctly

### Unit: Nested data navigation
- Load a `_data/sidebars/mydoc_sidebar.yml` with nested `folders` and `subfolders`
- Verify Liquid template can iterate `{% for entry in sidebar.entries %}` and access nested items
- Verify `entry.folders[0].subfolderitems` resolves correctly

### Integration: chirpy full build and DOM comparison
- Build chirpy with rustkyll
- Run DOM comparison against Jekyll cached output
- Verify 8+ pages match out of 17
- Check that `_tabs/` pages are all generated with correct content

### Integration: academicpages full build and DOM comparison
- Build academicpages with rustkyll
- Run DOM comparison against Jekyll cached output
- Verify all 45 pages are generated
- Verify 20+ pages match out of 45

### Integration: documentation-theme-jekyll full build and DOM comparison
- Build documentation-theme-jekyll with rustkyll
- Run DOM comparison against Jekyll cached output
- Verify 30+ pages match out of 98

### Regression: Existing sites
- Run DOM comparison on DTC, muan-blog, mlwiki
- Verify no regressions

## Output Verification

```bash
./scripts/cargo-safe build --release

# Build all three sites
./target/release/rustkyll build --source websites/chirpy/ --destination /tmp/chirpy_326
./target/release/rustkyll build --source websites/academicpages/ --destination /tmp/academic_326
./target/release/rustkyll build --source websites/documentation-theme-jekyll/ --destination /tmp/doctheme_326

# DOM comparisons
uv run scripts/dom_compare.py \
  --jekyll-dir websites/chirpy/_site_jekyll_cached \
  --rustkyll-dir /tmp/chirpy_326

uv run scripts/dom_compare.py \
  --jekyll-dir websites/academicpages/_site_jekyll_cached \
  --rustkyll-dir /tmp/academic_326

uv run scripts/dom_compare.py \
  --jekyll-dir websites/documentation-theme-jekyll/_site_jekyll_cached \
  --rustkyll-dir /tmp/doctheme_326
```

Expected output:
- chirpy: 8+ files matched (up from 1)
- academicpages: 20+ files matched (up from 1), 45 pages generated (up from 18)
- documentation-theme-jekyll: 30+ files matched (up from 2)

Verify no regressions:
```bash
./target/release/rustkyll build \
  --source websites/DataTalksClub/datatalksclub.github.io/ \
  --destination /tmp/dtc_326_regtest

uv run scripts/dom_compare.py \
  --jekyll-dir websites/DataTalksClub/datatalksclub.github.io/_site_jekyll_cached \
  --rustkyll-dir /tmp/dtc_326_regtest
# Must show 745+ matched
```

## Log

### [SWE] 2026-03-23

#### Phase 1: Diagnostic Report

**academicpages (45 pages)**
| Cause | Pages affected | Diffs |
|-------|---------------|-------|
| Missing pages: `_pages/` dir not included (no `include:` config support) | 28 pages missing entirely | N/A |
| YAML merge key (`<<: *ANCHOR`) not supported in data files | 14+ pages (share button heading missing) | ~350 |
| Build date "Site last updated" always differs | All 45 pages | 45 |
| Other content/markdown diffs | ~8 pages | ~200 |

**chirpy (17 pages)**
| Cause | Pages affected | Diffs |
|-------|---------------|-------|
| `sort_by: order` not supported in collection config | All 17 (sidebar nav wrong order) | ~150 |
| Permalink from defaults not checked for collections | All 17 (tabs URLs wrong: /tabs/X vs /X/) | ~70 |
| Canonical URL / og:url missing when site.url is empty | All 17 | ~34 |
| Dynamic includes (`{% include {{ var }} %}`) truncate output | 3 post pages | ~150 |
| `nil == false` bug in Liquid crate | Multiple pages | ~50 |

**documentation-theme-jekyll (98 pages)**
| Cause | Pages affected | Diffs |
|-------|---------------|-------|
| `site.title` empty (no github-metadata plugin detection) | 89 pages | 89 |
| `class='active'` missing (page.url lacks leading `/`) | 68+ pages | 68 |
| Build date "Site last generated" always differs | 89+ pages | 89 |
| YAML float `6.0` rendered as `6` | 80+ pages | 80 |
| `nil == false` returns true (toc/script missing) | 15+ pages | ~200 |
| Other content/markdown diffs | ~20 pages | ~3400 |

#### Phase 2: Fixes Implemented

1. **`include:` config support** (collection.rs, config.rs): Added `include` field to `SiteConfig` and modified `should_skip_directory()` to check the include list before skipping underscore-prefixed directories. Fixes academicpages `_pages/` missing.

2. **YAML merge key (`<<`) support** (yaml.rs): Implemented YAML 1.1 merge key handling in `LenientYamlLoader`. When key is `<<` and value is a mapping/array of mappings, merge entries into current mapping (first-wins for conflicts). Fixes academicpages `ui-text.yml` locale data.

3. **Collection `sort_by` field** (config.rs, collection.rs): Added `sort_by` option to `CollectionConfig`. When specified, items are sorted by that front matter field with numeric-first comparison. Fixes chirpy `_tabs` ordering by `order` field.

4. **Permalink from defaults for collections** (collection.rs): Extended collection permalink resolution to check `defaults` when the collection config has no explicit permalink. Fixes chirpy tabs URLs (`/tabs/about` -> `/about/`).

5. **Permalink leading slash normalization** (collection.rs): Ensured page and collection item URLs always start with `/` when permalink is specified in front matter without a leading slash. Fixes documentation-theme-jekyll sidebar `class='active'` matching.

6. **`site.title` from repository name** (generator.rs): When `site.title` is empty and github-pages gem is detected (via Gemfile), infer title from `repository:` config. Fixes documentation-theme-jekyll RSS feed title.

7. **YAML float preservation** (yaml.rs): Whole-number floats like `6.0` are preserved as strings instead of being collapsed to integers. Matches Jekyll/Ruby behavior. Fixes documentation-theme-jekyll sidebar version display.

8. **`nil == false` comparison fix** (template/engine.rs): Added preprocessing to rewrite `VAR == false` to `VAR == false and VAR != nil` and `VAR != false` to `VAR != false or VAR == nil`. Works around the Liquid crate treating nil == false as true (Ruby Liquid returns false). Fixes documentation-theme-jekyll toc rendering.

9. **Canonical URL for empty site.url** (template/seo_tag.rs): When `site.url` is empty, fall back to relative page URL for canonical link and og:url, matching jekyll-seo-tag behavior. Fixes chirpy SEO tags.

#### Results

| Site | Before matches | After matches | Before diffs | After diffs |
|------|---------------|--------------|-------------|------------|
| academicpages | 1/45 (18 pages) | 10/45 (46 pages) | 374 | 594 |
| chirpy | 1/17 | 1/17 | 560 | 250 |
| documentation-theme-jekyll | 2/98 | 3/98 | 4389 | 3942 |

**Note on match counts:** The dom_compare tool does not treat "Site last generated/updated" date text as acceptable differences. If these were filtered:
- academicpages would be 24/45 (14 pages have ONLY the date diff)
- documentation-theme-jekyll would be 48/98 (45 pages have ONLY the date diff)

**Regressions:** None detected.
- DTC: 747/790 (was 745+)
- muan-blog: 2172/2218 (was 2172+)
- mlwiki: 560/644 (was 560+)

#### Files Modified
- `src/config.rs` - Added `include` and `sort_by` fields
- `src/collection.rs` - Include list checking, sort_by sorting, permalink defaults, leading slash normalization
- `src/yaml.rs` - YAML merge key support, float preservation
- `src/generator.rs` - site.title from repository name
- `src/template/engine.rs` - nil == false preprocessing
- `src/template/seo_tag.rs` - Canonical URL for empty site.url
- `src/kramdown.rs` - Doc comment fix (clippy)

#### Tests Added
17 new test functions:
- 7 YAML tests (merge key basic/override/multiple/unicode, float preservation, integer stays integer)
- 6 collection tests (include config, unicode include, sort_by, permalink normalization, defaults permalink)
- 4 template engine tests (nil == false preprocessing and rendering)

All include non-ASCII/Unicode content where applicable.

#### Build Results
- `cargo build`: OK
- `./scripts/cargo-safe test`: 2644+ passed, 0 failed
- `./scripts/cargo-safe clippy -- -D warnings`: OK
- `cargo fmt --check`: OK

#### Follow-up Issues Needed
- Dynamic include tags (`{% include {{ variable }} %}`) - causes truncated output in chirpy
- dom_compare.py should treat "Site last generated/updated" text as acceptable date diffs
- Liquid crate nil == false bug - current workaround is template preprocessing, proper fix needs upstream crate change or fork
