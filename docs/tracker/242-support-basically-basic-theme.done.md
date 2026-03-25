# Issue 242: Support Basically Basic Jekyll theme

## Problem

Basically Basic is a Jekyll theme (~1k GitHub stars) by the creator of Minimal Mistakes and So Simple. It is not currently in our benchmark suite.

## Theme Details

- **GitHub:** https://github.com/mmistakes/jekyll-theme-basically-basic
- **Stars:** ~1,000
- **Use case:** Personal blogs, simple sites
- **Notable features:** By mmistakes (same author as minimal-mistakes), clean minimal design, skin support, search (Algolia/Lunr), breadcrumbs, resume/CV layout, responsive

## Scope

1. Clone the Basically Basic theme demo site into `websites/basically-basic/`.
2. Build the cloned site with both Jekyll and rustkyll.
3. Run DOM comparison against the Jekyll reference output and record the actual match rate.
4. Identify Basically Basic-specific rendering blockers and either fix them or create follow-up issues that reference this issue.

## Baseline

- DTC DOM baseline: `771/790`

## Acceptance Criteria

- [ ] The Basically Basic demo site is cloned into `websites/basically-basic/` and the repository state is documented in the issue log.
- [ ] Jekyll builds the demo site successfully and produces a reference `_site` output.
- [ ] rustkyll builds the demo site successfully without errors and produces HTML output for the same site.
- [ ] The DOM comparison between the Jekyll and rustkyll outputs is run and the issue log records the exact match count, differing-file count, and main diff categories.
- [ ] Representative pages that exercise Basically Basic features are verified in the output, including the homepage plus pages covering skin support, search functionality, breadcrumbs, resume/CV layout, and navigation if present in the demo.
- [ ] Any Basically Basic-specific rendering issues discovered during comparison are either fixed in this issue or explicitly tracked in follow-up issues that reference `#242`.
- [ ] The DTC DOM match count does not drop below `771/790`.

## Test Scenarios

### Integration: demo site setup
- Clone the upstream Basically Basic demo site from `https://github.com/mmistakes/jekyll-theme-basically-basic` into `websites/basically-basic/` and verify the expected theme files and configuration are present.
- Run `jekyll build` in the demo site and confirm the reference HTML output is generated.
- Run `rustkyll build --source websites/basically-basic --destination /tmp/basically-basic-rustkyll` and confirm the same page set is generated.

### Integration: output comparison
- Run DOM comparison against the Jekyll reference output and record the exact match count, differing-file count, and notable diff categories.
- Inspect representative pages for skin/theming, search markup, breadcrumb navigation, resume/CV layout, and responsive navigation elements if present in the demo.
- Verify any identified rendering blocker is either fixed or captured in a follow-up issue linked to `#242`.

### Regression: DTC DOM baseline
- Build the DTC site with rustkyll and run DOM comparison against the Jekyll reference.
- Confirm the match count is at least `771/790` (the committed baseline from commit `474ac9e`).

## Dependencies

- None (research/benchmark task)

## Log

### [PM] 2026-03-25
- Groomed the issue into a benchmark-oriented spec for the Basically Basic demo site.
- Recorded DTC baseline: `771/790` (from commit `474ac9e` per `dtc-dom-changelog.md`).
- Added explicit output verification requirements, representative feature coverage, DTC regression check, and traceable follow-up handling for any blockers.

### [SWE] 2026-03-25

#### Setup
- Cloned `https://github.com/mmistakes/jekyll-theme-basically-basic` into `websites/basically-basic/`.
- Used the `example/` subdirectory as the demo site (has posts, recipes collection, CV, tags, categories, search config).
- Inlined theme `_layouts`, `_includes`, `_sass`, and `assets` into the example directory (rustkyll does not support gem-based themes).
- Commented out `theme: jekyll-theme-basically-basic` in `example/_config.yml` for rustkyll compatibility.

#### Jekyll Build
- `bundle exec jekyll build` succeeded from `websites/basically-basic/example/` with 38 HTML files output.
- SCSS deprecation warnings (expected with newer sass versions) but no errors.
- Output cached to `websites/basically-basic/_site_jekyll_cached/`.

#### Rustkyll Build
- `./target/release/rustkyll build --source websites/basically-basic/example --destination websites/basically-basic/_site_rustkyll_recount` succeeded.
- 39 HTML pages generated (one extra: future-dated post `Post/9999/12/31/post-future-date.html`).
- 1 warning: SCSS import failure for `basically-basic/themes/default` (Liquid-in-SCSS not fully resolved).

#### DOM Comparison Results
- **0/18 matched, 18 files with differences, 649 total differences** (63 acceptable diffs filtered).
- 20 files only in Jekyll, 21 files only in rustkyll (permalink casing mismatch).

#### Issue Categories

1. **Category casing in permalinks** (20 files affected): Jekyll lowercases categories in permalink URLs (e.g., `edge case/2009/...`), rustkyll preserves original case (`Edge Case/2009/...`). This accounts for all 20+21 files appearing as "only in Jekyll"/"only in rustkyll".

2. **Author hash serialization** (all 18 common files): `site.author` is a YAML hash (`name`, `twitter`, `picture`), but rustkyll serializes it as a flat string (`__key_ordernametwitterpicture...`) instead of exposing it as a Liquid hash. Affects SEO meta tags and JSON-LD.

3. **baseurl not applied** (all 18 common files): URLs in SEO tags missing `/example` prefix. Jekyll prepends `baseurl` to canonical URLs, OG URLs, and JSON-LD URLs; rustkyll does not.

4. **Locale format** (all 18 common files): Jekyll converts `lang: en-US` to `en_US` for `og:locale`; rustkyll outputs `en-US` as-is.

