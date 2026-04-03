# Issue 358: Post excerpts in `paginator.posts` show raw markdown instead of rendered HTML

## Problem

When iterating over `paginator.posts` in a template, `post.excerpt` returns raw markdown text instead of rendered HTML. Jekyll converts excerpts to HTML before exposing them in the template context, so themes expect HTML content.

Discovered in the Type theme (`websites/type-theme/`), where the homepage (`index.html`) lists posts with excerpts. The rustkyll output shows raw markdown (e.g., `**bold**` instead of `<strong>bold</strong>`), causing 9 DOM differences on the homepage.

Related to issue #244 (Type theme support).

## Impact

Any Jekyll site that displays post excerpts on index or archive pages will show raw markdown instead of formatted HTML.

## Possible Fix

Render post excerpts through the markdown pipeline before injecting them into the template context for `paginator.posts` and `site.posts`.
