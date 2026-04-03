# Issue 338: DTC complex HTML/markdown blog posts (3 pages)

## Problem

Three DTC blog posts have major structural diffs (139-204 each) due to complex mixing of inline HTML and markdown content. These are the hardest remaining DTC pages.

## Affected pages

### 1. blog/ml-deployment-lambda.html (204 diffs)
Raw markdown links not rendered as `<a>` tags, major HTML structure differences. The post mixes `<figure>` blocks, code blocks, and markdown extensively.

### 2. blog/practical-guide-better-code.html (153 diffs)
Similar to above — inline HTML mixed with markdown causing pulldown-cmark to misparse large sections.

### 3. blog/how-to-run-postgresql-and-pgadmin-with-docker.html (139 diffs)
Complex code blocks and HTML mixed with markdown. Structure differs significantly from Jekyll output.

## Root cause

These posts use patterns like:
- `<figure>` / `<figcaption>` blocks interspersed with markdown paragraphs
- Markdown links inside or adjacent to HTML blocks
- Code blocks within complex HTML structures

pulldown-cmark treats these as HTML blocks and stops processing markdown inside them, while kramdown continues markdown processing in certain contexts.

## Priority

LOW — These are the hardest 3 pages and each has 100+ diffs. Fixing them likely requires deep changes to how HTML blocks interact with markdown parsing.
