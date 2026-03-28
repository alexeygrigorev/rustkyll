# Issue 442: Site config data serialization in Liquid templates

## Problem

Complex site config objects (author maps, navigation arrays) are
rendered as `__key_order...` dump strings instead of proper values.

## Affected Sites

- so-simple-theme (0/11) — author data in meta tags
- basically-basic (0/7) — author data, site title

## Root Cause

The `__key_order` metadata added to Objects for iteration ordering
is leaking into string serialization. When a template does
`{{ site.author }}` and author is an Object, the `__key_order` key
appears in the output.

## Scope

Ensure `__key_order` is filtered out during Object-to-string
serialization, not just during for-loop iteration.
