# Issue 11: Books Collection Pages

## Description

Generate HTML pages for `_books/` using the `book.html` layout. Each book gets a page at `/books/:title.html` with cover, title, authors, date range, description, links, and Q&A archive.

## Dependencies

- Issue 05 (collection loader)
- Issue 08 (layout and includes)

## Scope

- Render `_layouts/book.html` for each book
- Title, authors (linked to people pages), date range (start-end)
- Cover image display
- Description content
- External links section
- Q&A archive with nested replies (name, text, replies)
- `newline_to_br` and `markdownify` filters for archive text
- Subscribe CTA include
- Test with 3+ actual books
