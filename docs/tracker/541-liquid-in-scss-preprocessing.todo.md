# Issue 541: Liquid template preprocessing in SCSS files

## Problem

Some Jekyll themes embed Liquid template syntax inside SCSS files with front matter.
Jekyll processes these files through Liquid before SCSS compilation. Rustkyll does
not perform Liquid preprocessing on SCSS files, so the Liquid syntax passes through
to the SCSS compiler and causes compilation failures.

### Example (basically-basic `assets/stylesheets/main.scss`)

```scss
---
# Only the main Sass file needs front matter (the dashes are enough)
---

@charset "utf-8";

// Theme skin
@import "basically-basic/themes/{{ site.data.theme.skin | default: 'default' }}";

@import "basically-basic";
```

The `{{ site.data.theme.skin | default: 'default' }}` should be resolved to a
concrete value (e.g. `default`) before SCSS compilation.

## Root Cause

Rustkyll's SCSS compilation pipeline does not run Liquid rendering on SCSS files
that have front matter, even though Jekyll does. Jekyll treats any file with YAML
front matter as a Liquid template, regardless of file extension.

## Acceptance Criteria

- [ ] SCSS files with front matter are processed through Liquid before SCSS compilation
- [ ] `{{ site.data.theme.skin | default: 'default' }}` resolves correctly
- [ ] The basically-basic site produces a valid `main.css` after Liquid preprocessing
- [ ] Non-front-matter SCSS files (partials) are NOT processed through Liquid
- [ ] DTC DOM baseline must not regress

## Related Issues

- #249 (Mediumish SASS import resolution)
- #345 (al-folio SASS import resolution)
- Discovered in #355 (basically-basic triage)
