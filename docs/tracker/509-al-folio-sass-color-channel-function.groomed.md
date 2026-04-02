# Issue 509: Fix al-folio SASS compilation -- color.channel() function support

## Status: ALREADY RESOLVED (duplicate of #345)

The `color.channel()` workaround was implemented as part of issue #345 (al-folio Sass import resolution). The function `rewrite_color_channel_calls()` in `src/generator.rs` rewrites `color.channel($expr, "red", $space: rgb)` to `red($expr)` (and similarly for green/blue) before passing SCSS to the grass compiler. This is applied in both `ImportFixFs::read()` and `compile_scss()`.

### Evidence (verified 2026-04-02)

- Building al-folio produces **no** `color.channel` SCSS errors
- Generated `assets/css/main.css` is 113KB of valid CSS with resolved color values
- No `color.channel` text appears in the output CSS
- `color.adjust()` calls (used in `_variables.scss`) also compile correctly via grass's native support
- Existing tests: `test_rewrite_color_channel_to_global`, `test_rewrite_color_channel_green_blue`, `test_rewrite_color_channel_preserves_other`, `test_compile_scss_color_channel_workaround`

## Original Problem

Building al-folio with rustkyll produced a SASS compilation error:

```
Failed to compile SCSS for page main: Error: Undefined function.
   |
24 |     #{color.channel(v.$black-color, "red", v.$space: rgb)},
   |       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

The `color.channel()` function is part of Dart Sass 1.79+ (`sass:color` module). Grass 0.13.4 does not support it natively.

## Resolution

A string-based preprocessor (`rewrite_color_channel_calls`) rewrites the 6 `color.channel()` calls in `_themes.scss` to equivalent global functions (`red()`, `green()`, `blue()`), which grass does support. This covers the `$space: rgb` case used by al-folio.

Known limitation: only `$space: rgb` is handled. Other color spaces (hsl, hwb, etc.) would need additional patterns if encountered in other themes.

## Acceptance Criteria (verification only)

- [x] Building al-folio does not produce the `Undefined function: color.channel` SASS error
- [x] Generated `assets/css/main.css` contains valid CSS with color values (113KB, non-empty)
- [x] DTC DOM baseline: 790/790 pages (596 matched, 194 with differences, 255 total diffs)
- [x] `cargo build` compiles without errors
- [x] Existing tests pass: `test_rewrite_color_channel_to_global`, `test_compile_scss_color_channel_workaround`, etc.

## Test Scenarios (already covered)

### Unit: color.channel rewrite (in generator.rs)
- `test_rewrite_color_channel_to_global` -- rewrites `color.channel($c, "red", $space: rgb)` to `red($c)`
- `test_rewrite_color_channel_green_blue` -- rewrites green and blue channels
- `test_rewrite_color_channel_preserves_other` -- does not modify `color.adjust` calls

### Integration: stylesheet compilation (in generator.rs)
- `test_compile_scss_color_channel_workaround` -- compiles SCSS with `color.channel()` via grass, produces `rgba(0, 0, 0, 0.4)`

## Dependencies

- Issue #345 (al-folio Sass import resolution) -- DONE, includes this fix

## Recommendation

This issue should be closed as a duplicate of #345. No additional work is needed.
