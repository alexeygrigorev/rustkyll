# Complex Jekyll Site Testing Results

## Verified Results (2026-03-14)

- **Sites tested:** 8
- **Full success:** 5 (opensource-guide, government-github, wtf-html-css, academicpages, hyde)
- **Build failure:** 3 (jekyll-docs, edition-template, bitcoin-org)
- **Total pages rendered across successful sites:** 24

Previous result (before issues #37-44): 1 full success, 2 partial, 5 failures.

## Site Results Table

| # | Site | Build Status | Pages | Static Files | Time | Blocker |
|---|------|-------------|-------|-------------|------|---------|
| 1 | Jekyll Docs (docs/) | FAIL | 0 | 0 | 0.00s | `{% avatar %}` plugin tag |
| 2 | Open Source Guide | OK | 4 | 72 | 0.88s | None |
| 3 | Edition Template | FAIL | 0 | 0 | 0.00s | Config YAML null value for `baseurl` |
| 4 | Government GitHub | OK | 13 | 37 | 0.09s | None |
| 5 | WTF HTML & CSS | OK | 1 | 9 | 0.01s | None |
| 6 | Bitcoin.org | FAIL | 0 | 0 | 0.05s | `{% translate %}` custom plugin tag |
| 7 | AcademicPages | OK | 1 | 69 | 0.42s | None |
| 8 | Hyde | OK | 5 | 10 | 0.01s | None |

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

## Improvements Since Last Test

| Site | Previous Status | Current Status | Fix |
|------|----------------|----------------|-----|
| Open Source Guide | Partial (0 pages) | OK (4 pages) | `{% seo %}` (#38), hash integer indexing (#44) |
| Government GitHub | Failure | OK (13 pages) | Dynamic include paths (#41) |
| AcademicPages | Failure | OK (1 page) | Include subdirectory paths (#39) |
| Hyde | Partial (0 pages) | OK (5 pages) | `{% highlight %}` (#40), `site.related_posts`/`site.pages` (#42) |
| WTF HTML & CSS | Success (1 page) | OK (1 page) | No regression |

## Detailed Error Analysis

### 1. Jekyll Docs (jekyll/jekyll docs/)

**Error phase:** Template parsing
**Error:** `{% avatar %}` plugin tag not supported

```
Build failed: template parse error: liquid:
    {% avatar user=author size=24 %}
       ^----^
    = Unknown tag.
  with: requested=avatar
```

The previous blocker (`date_to_long_string` filter) is now resolved -- a warning is emitted but the build continues. However, the site also uses the `jekyll-avatar` plugin which provides the `{% avatar %}` tag. This is not implemented in rustkyll.

**Note:** The jekyll-docs site source is at `websites/jekyll-docs/docs/` (a subdirectory of the repository), not at the repository root.

### 2. Edition Template (CloudCannon/edition-jekyll-template)

**Error phase:** Config parsing
**Error:** YAML null value for `baseurl` field

```
Build failed: config error: failed to parse config YAML (lenient):
  failed to deserialize YAML value: invalid type: unit value, expected a string
```

The `_config.yml` has `baseurl:` with no value (YAML null). Ruby's YAML parser treats this as an empty string, but rustkyll's config deserializer expects a string type. This is a config parsing strictness issue.

### 3. Bitcoin.org (bitcoin/bitcoin.org)

**Error phase:** Template parsing
**Error:** `{% translate %}` custom tag

```
Build failed: template parse error: liquid:
    <h1>{% translate pagetitle %}</h1>
           ^-------^
    = Unknown tag.
  with: requested=translate
```

The previous blocker (duplicate YAML keys) is now resolved (#43). However, the site uses a custom `{% translate %}` tag for i18n, which is a site-specific Jekyll plugin not supported by rustkyll. This site requires extensive custom plugin support to build.

### 4. WTF HTML & CSS (mdo/wtf-html-css)

**Status:** OK -- no regression.
1 page rendered, 9 static files copied.

### 5. Open Source Guide (github/opensource.guide)

**Status:** OK -- previously failed on `{% seo %}` and hash integer indexing.
4 pages rendered, 72 static files copied.

### 6. Government GitHub (github/government.github.com)

**Status:** OK -- previously failed on dynamic include paths.
13 pages rendered, 37 static files copied.

### 7. AcademicPages (academicpages/academicpages.github.io)

**Status:** OK -- previously failed on include subdirectory paths.
1 page rendered, 69 static files copied.

### 8. Hyde (poole/hyde)

**Status:** OK -- previously failed on `{% highlight %}`, `site.related_posts`, `site.pages`.
5 pages rendered, 10 static files copied.

## New Issues Discovered

| Issue | Description | Affected Site |
|---|---|---|
| (new) | `{% avatar %}` plugin tag not supported | Jekyll Docs |
| (new) | Config YAML null values cause deserialization error (e.g., `baseurl:` with no value) | Edition Template |
| (new) | `{% translate %}` custom plugin tag not supported | Bitcoin.org |

## Summary of Remaining Blockers

The 3 failing sites all require features that go beyond standard Jekyll:

1. **Jekyll Docs**: Needs `jekyll-avatar` plugin support
2. **Edition Template**: Needs lenient config parsing for null string fields
3. **Bitcoin.org**: Needs custom `{% translate %}` plugin (site-specific i18n system)

Of these, the Edition Template config issue (#2) is the most tractable -- it just needs the config parser to treat YAML null as empty string for string fields. The other two require new plugin tag implementations.
