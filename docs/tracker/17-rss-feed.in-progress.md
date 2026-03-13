# Issue 17: RSS Feed Generation

## Description

Generate an RSS/Atom feed for blog posts, equivalent to the `jekyll-feed` plugin output.

## Dependencies

- Issue 05 (collection loader)
- Issue 10 (blog posts)

## Scope

- Generate `feed.xml` (Atom format)
- Include latest blog posts (title, content, date, author, URL)
- Valid Atom XML
- Proper date formatting (ISO 8601)
- Site metadata (title, URL, description)
- Test output is valid Atom XML
