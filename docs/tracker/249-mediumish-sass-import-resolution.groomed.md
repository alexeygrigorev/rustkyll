# Issue 249: Mediumish Sass import resolution

## Problem

Mediumish's `assets/css/main.scss` imports `syntax` and `starsnonscss` from the default `_sass/` directory. rustkyll may warn that it cannot resolve these imports during the build.

## Root Cause

The theme expects Sass import paths resolved via Jekyll's default `sass_dir: _sass` convention. The Mediumish `_config.yml` does not set any explicit `sass` config, so it relies entirely on the default `_sass` directory.

The `_sass/` directory contains:
- `_syntax.scss`
- `_starsnonscss.scss`
- `_stars.scss`

The `main.scss` imports `syntax` and `starsnonscss` (without underscore prefix), which is standard Sass partial resolution.

## Current State

rustkyll already handles `sass_dir` defaults (defaulting to `_sass`) and has `strip_scss_import_extensions` logic. This issue may already be resolved by the existing Sass infrastructure. Verification is needed.

## Scope

1. Verify that rustkyll correctly resolves the Mediumish Sass imports from the default `_sass` directory.
2. Confirm the generated CSS output contains the expected styles from `_syntax.scss` and `_starsnonscss.scss`.
3. Verify no Sass import warnings are emitted during the Mediumish build.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes
- [ ] Building `websites/mediumish/` with rustkyll does not emit Sass import-path warnings for `syntax` or `starsnonscss`
- [ ] The generated CSS file at the expected output location contains styles from `_syntax.scss` (e.g., syntax highlighting classes) and `_starsnonscss.scss`
- [ ] The Mediumish DOM comparison does not regress from the #239 baseline
- [ ] DTC DOM count remains at 788/790 or above

## Test Scenarios

### Unit: default sass_dir resolution
- Verify `parse_sass_config` returns `_sass` as the default `sass_dir` when no `sass` key is in `_config.yml` (already covered)
- Verify imports without underscore prefix resolve to `_`-prefixed partial files

### Integration: Mediumish stylesheet build
- Build `websites/mediumish/` with rustkyll and confirm the CSS output file is generated
- Inspect the generated CSS for syntax highlighting classes from `_syntax.scss`
- Verify no import-resolution warnings appear in the build log

## Dependencies

- Issue #239 (must be `.done.md` or `.in-progress.md`)
