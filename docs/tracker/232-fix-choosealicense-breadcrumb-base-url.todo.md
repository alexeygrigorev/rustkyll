# Issue 232: Fix choosealicense.com breadcrumb JSON-LD base URL

## Problem

choosealicense.com's layout-generated breadcrumb JSON-LD (`itemListElement`) uses `https://github.com/pages/alexeygrigorev/rustkyll/` instead of `https://choosealicense.com/`. This accounts for approximately 140 `jsonld_value_differs` diffs.

The template uses `site.github.url` or a similar GitHub Pages variable for the breadcrumb base URL. Rustkyll is either injecting an incorrect value for `site.github.url` or the variable resolution differs from Jekyll's behavior.

## Origin

Identified as RC1 in issue 226 but excluded from scope there because it requires deeper investigation of how `site.github` variables are populated during the build.

## Acceptance Criteria

- [ ] Investigate how Jekyll populates `site.github` variables (via the jekyll-github-metadata plugin)
- [ ] Ensure `site.github.url` resolves to the correct site URL (not the GitHub Pages build URL)
- [ ] Building choosealicense.com with rustkyll produces breadcrumb JSON-LD with the correct base URL
- [ ] Existing tests continue to pass

## Dependencies

- None
