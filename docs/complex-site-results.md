# Complex Jekyll Site Testing Results

Tested as part of Issue #35. All sites were shallow-cloned into `websites/` and tested with `rustkyll build`.

## Summary

- **Sites tested:** 8
- **Full success:** 1 (wtf-html-css)
- **Partial success (some pages rendered):** 2 (opensource-guide, hyde)
- **Build failure:** 5 (jekyll-docs, edition-template, government-github, bitcoin-org, academicpages)
- **Total pages rendered across all sites:** 1

## Site Results Table

| # | Site | GitHub URL | Approx Pages | Build Status | Pages Rendered | Blocker |
|---|------|-----------|-------------|-------------|---------------|---------|
| 1 | Jekyll Docs | jekyll/jekyll (docs/) | ~80 | Failure | 0 | Missing `date_to_long_string` filter |
| 2 | Open Source Guide | github/opensource.guide | ~14 articles | Partial | 0 | Hash integer indexing (`locale[0]` on map), `{% seo %}` tag |
| 3 | Edition Template | CloudCannon/edition-jekyll-template | ~15 | Failure | 0 | `{% seo %}` plugin tag |
| 4 | Government GitHub | github/government.github.com | ~50 | Failure | 0 | Dynamic include `{% include {{ expr }} %}` |
| 5 | WTF HTML & CSS | mdo/wtf-html-css | ~1 page | Success | 1 | None |
| 6 | Bitcoin.org | bitcoin/bitcoin.org | ~270 | Failure | 0 | Duplicate YAML keys in `_config.yml` |
| 7 | AcademicPages | academicpages/academicpages.github.io | ~90 | Failure | 0 | Include subdirectory paths (`head/custom.html`) |
| 8 | Hyde | poole/hyde | ~5 | Partial | 0 | `{% highlight %}` tag, `site.related_posts`, `site.pages` |

## Feature Coverage Matrix

| Feature | jekyll-docs | opensource-guide | edition-template | government-github | wtf-html-css | bitcoin-org | academicpages | hyde |
|---------|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| Multiple collections | Y | Y | Y | - | - | Y | Y | - |
| Data files (`_data/`) | Y | Y | - | Y | - | Y | Y | - |
| Custom plugins (`{% seo %}`, etc.) | Y | Y | Y | - | - | - | - | - |
| Nested includes | Y | Y | - | Y | - | - | Y | Y |
| `{% highlight %}` code blocks | - | - | - | - | - | - | - | Y |
| Complex Liquid logic | Y | Y | - | Y | - | Y | Y | Y |
| Sass/SCSS | - | Y | - | - | - | - | - | - |
| i18n / localization | - | Y | - | - | - | Y | - | - |
| Category/tag pages | - | - | - | - | - | - | Y | - |
| `site.pages` / `site.related_posts` | - | - | - | - | - | - | - | Y |

The selected sites collectively exercise: multiple collections, data-driven pages, custom plugins, nested includes, complex Liquid logic, i18n, and highlight blocks -- covering 7+ of the target feature areas.

## Detailed Error Analysis

### 1. Jekyll Docs (jekyll/jekyll docs/)

**Error phase:** Template parsing
**Error:** Missing `date_to_long_string` filter

```
Unknown filter: date_to_long_string
```

The Jekyll docs site uses `date_to_long_string`, a Jekyll built-in filter that converts dates to long format (e.g., "14 March 2026"). This is tracked in Issue #37 (missing Jekyll filters).

**Notable features:** docs collection, data files (config_options, docs_nav.yml), 21 includes, multiple layouts, plugins (jekyll-avatar, jekyll-feed, jekyll-mentions, jekyll-redirect-from, jekyll-seo-tag).

### 2. Open Source Guide (github/opensource.guide)

**Error phase:** Template rendering
**Error:** Hash integer indexing on locale data

```
Unknown index: variable=locale, requested index=0,
available indexes=en, es, pcm, de, ru, ...
```

The site iterates over locale data files and tries to access hash entries by integer index (`locale[0]`). This is valid in Jekyll's Liquid but not supported in rustkyll. Additionally, the CONTRIBUTING page fails on `{% seo %}`.

**Notable features:** articles collection, i18n via data files (28 locales), Sass/SCSS, 5 includes, SEO plugin.

**New issue created:** #44 (hash integer indexing)

### 3. Edition Template (CloudCannon/edition-jekyll-template)

**Error phase:** Template parsing
**Error:** `{% seo %}` tag not recognized

```
Unknown tag: seo
```

This small documentation template relies on `jekyll-seo-tag` plugin. Tracked in Issue #38.

**Notable features:** docs collection, plugins (jekyll-sitemap, jekyll-seo-tag, jekyll-feed).

### 4. Government GitHub (github/government.github.com)

**Error phase:** Template parsing
**Error:** Dynamic include path with Liquid expression

