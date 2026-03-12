# Issue 05: Collection Loader

## Description

Load Jekyll collections (`_people/`, `_books/`, `_podcast/`, `_posts/`, `_courses/`, `_conferences/`, `_tools/`) from the filesystem. Each collection item has front matter metadata and a content body. Also load standalone pages (*.md in root).

## Dependencies

- Issue 02 (config parsing -- need collection definitions)
- Issue 03 (front matter parsing)

## Scope

- `src/collection.rs` module
- `CollectionItem` struct with: slug, front matter fields, raw content, HTML content, URL
- Load all items for a collection directory
- Generate URLs from permalink patterns (`:collection`, `:title`)
- Handle `_posts/` naming convention (`YYYY-MM-DD-title.md`)
- Load standalone `.md` pages from root directory
- Unit tests with actual content files
