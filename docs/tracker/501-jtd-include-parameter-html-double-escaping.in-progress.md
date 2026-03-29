# Issue 501: Fix HTML double-escaping in Liquid include parameters

## Problem

When a Liquid `{% include %}` tag passes HTML content with escaped quotes in parameters, rustkyll double-escapes the `\"` sequences into `&quot;`, producing corrupted HTML attributes in the output.

This is the dominant rendering bug in the just-the-docs site, affecting ALL 46 failing pages (out of 47 total).

### Example

The just-the-docs default layout calls:
```liquid
{% include vendor/anchor_headings.html
   anchorBody="<svg viewBox=\"0 0 16 16\" aria-hidden=\"true\"><use xlink:href=\"#svg-link\"></use></svg>"
   anchorAttrs="aria-labelledby=\"%html_id%\""
%}
```

**Jekyll output** (correct):
```html
<a href="#navigation" class="anchor-heading" aria-labelledby="navigation">
  <svg viewBox="0 0 16 16" aria-hidden="true"><use xlink:href="#svg-link"></use></svg>
</a>
```

**Rustkyll output** (broken):
```html
<a href="#navigation" class="anchor-heading" aria-labelledby=&quot;navigation&quot;>
  <svg viewBox=&quot;0 0 16 16&quot; aria-hidden=&quot;true&quot;><use xlink:href=&quot;#svg-link&quot;></use></svg>
</a>
```

The `&quot;` in attribute values causes the HTML parser to create bogus attributes like `0=''`, `16=''`, `16&quot;=''` because the browser interprets `viewBox=&quot;0` as `viewBox="` followed by bare words `0`, `16`, `16"` as attribute names.

### Root Cause

When include parameters contain escaped quotes (`\"`), rustkyll's include parameter parser or its Liquid variable interpolation is HTML-entity-encoding the quote characters instead of treating them as literal `"` characters in the output.

## Affected Pages

All 46 failing pages in just-the-docs. Every page with headings gets corrupted anchor links. Fixing this would immediately bring 10+ pages to 0 diffs (the pages that have only these 7 SVG-related diffs).

### Pages with ONLY this bug (7 diffs each, would become MATCH):
- docs/layout/minimal/default-child/index.html
- docs/navigation/index.html
- docs/navigation/main/x/index.html
- docs/navigation/main/xs/index.html
- docs/navigation/main/xt/index.html
- docs/navigation/main/xu/index.html
- docs/navigation/parents/index.html
- docs/ui-components/index.html
- docs/utilities/index.html
- docs/utilities/responsive-modifiers/index.html

## Scope

1. Find where include parameters with escaped quotes are parsed
2. Fix the parser to treat `\"` as literal `"` in the output (not `&quot;`)
3. Ensure HTML content passed through include parameters is not re-escaped
4. This is a generic Liquid engine fix, not just-the-docs-specific

## Dependencies

None.

## Baseline

- just-the-docs: 1/47
- DTC: 790/790 (must not regress)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] Include parameters with escaped quotes produce literal `"` in output
- [ ] SVG attributes in anchor headings render correctly: `viewBox="0 0 16 16"`, `aria-hidden="true"`, `aria-labelledby="navigation"`
- [ ] just-the-docs DOM score improves from 1/47 to at least 11/47 (the 10 pages with only this bug, plus the original match)
- [ ] DTC DOM baseline remains at 790/790
- [ ] No Liquid leaks introduced

## Test Scenarios

### Unit: Include parameter parsing
- Parse include with `param="<svg viewBox=\"0 0 16 16\">"` -- verify the value contains literal `"` not `&quot;`
- Parse include with `attr="aria-labelledby=\"%html_id%\""` -- verify escaped quotes become real quotes after substitution
- Parse include with no escaped quotes -- verify no change in behavior

### Integration: just-the-docs anchor headings
- Build a minimal site with a layout that calls `{% include anchor_headings.html %}` with escaped-quote parameters
- Verify the generated HTML contains `aria-labelledby="some-id"` not `aria-labelledby=&quot;some-id&quot;`
- Verify `<svg viewBox="0 0 16 16">` renders correctly

## Log

### [SWE] 2026-03-29

- Wrote 3 tests in src/template/layout.rs (TDD - tests first):
  - test_include_param_escaped_quotes_svg_viewbox
  - test_include_param_escaped_quotes_aria_labelledby
  - test_include_param_no_escaped_quotes_unchanged
- Ran tests: FAIL as expected -- got `&quot;` in output, expected literal `"`
- Root cause: `unescape_include_params()` converts `\"` to `&quot;` for the Liquid parser, but the HTML entities are never converted back to literal quotes before being stored as include parameter values
- Fix: Added `unescape_html_entities_in_value()` in src/template/include_tag.rs that converts `&quot;` back to `"` and `&#39;` back to `'` when building include parameter values in `render_to()`
- Ran tests: PASS -- all 3 new tests pass, all 3190 existing tests pass
- Clippy: clean (no warnings)
- Fmt: clean
- DOM verification:
  - DTC: 790/790 (no regression)
  - just-the-docs: 11/47 (was 1/47, +10 pages)
- Files modified: src/template/include_tag.rs, src/template/layout.rs