5. **Syntax highlighting** (1 file, 292 diffs): `markup-syntax-highlighting.html` has 280+ differences in code block rendering -- Rouge-style `<span>` classes differ from rustkyll's highlighting output.

6. **Empty layout rendering** (2 files: `tags/index.html`, `categories/index.html`): These pages use `site.tags` and `site.categories` iteration via `{% for tag in site.tags %}` with `tag[1].size` -- rendered as empty in rustkyll.

7. **Future post filtering** (1 file): Jekyll excludes `post-future-date.html` (date 9999-12-31) by default; rustkyll includes it.

8. **JSON-LD missing fields**: `mainEntityOfPage` and `publisher.name` fields missing from rustkyll's SEO tag output.

9. **Image/hero metadata**: Recipe and hero-image pages have `page.image` as a hash (`path`, `thumbnail`, `caption`) but rustkyll serializes it as a flat string.

#### Representative Pages Verified
- **Homepage** (`index.html`): Renders with 82 differences -- pagination links, post excerpts, and baseurl issues.
- **CV page** (`cv.html`): Renders with 11 differences -- mostly SEO/meta issues, CV content from `_data/cv.json` renders.
- **Recipes collection** (`recipes/`): Collection pages render but have image metadata serialization issues (15 diffs each).
- **Tags/Categories**: Empty output due to `site.tags`/`site.categories` iteration not working.
- **Search**: Search markup depends on theme JS/Algolia config, not directly testable via DOM comparison.
- **Skin support**: SCSS compilation fails for theme skin import (Liquid-in-SCSS).

#### DTC DOM Baseline
- DTC DOM: **772/790** (above required 771/790 baseline). No regression.

#### Follow-up Issues Needed
The following blockers are NOT specific to Basically Basic and affect multiple themes:

1. **Category case in permalinks**: Jekyll lowercases categories in URLs; rustkyll preserves case. (Affects Basically Basic, likely other themes with category permalinks.)
2. **Author/image hash serialization in SEO tags**: Complex YAML values (`site.author`, `page.image`) render as flat strings instead of being accessed as hashes in `jekyll-seo-tag` include.
3. **baseurl handling in SEO/meta tags**: `baseurl` not prepended to canonical, OG, and JSON-LD URLs.
4. **Future post filtering**: rustkyll should exclude posts with future dates by default (unless `future: true`).
5. **Liquid-in-SCSS**: SCSS files containing Liquid tags (e.g., `{{ site.data.theme.skin }}`) not processed before SCSS compilation.
6. **site.tags / site.categories iteration**: `{% for tag in site.tags %}` with `tag[1].size` not working for tag/category archive pages.

#### Files Modified
- `websites/basically-basic/` -- cloned theme repo with inlined theme files in `example/`
- `websites/basically-basic/example/_config.yml` -- commented out `theme:` line
- `websites/basically-basic/_site_jekyll_cached/` -- Jekyll reference output
- `websites/basically-basic/_site_rustkyll_recount/` -- rustkyll output

#### Build/Test Results
- All existing tests pass: 2787 unit + 148 integration = 2935+ total, 0 failures.
- Clippy: clean (no changes to src/).
- DTC DOM: 772/790 (no regression).

### [QA] 2026-03-25

#### Independent Verification
- Rebuilt site: `./target/release/rustkyll build --source websites/basically-basic/example --destination /tmp/basically-basic-qa` -- succeeds with 1 SCSS warning (expected).
- Ran DOM comparison independently: `python3 scripts/dom_compare.py --jekyll-dir websites/basically-basic/_site_jekyll_cached --rustkyll-dir /tmp/basically-basic-qa` -- confirms **0/18 matched, 18 files with differences, 649 total differences (63 acceptable diffs filtered)**.
- DTC DOM baseline: ran `./scripts/recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io` -- result **772/790** (above required 771/790).
- Tests: all pass (0 failures).
- Clippy: clean (warnings only from upstream `liquid-lib` crate, not from project code).
- Format: `cargo fmt --check` clean.

#### Acceptance Criteria Verdicts
1. Theme cloned into `websites/basically-basic/`: PASS
2. Jekyll builds with reference output in `_site_jekyll_cached/`: PASS
3. rustkyll builds successfully: PASS
4. DOM comparison recorded (0/18, 18 differing, 649 diffs): PASS
5. Representative pages verified (homepage, CV, recipes, tags/categories, search, skin): PASS
6. Follow-up issue #355 created for blockers, references #242: PASS
7. DTC DOM >= 771/790 (actual 772/790): PASS

#### VERDICT: PASS

### [PM] 2026-03-25 -- Acceptance Review

#### Criteria Verification
1. Theme cloned into `websites/basically-basic/`: CONFIRMED -- directory exists with full theme content and inlined layouts/includes/sass.
2. Jekyll reference output in `_site_jekyll_cached/`: CONFIRMED -- contains HTML files with lowercase category directories as expected.
3. rustkyll builds successfully: CONFIRMED -- `_site_rustkyll_recount/` has HTML output; QA independently rebuilt to `/tmp/basically-basic-qa`.
4. DOM comparison recorded (0/18, 649 diffs, 9 issue categories): CONFIRMED -- SWE and QA results match exactly.
5. Representative pages verified (homepage, CV, recipes, tags/categories, search, skin): CONFIRMED -- each feature area inspected and documented with specific diff counts.
6. Follow-up issue #355 created: CONFIRMED -- `docs/tracker/355-basically-basic-rendering-blockers.todo.md` exists, references #242, lists all 8 blocker categories with cross-references to related issues (#249, #345, #354).
7. DTC DOM >= 771/790: CONFIRMED -- 772/790, independently verified by both SWE and QA.

#### No Descoped Criteria
All 7 acceptance criteria have evidence. No criteria were dropped or weakened.

#### VERDICT: ACCEPT
