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

## Investigation 2026-04-02

All three pages now produce **zero DOM diffs** against Jekyll output. The original 139-204 diffs per page have been resolved by subsequent improvements to kramdown/HTML-block handling.

### Current state (2026-04-02)
- `blog/ml-deployment-lambda.html` -- 0 DOM diffs (was 204)
- `blog/practical-guide-better-code.html` -- 0 DOM diffs (was 153)
- `blog/how-to-run-postgresql-and-pgadmin-with-docker.html` -- 0 DOM diffs (was 139, also confirmed zero in #349)

### Remaining textual differences (non-DOM)
Only cosmetic whitespace differences remain:
- Blank line count differences between Jekyll and rustkyll output
- HTML attribute formatting (`allowfullscreen=""` vs `allowfullscreen`)
- `language-plaintext highlighter-rouge` vs `highlighter-rouge` class names
- Minor indentation differences in blockquotes

None of these affect the rendered DOM tree or visual output.

### Recommendation
**Close this issue as already resolved.** No code changes needed. The fixes that resolved these pages were likely part of earlier kramdown and HTML-block processing improvements.
