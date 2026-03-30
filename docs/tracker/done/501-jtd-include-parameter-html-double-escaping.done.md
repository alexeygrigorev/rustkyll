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

### [SWE] 2026-03-30 22:00

**Investigation: Fix already implemented**

- Examined the full rendering pipeline for include parameters with escaped quotes
- The fix is already implemented in two places:
  1. `unescape_include_params()` in `src/template/include_tag.rs:354` — preprocessing step that converts `\"` to `&quot;` so the Liquid parser can handle it
  2. `unescape_html_entities_in_value()` in `src/template/include_tag.rs:409` — rendering step that converts `&quot;` back to literal `"` after Liquid evaluation
- Existing tests cover the fix:
  - `test_unescape_include_params_double_quotes` (include_tag.rs)
  - `test_preprocess_escaped_quotes_anchor_headings` (include_tag.rs)
  - `test_include_param_escaped_quotes_svg_viewbox` (layout.rs)
  - `test_include_param_escaped_quotes_aria_labelledby` (layout.rs)
  - `test_include_param_no_escaped_quotes_unchanged` (layout.rs)
  - Plus 6 more preprocessing tests in include_tag.rs

**Verification:**
- All 3540 tests pass (2 ignored)
- Clippy clean, fmt clean
- DTC DOM: 790/790, 0 total diffs
- DTC build time: 0.654s (< 1.0s)
- just-the-docs DOM: 16/47 (exceeds acceptance criterion of 11/47)
- All 10 pages listed in issue as "pages with ONLY this bug" are MATCH:
  - docs/layout/minimal/default-child/index.html ✅
  - docs/navigation/index.html ✅
  - docs/navigation/main/x/index.html ✅
  - docs/navigation/main/xs/index.html ✅
  - docs/navigation/main/xt/index.html ✅
  - docs/navigation/main/xu/index.html ✅
  - docs/navigation/parents/index.html ✅
  - docs/ui-components/index.html ✅
  - docs/utilities/index.html ✅
  - docs/utilities/responsive-modifiers/index.html ✅

**Acceptance Criteria Status:**
- [x] `cargo build` compiles without errors
- [x] `cargo test` passes with all existing tests (3540 pass, 0 fail)
- [x] Include parameters with escaped quotes produce literal `"` in output
- [x] SVG attributes in anchor headings render correctly: `viewBox="0 0 16 16"`, `aria-hidden="true"`, `aria-labelledby="navigation"`
- [x] just-the-docs DOM score improves from 1/47 to at least 11/47 → current: **16/47**
- [x] DTC DOM baseline remains at 790/790
- [x] No Liquid leaks introduced

**Files modified:** None (fix was already in place from prior work)
**Removed stale file:** `docs/tracker/501-jtd-include-parameter-html-double-escaping.groomed.md` (duplicate of .in-progress.md)

### [QA] 2026-03-30 23:30

- Tests: 3540 passed, 0 failed, 2 ignored (3 runs; 2 runs had 1 intermittent flaky failure in unrelated link_tag tests — pre-existing, not issue 501)
- Clippy: clean (warnings only in liquid-lib, not rustkyll)
- Fmt: clean
- Acceptance criteria:
  - [x] `cargo build` compiles without errors — PASS
  - [x] `cargo test` passes — PASS (3540/3542, 2 ignored)
  - [x] Include parameters with escaped quotes produce literal `"` in output — PASS (verified `unescape_include_params` at include_tag.rs:354 and `unescape_html_entities_in_value` at include_tag.rs:409)
  - [x] SVG attributes in anchor headings render correctly — PASS
  - [x] just-the-docs DOM score >= 11/47 — PASS (16/47, exceeds criterion)
  - [x] DTC DOM baseline 790/790 — PASS (790/790, 0 diffs, no regression)
  - [x] No Liquid leaks introduced — PASS
- DTC build: 0.71s (under 1.0s limit)
- JTD DOM: 16/47 matched, 31 with diffs (baseline was 1/47, improvement confirmed)
- DTC DOM: 790/790, 0 total diffs, no regression
- VERDICT: PASS

### [PM] 2026-03-30 23:45
- Reviewed diff: 0 files changed (fix was already in place from prior work)
- Code verified: `unescape_include_params()` at include_tag.rs:354 and `unescape_html_entities_in_value()` at include_tag.rs:409 — clean two-phase approach (preprocess `\"` → `&quot;` for Liquid parser, then post-process back to literal `"` after evaluation)
- Output verification: built JTD and DTC independently, spot-checked docs/navigation/index.html — `viewBox="0 0 24 24"` renders with literal quotes, 0 `&quot;` instances in output
- Results verified:
  - Tests: 3540 passed, 0 failed, 2 ignored — PASS
  - Clippy: clean (warnings only in liquid-lib, not rustkyll) — PASS
  - JTD DOM: 16/47 (exceeds acceptance criterion of ≥11/47, up from baseline 1/47) — PASS
  - DTC DOM: 790/790, 0 diffs (no regression from baseline 790/790) — PASS
  - All 10 pages listed as "pages with ONLY this bug" confirmed MATCH
- Acceptance criteria: all 7 met
- Follow-up issues created: none
- VERDICT: ACCEPT
