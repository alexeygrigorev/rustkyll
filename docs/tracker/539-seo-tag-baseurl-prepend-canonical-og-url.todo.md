# Issue 539: SEO tag baseurl prepend for canonical/og:url/JSON-LD URLs

## Problem

When a site sets `baseurl` in `_config.yml` (e.g. `baseurl: "/example"`), Jekyll's
`jekyll-seo-tag` automatically prepends `site.baseurl` to canonical URLs, `og:url`,
and JSON-LD `url` fields. Rustkyll's `src/template/seo_tag.rs` does not apply
`site.baseurl` when constructing these values.

This causes ~180 DOM differences on the basically-basic example site alone, and
will affect any site that uses a non-empty `baseurl`.

### Expected (Jekyll)

```html
<link rel="canonical" href="/example/404.html" />
<meta property="og:url" content="/example/404.html" />
```

### Actual (rustkyll)

```html
<link rel="canonical" href="/404.html" />
<meta property="og:url" content="/404.html" />
```

## Root Cause

`src/template/seo_tag.rs` constructs canonical/og:url values from `page.url` without
prepending `site.baseurl`. The JSON-LD `url`, `mainEntityOfPage.@id`, and
`publisher.logo.url` fields also miss the prefix.

## Acceptance Criteria

- [ ] Canonical URL includes `site.baseurl` prefix when set
- [ ] `og:url` includes `site.baseurl` prefix when set
- [ ] JSON-LD `url` and `mainEntityOfPage.@id` include `site.baseurl` prefix
- [ ] JSON-LD `publisher.logo.url` includes `site.baseurl` prefix
- [ ] Sites with empty/unset baseurl are unaffected
- [ ] DTC DOM baseline must not regress

## Dependencies

- Discovered in #355 (basically-basic triage)
