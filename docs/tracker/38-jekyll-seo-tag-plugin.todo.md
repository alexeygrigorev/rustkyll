# Issue 38: Support `{% seo %}` Tag (Jekyll SEO Tag Plugin)

## Problem

Cross-site testing (Issue 32) revealed that sites using the `jekyll-seo-tag` plugin fail to build because the `{% seo %}` Liquid tag is not recognized.

The `jekyll-seo-tag` plugin is one of the most widely-used Jekyll plugins. It generates:
- `<title>` tag
- `<meta name="description">` tag
- Open Graph (`og:*`) meta tags
- Twitter Card meta tags
- JSON-LD structured data

## Found In

- `alexeygrigorev/aihero` -- uses `{% seo %}` in its layout

## Requirements

- Implement a `{% seo %}` tag that generates basic SEO metadata from page front matter and site config
- At minimum, generate `<title>` and `<meta name="description">` tags
- Ideally, generate Open Graph and Twitter Card meta tags

## Dependencies

- None
