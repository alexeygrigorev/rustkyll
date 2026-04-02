# Issue 541: Liquid template preprocessing in SCSS files

## Status: ALREADY RESOLVED -- Verification Only

## Problem (as originally filed)

Some Jekyll themes embed Liquid template syntax inside SCSS files with front matter.
Jekyll processes these files through Liquid before SCSS compilation. The concern was
that rustkyll does not perform Liquid preprocessing on SCSS files, causing SCSS
compilation failures.

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

## Investigation Results

**This issue is already resolved.** The Liquid preprocessing pipeline for SCSS files
works correctly in the current codebase. Here is the evidence:

1. **Code path**: SCSS files with front matter are loaded as pages. In `generator.rs`
   (line ~2263), pages without a layout are rendered via
   `render_content_only_with_cached_site()`, which checks for `{{` and `{%` markers
   and runs Liquid rendering before returning the content. The result is then passed
   to `compile_scss()` (line ~2277).

2. **basically-basic**: Builds successfully. The `{{ site.data.theme.skin | default: 'default' }}`
   resolves to `default` (from `_data/theme.yml`), the `_default.scss` theme is imported,
   and `assets/stylesheets/main.css` is generated (31 KB of valid CSS).

3. **minima**: Builds successfully. The `{{ site.minima.skin | default: 'classic' }}`
   resolves correctly and `assets/css/style.css` is generated (105 KB of valid CSS).

4. **just-the-docs**: Builds successfully. Even the complex `{% include css/just-the-docs.scss.liquid %}`
   pattern works, producing `just-the-docs-default.css` (105 KB), `just-the-docs-light.css`,
   and `just-the-docs-dark.css`.

5. **DTC docs subsite**: Also uses the just-the-docs SCSS pattern (same `{% include %}`
   approach). Not built in this verification since DTC docs is a separate subsite.

## Scope

This issue requires only verification that Liquid-in-SCSS preprocessing works for all
affected sites. No code changes are needed. The engineer should:

1. Confirm the sites listed below produce valid CSS output
2. Write a regression test to ensure Liquid-in-SCSS continues to work
3. Update issue #355 blocker #8 to mark it as resolved

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes (including new regression test)
- [ ] Regression test: parse an SCSS file with front matter containing `{{ site.data.x | default: 'fallback' }}`, verify Liquid is resolved before SCSS compilation
- [ ] basically-basic site builds and produces `assets/stylesheets/main.css` with valid CSS (no Liquid syntax remaining in output)
- [ ] minima site builds and produces `assets/css/style.css` with valid CSS
- [ ] just-the-docs site builds and produces `assets/css/just-the-docs-default.css` with valid CSS
- [ ] Issue #355 blocker #8 is updated to reflect this is resolved
- [ ] DTC DOM baseline must not regress (currently 596/790 matched)

## Test Scenarios

### Unit: Liquid-in-SCSS preprocessing

- Create an SCSS file with front matter and `{{ site.data.theme.skin | default: 'default' }}` in an `@import`
- Provide mock `site.data.theme` with `skin: "night"`
- Verify the Liquid tag resolves to `night` before SCSS compilation
- Verify the resulting CSS contains styles from the `night` skin partial

### Unit: SCSS without Liquid passes through unchanged

- Create an SCSS file with front matter but no Liquid tags
- Verify it compiles to CSS without any Liquid processing overhead

### Integration: basically-basic CSS generation

- Build the basically-basic site
- Verify `assets/stylesheets/main.css` exists and contains no `{{` or `{%` markers
- Verify the CSS contains the default theme skin styles

## Dependencies

- None (this is a verification issue, no code changes expected)

## Related Issues

- #249 (Mediumish SASS import resolution) -- resolved
- #345 (al-folio SASS import resolution) -- resolved
- #355 (basically-basic rendering blockers) -- blocker #8 should be marked resolved
- Discovered in #355 triage

## DTC DOM Baseline

- Baseline: 596/790 matched files
- Must not drop below 596

## Log

### [PM] 2026-04-02 grooming
- Investigated codebase: `render_content_only_with_cached_site()` in `layout.rs` already runs Liquid on SCSS content
- Verified basically-basic, minima, just-the-docs all produce valid CSS from Liquid-in-SCSS files
- Issue is already resolved in current code; scope reduced to verification + regression test
- DTC DOM baseline recorded: 596/790
