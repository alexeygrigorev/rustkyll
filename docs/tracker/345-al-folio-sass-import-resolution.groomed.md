# Issue 345: Fix al-folio Sass import resolution

## Problem

The al-folio demo build emits Sass warnings because rustkyll cannot resolve the theme's stylesheet imports. The main stylesheet at `assets/css/main.scss` uses modern `@use` directives with Liquid interpolation and built-in Sass modules.

## Root Cause

The al-folio `main.scss` has several characteristics that challenge the current Sass pipeline:

1. **`@use` directives instead of `@import`**: The file uses `@use "variables"`, `@use "themes"`, etc. The existing `strip_scss_import_extensions` only processes `@import`, `@use`, and `@forward` for extension stripping, but the actual resolution of `@use` through grass may behave differently from `@import`.

2. **Liquid interpolation in Sass**: Line 13 contains `$max-content-width: {{ site.max_width | default: "930px" }}` -- this Liquid template must be resolved before Sass compilation. The current pipeline does process Liquid first, but the `@use ... with (...)` syntax is a modern Sass feature that grass must support.

3. **Built-in Sass modules**: Lines 7-8 use `@use "sass:math"` and `@use "sass:string"` which require grass to support the Sass module system.

4. **Many partials**: The file imports 18+ partials from `_sass/`, including a `font-awesome/` subdirectory tree. All must be resolvable from the `_sass` load path.

The al-folio `_config.yml` sets `sass: { style: compressed }` with no custom `sass_dir` or `load_paths`, so it relies on the default `_sass` directory.

## Scope

1. Verify that grass correctly handles `@use` directives with the `_sass` load path.
2. Verify that Liquid interpolation in `@use ... with (...)` is processed before Sass compilation.
3. Verify that `@use "sass:math"` and `@use "sass:string"` are supported by the grass compiler version in use.
4. Fix any import resolution issues so that the al-folio stylesheet compiles without warnings.
5. Verify the generated CSS output contains styles from the expected partials.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes
- [ ] Building `websites/al-folio/` with rustkyll does not emit Sass import-path warnings
- [ ] The al-folio main stylesheet is generated successfully and contains CSS from the theme's partials (e.g., navbar, blog, publications, font-awesome classes)
- [ ] The `@use "sass:math"` and `@use "sass:string"` built-in modules compile without error
- [ ] Liquid interpolation in the `@use "variables" with (...)` block is correctly resolved before Sass compilation
- [ ] The al-folio DOM comparison does not regress from the #235 baseline
- [ ] DTC DOM count remains at 788/790 or above

## Test Scenarios

### Unit: @use directive handling
- Verify `strip_scss_import_extensions` correctly processes `@use "file.scss"` to `@use "file"` (already covered)
- Create a temp site with `_sass/_vars.scss` and a `main.scss` using `@use "vars"`, verify Sass compiles successfully via `compile_scss`

### Unit: Liquid-in-Sass resolution
- Create a test where Sass source contains a Liquid-resolved value (e.g., `$width: 930px;`), verify the Sass compiles after Liquid processing

### Integration: al-folio stylesheet build
- Build `websites/al-folio/` with rustkyll and confirm the CSS output file is generated without Sass warnings
- Inspect the generated CSS for expected classes from `_navbar.scss`, `_blog.scss`, and `font-awesome/fontawesome.scss`
- Verify the CSS is compressed (as configured in `_config.yml`)

## Dependencies

- Issue #235 (must be `.done.md` or `.in-progress.md`)
