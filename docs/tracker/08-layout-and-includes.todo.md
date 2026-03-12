# Issue 08: Layout and Includes System

## Description

Implement the Jekyll layout wrapping and includes system. Layouts wrap content (`{{ content }}`). Includes are reusable snippets (`{% include file.html param=value %}`). Layouts can reference other layouts (nesting).

## Dependencies

- Issue 06 (template engine core)
- Issue 07 (template filters)

## Scope

- Load layouts from `_layouts/` directory
- Load includes from `_includes/` directory
- Layout wrapping: render page content, then wrap in layout template
- `{% include %}` tag with named parameters (`include.param`)
- Support layout chaining (layout references another layout)
- Populate `page.*` variables from front matter
- Populate `site.*` variables (collections, data, config)
- Populate `content` variable in layouts
- Unit tests with actual layouts from the site
