# Issue 256: Support dynamic include paths

## Problem

`{% include analytics/{{ var }}.html %}` — variable interpolation inside include tag path is not supported. Found in jekyll-theme-chirpy.

## Acceptance Criteria

- [ ] Include tag evaluates Liquid expressions in path string
- [ ] Tests added
- [ ] No regressions
