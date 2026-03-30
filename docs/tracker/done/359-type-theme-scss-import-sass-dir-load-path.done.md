# Issue 359: SCSS `@import` with `sass_dir` load path not supported by grass compiler

## Status: ALREADY RESOLVED

This issue was resolved in prior work. The `compile_scss` function in `src/generator.rs` already:

1. Parses `sass_dir` from the site config (defaulting to `_sass`)
2. Passes the resolved `sass_dir` path as a `load_path` to the grass compiler options
3. Also handles explicit `load_paths` from config

## Verification

- The Type theme (`websites/type-theme/`) builds successfully with rustkyll.
- `assets/css/main.css` is generated with 8613 bytes of compiled CSS (normalize.css + theme styles).
- Existing unit tests `test_compile_scss_with_sass_dir` and `test_compile_scss_with_load_paths` cover this behavior.
- The `ImportFixFs` custom filesystem also strips `.scss`/`.sass` extensions from `@import` statements to work around a grass resolution edge case.

## Dependencies

- Issue #244 (Type theme support, already `.done.md`)
