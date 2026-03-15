# Issue 106: Add syntax highlighting (Rouge-compatible spans)

## Problem

Jekyll uses Rouge for syntax highlighting, producing <span class="c1">, <span class="k">, etc. inside code blocks. rustkyll wraps code blocks in the correct kramdown div structure but does not generate syntax highlighting tokens.

This causes visible differences on blog posts with code blocks.

## Acceptance criteria

- Code blocks with language tags produce syntax-highlighted HTML matching Rouge output
- Blog post /blog/practical-guide-better-code.html achieves 0% pixel diff
- Consider using syntect crate for highlighting
- All existing tests pass
