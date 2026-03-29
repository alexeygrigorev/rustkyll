# Issue 353: Hydeout `find:` Liquid filter not supported

## Problem

Rustkyll does not support the `find:` Liquid filter used in Hydeout's `_includes/back-link.html`:

```liquid
{% assign back_page = site.pages | find: "name", page.back_page %}
```

This causes the about page, tags page, edge-case page, and markup page to fail rendering.

Related to issue #241 (Hydeout theme support).

## Impact

4 standalone pages fail to render with proper layout due to this missing filter.

## Possible Fix

Implement the `find:` filter that takes a property name and value, returning the first matching item from an array.
