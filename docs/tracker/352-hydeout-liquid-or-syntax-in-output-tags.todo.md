# Issue 352: Hydeout Liquid `or` syntax in output tags

## Problem

Rustkyll's Liquid parser fails on `{{ page.guid or page.id }}` found in Hydeout's `_includes/disqus.html`. While this is technically invalid Liquid (it's JavaScript that happens to be inside a Liquid output tag), Jekyll handles it gracefully because the enclosing `{% if site.disqus.shortname %}` evaluates to false, so the block is never rendered.

Rustkyll eagerly parses template content inside false conditional branches, causing a parse error that prevents the entire post from rendering properly.

Related to issue #241 (Hydeout theme support).

## Impact

All 24 Hydeout posts fail to render with proper layout because of this parse error in `disqus.html`.

## Possible Fix

Either:
1. Skip parsing of template content inside false conditional branches (lazy evaluation)
2. Treat unparseable `{{ }}` tags as raw text output instead of hard errors
3. Add `or` as a supported operator in output tags (matching Jinja2 behavior)
