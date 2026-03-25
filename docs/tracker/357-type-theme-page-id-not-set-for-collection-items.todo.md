# Issue 357: `page.id` not set for collection items in template context

## Problem

Jekyll sets `page.id` for posts and collection documents (e.g., `/2014/11/30/sample-post`), but rustkyll does not populate this field in the template context. This causes conditionals like `{% if page.id %}` to evaluate to false, skipping blocks that should render.

Discovered in the Type theme (`websites/type-theme/`), where `_includes/head.html` uses `{% if site.theme_settings.katex and page.id %}` to conditionally include KaTeX CSS and JavaScript. Because `page.id` is not set, the KaTeX `<link>` and `<script>` tags are missing from `2014/11/30/sample-post.html`.

Related to issue #244 (Type theme support).

## Impact

Any Jekyll site that uses `page.id` in conditionals or output will behave incorrectly. This affects themes that conditionally load resources per-page (KaTeX, MathJax, syntax highlighting, etc.).

## Possible Fix

Set `page.id` in the template context for posts and collection documents, matching Jekyll's format (e.g., `/2014/11/30/sample-post` for a post dated 2014-11-30 with slug `sample-post`).
