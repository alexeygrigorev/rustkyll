# Issue 509: Fix al-folio SASS compilation -- color.channel() function support

## Problem

Building al-folio with rustkyll produces a SASS compilation warning:

```
Failed to compile SCSS for page main: Error: Undefined function.
   |
24 |     #{color.channel(v.$black-color, "red", v.$space: rgb)},
   |       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
./_sass/_themes.scss:24:7
```

The `color.channel()` function is part of the Dart Sass `sass:color` module (Sass 1.28+). rustkyll's SASS compiler does not support this function, so the main stylesheet fails to compile. This means al-folio pages have no CSS styling even when layouts are applied.

## Scope

1. Investigate whether rustkyll's SASS compiler (likely `grass` or `rsass`) supports the `sass:color` module and `color.channel()`.
2. If the compiler supports it, fix the integration to enable it.
3. If the compiler does not support it, evaluate workarounds:
   - Upgrade the SASS compiler dependency if a newer version supports it.
   - Pre-process or polyfill the function.
   - Document the limitation if no fix is feasible.
4. Verify that al-folio's main stylesheet compiles without errors.

## Baseline

- al-folio SASS warnings: 1 (color.channel)
- DTC DOM baseline: 790/790

## Acceptance Criteria

- [ ] Building al-folio does not produce the `Undefined function: color.channel` SASS error.
- [ ] The generated `assets/css/main.css` contains valid CSS with color values (not empty or error output).
- [ ] DTC DOM match count does not drop below 790/790.
- [ ] `cargo build` compiles without errors; `cargo clippy` clean; `cargo fmt` clean.
- [ ] If full `sass:color` support is not feasible, the limitation is documented and a workaround is in place that produces reasonable CSS output.

## Test Scenarios

### Integration: stylesheet compilation
- Build al-folio and verify `assets/css/main.css` exists and has non-trivial content (> 1KB).
- Verify no SASS compilation warnings in the build output.

### Unit: color.channel function
- Compile a minimal SCSS snippet using `@use "sass:color"; color.channel($color, "red")` and verify it produces valid CSS.

## Dependencies

- Issue #235 (al-folio site is set up)
- Issue #345 (overlaps with SASS import resolution -- this issue focuses specifically on the `color.channel` function)
