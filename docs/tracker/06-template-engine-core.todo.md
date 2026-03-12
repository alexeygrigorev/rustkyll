# Issue 06: Template Engine Core

## Description

Implement a Liquid-subset template engine that supports the tags used by the Jekyll site: `for`, `if/elsif/else/endif`, `unless`, `assign`, `capture`, `include` (basic), and `break`. Support variable access with dot notation (`site.posts`, `page.title`, `forloop.index`).

## Dependencies

- Issue 01 (project setup)

## Scope

- `src/template/` module (engine.rs, parser.rs, renderer.rs)
- Parse Liquid `{{ }}` expressions and `{% %}` tags
- Render templates with a context (variable map)
- Support dot notation for nested access (`site.data.events`)
- Support `forloop` variables (index, first, last)
- Support `for` with `limit` parameter
- Unit tests for each tag type
- NOTE: Use the `liquid` crate if it covers enough, or implement a minimal subset
