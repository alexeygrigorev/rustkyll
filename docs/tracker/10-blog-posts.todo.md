# Issue 10: Blog Posts

## Description

Generate HTML pages for `_posts/` using the `post.html` layout. Each post gets a page at `/blog/:title.html` with title, subtitle, authors, date, content, and Article JSON-LD schema.

## Dependencies

- Issue 05 (collection loader)
- Issue 08 (layout and includes)

## Scope

- Render `_layouts/post.html` for each post
- Title (from `page.h1` or `page.title`), subtitle, date, authors
- Author links to `/people/:short.html`
- Full markdown content rendered to HTML
- `{% include youtube.html %}` support
- JSON-LD Article schema with authors, dates, images
- BreadcrumbList schema
- Test with 3+ actual posts
