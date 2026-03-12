# Issue 16: Sitemap Generation

## Description

Generate `sitemap.xml` listing all pages, posts, and collection items with their URLs and last-modified dates.

## Dependencies

- Issue 05 (collection loader)
- Issue 14 (standalone pages)

## Scope

- Generate valid XML sitemap
- Include all posts, people, books, podcast episodes, conferences, courses, tools
- Include standalone pages
- Proper URL generation using site.url
- XML escaping for URLs
- Test output is valid XML with expected URLs
