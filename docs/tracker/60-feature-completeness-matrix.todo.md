# Issue 60: Feature completeness matrix

## Problem

There is no single place to see which Jekyll features rustkyll supports and which are missing. Users need to quickly assess whether rustkyll will work for their site.

## Goal

Create a comprehensive feature completeness table comparing rustkyll against Jekyll. Publish it as a standalone page and link from the README.

## Deliverables

1. `docs/jekyll-compatibility.md` -- a table listing every Jekyll feature with its status in rustkyll
2. README.md updated with a link to the compatibility page

## Feature categories to cover

- Core: config parsing, front matter, Markdown rendering, layouts, includes, static files, permalinks
- Collections: posts, custom collections, drafts, pagination
- Templates: Liquid tags, filters, variables (site, page, content, paginator)
- Data files: YAML, JSON, CSV
- Plugins: jekyll-seo-tag, jekyll-feed, jekyll-sitemap, jekyll-redirect-from, jekyll-paginate, jekyll-avatar, jekyll-mentions, jekyll-include-cache, etc.
- Assets: Sass/SCSS, CoffeeScript
- CLI: build, serve, new, doctor, clean
- Other: incremental builds, live reload, baseurl/url handling, categories/tags, related_posts

## Table format

| Feature | Jekyll | rustkyll | Notes |
|---------|--------|----------|-------|
| YAML front matter | yes | yes | |
| Sass/SCSS | yes | no | Pre-compile CSS as workaround |
| jekyll-paginate | yes | no | |

Use "yes", "partial", or "no" for status.

## Dependencies

None

## Acceptance criteria

- docs/jekyll-compatibility.md exists with a comprehensive feature table
- Every major Jekyll feature is listed (at least 40 features)
- Status is accurate (verified against the codebase, not guessed)
- README links to the compatibility page
- No code changes to src/
