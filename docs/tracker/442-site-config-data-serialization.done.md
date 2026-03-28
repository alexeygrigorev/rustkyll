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

## Log

### [SWE] 2026-03-28
- Wrote test_object_render_hides_key_order and test_object_source_hides_key_order (vendor/liquid-core/src/model/object/mod.rs)
- Ran tests: FAIL as expected -- `__key_order` appears in both render and source output
- Implemented fix: added `if k == "__key_order" { continue; }` in ObjectSource::fmt and ObjectRender::fmt
- Ran tests: PASS -- `__key_order` no longer appears in render or source output
- Full test suite: 3037+ tests pass, 0 fail
- Clippy on liquid-core: clean
- Fmt on liquid-core: clean
- DTC DOM: 790/790 (no regression)
- Files modified: vendor/liquid-core/src/model/object/mod.rs
