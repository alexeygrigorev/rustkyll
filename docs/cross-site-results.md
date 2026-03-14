# Cross-Site Build Testing Results

Tested: 2026-03-14

## Summary

**11 Jekyll sites found** across `alexeygrigorev` and `DataTalksClub` GitHub accounts.

- **7 of 11 sites build successfully** (64%)
- **4 of 11 sites fail** (36%)

## Sites Tested

### alexeygrigorev (8 sites)

| Repository | Status | Pages | Static Files | Time | Notes |
|---|---|---|---|---|---|
| alexeygrigorev.github.io | OK | 0 | 16 | 0.01s | Simple personal site, no collections |
| kids-horror-stories-ru | OK | 1344 | 2622 | 4.06s | Large site with 1343 posts |
| snippets | OK | 0 | 5 | 0.01s | Minimal site |
| data-science-interviews | OK | 0 | 24 | 0.01s | Pages not rendered (no layout specified) |
| mlwiki.org | OK | 1 | 6 | 0.00s | Minimal wiki site |
| mlbookcamp-page | FAIL | - | - | - | Unknown filter: `erl_encode` |
| aihero | FAIL | - | - | - | Unknown tag: `seo` (Jekyll SEO Tag plugin) |
| little-book-of-metals-ru | FAIL | - | - | - | Unknown filter: `normalize_whitespace` |

### DataTalksClub (3 sites)

| Repository | Status | Pages | Static Files | Time | Notes |
|---|---|---|---|---|---|
| datatalksclub.github.io | OK | 779 | 1455 | 16.75s | Primary reference site; 6 template warnings |
| courses | OK | 0 | 82 | 0.01s | Course listing site, no rendered pages |
| docs | FAIL | - | - | - | Include path with `/` not supported |

## Failure Details

### 1. Unknown filter: `erl_encode` (mlbookcamp-page)

```
Build failed: template error: template parse error: liquid: Unknown filter
  with: requested filter=erl_encode
```

The template `_layouts/article.html` uses `{{ page.title | erl_encode }}` which is likely a typo for `url_encode`. This is a valid error -- the site has a bug in its template. However, rustkyll should ideally not crash on unknown filters but skip them with a warning.

### 2. Unknown tag: `seo` (aihero)

```
Build failed: template error: template parse error: liquid:
    {% seo %}
       ^^^
    = Unknown tag.
```

The `{% seo %}` tag comes from the `jekyll-seo-tag` plugin. This is a widely-used Jekyll plugin that generates SEO metadata (Open Graph, Twitter Cards, JSON-LD). Rustkyll does not currently support Jekyll plugins.

### 3. Unknown filter: `normalize_whitespace` (little-book-of-metals-ru)

```
Build failed: template error: template parse error: liquid: Unknown filter
  with: requested filter=normalize_whitespace
```

The `normalize_whitespace` filter is a built-in Jekyll filter that collapses multiple whitespace characters into a single space. It is not implemented in rustkyll.

### 4. Include path with `/` separator (docs)

```
Build failed: template error: template parse error: liquid:
    {% include icons/icons.html %}
                    ^---
    = expected Value, Range, ...
```

Include tags with path separators (`/`) in the filename are not parsed correctly by the Liquid template engine. This is the same issue affecting 6 posts in `datatalksclub.github.io` (the `course-structured-data/*.html` includes).

## Distinct Failure Modes

1. **Missing Jekyll filters** (`normalize_whitespace`, `erl_encode`) -- filters not implemented in rustkyll
2. **Missing Jekyll plugin tags** (`{% seo %}`) -- plugin system not supported
3. **Include paths with `/`** -- template parser does not handle subdirectory includes

## Update (2026-03-14)

Issues #37-42 implemented the following fixes:
- Issue #37: Missing Jekyll filters (`normalize_whitespace`, `number_of_words`, `group_by`, `xml_escape`, `truncatewords`, `date_to_long_string`, etc.)
- Issue #38: `{% seo %}` tag (Jekyll SEO Tag plugin)
- Issue #39: Include paths with `/` subdirectory separator
- Issue #40: `{% highlight %}` tag
- Issue #41: Dynamic include paths (`{% include {{ expr }} %}`)
- Issue #42: `site.related_posts` and `site.pages`

### Expected impact on previously-failing sites

| Repository | Previous Status | Expected Status | Reason |
|---|---|---|---|
| mlbookcamp-page | FAIL | FAIL | `erl_encode` is a site-specific typo, not a standard Jekyll filter |
| aihero | FAIL | OK | `{% seo %}` tag now supported (#38) |
| little-book-of-metals-ru | FAIL | OK | `normalize_whitespace` filter now supported (#37) |
| DataTalksClub/docs | FAIL | OK | Include paths with `/` now supported (#39) |

Expected result: 10 of 11 sites should now build (91%), up from 7 of 11 (64%).

The only remaining failure (`mlbookcamp-page`) is due to a typo in the site itself (`erl_encode` instead of `url_encode`), not a rustkyll limitation.