```
{% include {{ page.form | append: '.html' }} %}
Expected Value, Range, ...
```

The site uses a dynamic include pattern where the included filename is computed from a Liquid expression. This is a different pattern from static include paths.

**Notable features:** Data files (civic_hackers.yml, governments.yml, research.yml), 4 includes, plugins (jekyll-avatar, jekyll-redirect-from, jekyll-seo-tag, jekyll-coffeescript, jekyll-sitemap).

**New issue created:** #41 (dynamic include paths)

### 5. WTF HTML & CSS (mdo/wtf-html-css)

**Error phase:** None -- built successfully
**Result:** 1 page rendered, 6 static files copied

This is a relatively simple single-page Jekyll site with a default layout. It built completely and correctly.

**Notable features:** Simple layout, includes (8), no collections, no plugins.

### 6. Bitcoin.org (bitcoin/bitcoin.org)

**Error phase:** Config parsing
**Error:** Duplicate YAML keys

```
duplicate entry with key "/en/developer-reference#getrawtransaction"
```

The site's `_config.yml` has duplicate keys (likely in redirect mappings). Ruby's YAML parser allows this (last value wins), but `serde_yaml` rejects it.

**Notable features:** alerts collection, i18n, data files, 6 includes, 5+ layouts, ~270 pages.

**New issue created:** #43 (duplicate YAML keys)

### 7. AcademicPages (academicpages/academicpages.github.io)

**Error phase:** Template parsing
**Error:** Include path with subdirectory separator

```
{% include head/custom.html %}
Expected Value, Range, ...
```

The include tag uses a path with `/` separator, which is not parsed correctly. Tracked in Issue #39.

**Notable features:** teaching collection, data files (authors.yml, navigation.yml, ui-text.yml, cv.json), 35 includes, 5+ layouts, plugins (jekyll-feed, jekyll-gist, jekyll-paginate, jekyll-sitemap, jekyll-redirect-from).

### 8. Hyde (poole/hyde)

**Error phase:** Template rendering/parsing (varies by page)
**Errors:**
1. `{% highlight %}` tag not recognized
2. `site.related_posts` not available
3. `site.pages` not available

```
Unknown tag: highlight
Unknown index: variable=site, requested index=related_posts
Unknown index: variable=site, requested index=pages
```

Multiple failure modes across different pages. The highlight tag is a built-in Jekyll tag for syntax highlighting. `site.related_posts` and `site.pages` are Jekyll-populated variables not yet in rustkyll.

**Notable features:** Posts with code blocks, sidebar navigation, related posts feature.

**New issues created:** #40 (highlight tag), #42 (site.related_posts and site.pages)

## Follow-Up Issues Created

| Issue | Title | Failure Mode | Affected Sites |
|-------|-------|-------------|----------------|
| #40 | Support `{% highlight %}` tag | Unknown tag: highlight | Hyde, So Simple Theme |
| #41 | Support dynamic include paths | `{% include {{ expr }} %}` | Government GitHub |
| #42 | Support `site.related_posts` and `site.pages` | Missing site variables | Hyde |
| #43 | Handle duplicate YAML keys in config | serde_yaml rejects duplicate keys | Bitcoin.org |
| #44 | Support integer indexing on hash values | `hash[0]` on map types | Open Source Guide |

## Pre-Existing Issues That Block Sites

| Issue | Title | Affected Sites |
|-------|-------|----------------|
| #37 | Missing Jekyll filters | Jekyll Docs (`date_to_long_string`) |
| #38 | `{% seo %}` tag plugin | Edition Template, Open Source Guide |
| #39 | Include subdirectory paths | AcademicPages |

## Update (2026-03-14)

Issues #37-42 have been implemented. Expected impact on these complex sites:

| Site | Previous Status | Expected Status | Reason |
|------|----------------|-----------------|--------|
| Jekyll Docs | Failure | OK | `date_to_long_string` filter now supported (#37) |
| Open Source Guide | Partial | Partial | `{% seo %}` fixed (#38), but still needs hash integer indexing (#44, in progress) |
| Edition Template | Failure | OK | `{% seo %}` tag now supported (#38) |
| Government GitHub | Failure | OK | Dynamic include paths now supported (#41) |
| WTF HTML & CSS | Success | OK | Already working |
| Bitcoin.org | Failure | Partial | Still needs duplicate YAML key handling (#43, in progress) |
| AcademicPages | Failure | OK | Include subdirectory paths now supported (#39) |
| Hyde | Partial | OK | `{% highlight %}` (#40), `site.related_posts` (#42), `site.pages` (#42) all now supported |

Expected result: 6 of 8 sites should fully build (up from 1 of 8). Remaining blockers:
- Open Source Guide: needs #44 (hash integer indexing, in progress)
- Bitcoin.org: needs #43 (duplicate YAML keys, in progress)
