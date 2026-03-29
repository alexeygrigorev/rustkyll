# Issue 511: government-github -- implement `{% octicon %}` tag SVG rendering

## Problem

The `{% octicon %}` Liquid tag (from jekyll-octicons plugin) is currently stripped to empty output. Jekyll renders it as inline `<svg>` elements. This causes 14 missing SVG elements across 5 pages on the government-github site.

Affected pages:
- 404.html (1 missing SVG -- footer GitHub logo)
- aws-govcloud.html (10 missing SVGs -- check icons, chevron, footer logo)
- fedramp-confirmation/index.html (1 missing SVG -- footer logo)
- fedramp-faq.html (1 missing SVG -- footer logo)
- fedramp/index.html (1 missing SVG -- footer logo)

## Root Cause

The `{% octicon %}` tag is registered as an unknown/passthrough tag that silently produces empty output. Jekyll's `jekyll-octicons` plugin renders SVG paths inline.

Tag syntax: `{% octicon icon-name height:N class:"classes" aria-label:label %}`

Examples from the site:
- `{% octicon mark-github height:24 class:"fill-gray-light" aria-label:github-logo %}`
- `{% octicon check height:18 class:"octicon octicon-check fill-green" aria-label:check %}`
- `{% octicon chevron-right height:18 class:"d-inline fill-blue ml-1" %}`
- `{% octicon terminal height:28 class:"fill-blue d-inline mr-2" aria-label:terminal %}`

## Solution

Implement the `{% octicon %}` tag to render inline SVGs matching the GitHub Octicons icon set. The tag should:

1. Parse the icon name and optional parameters (height, width, class, aria-label)
2. Look up the SVG path data for the named octicon
3. Render an inline `<svg>` element with the correct viewBox, dimensions, class, aria attributes, and path

The SVG path data for common octicons can be embedded as a static lookup table. The government-github site uses these icons: mark-github, check, chevron-right, terminal, server, beaker, tools, code, lock, globe, checklist, person.

## Acceptance Criteria

- [ ] `{% octicon mark-github height:24 %}` renders an `<svg>` element with the correct path data
- [ ] Height, width, class, and aria-label parameters are applied to the SVG element
- [ ] All 12 octicon names used in government-github render correct SVGs
- [ ] The 5 affected pages now pass DOM comparison (14 fewer diffs)
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes
- [ ] DTC DOM baseline must not drop below 790/790
- [ ] government-github DOM score improves from 5/21 (net improvement from this fix: +5 pages)

## Test Scenarios

### Unit: Octicon tag parsing
- Parse `{% octicon mark-github height:24 class:"fill-gray" aria-label:github %}` -- verify SVG output contains viewBox, height, class, aria-label, and path
- Parse `{% octicon check height:18 %}` -- verify SVG with correct check icon path
- Parse `{% octicon chevron-right height:18 class:"d-inline fill-blue ml-1" %}` -- verify class applied
- Unknown octicon name produces empty output (graceful degradation)

### Integration: government-github pages
- Build government-github site, verify 404.html contains `<svg>` in footer
- Build government-github site, verify aws-govcloud.html contains 10 `<svg>` elements
- Verify SVG attributes match Jekyll's cached output (viewBox, path data, class names)

## Dependencies

- None (independent of other government-github issues)

## Log

### [SWE] 2026-03-29

- **TDD step 1**: Wrote 14 unit tests for octicon rendering (mark-github, check, book exact match against Jekyll cached output; width-first vs height-first dimension ordering; unknown icon graceful degradation; preprocessing with non-octicon tags; whitespace control markers; unicode surroundings; default dimensions; all 13 government-github icons)
- **TDD step 2**: Tests initially FAILED because the Liquid tag approach could not parse `key:value` syntax -- the Liquid tokenizer splits `height:24` incorrectly
- **TDD step 3**: Switched to preprocessing approach (like gist_tag.rs). Created `src/template/octicon_tag.rs` with `preprocess_octicon_tags()` that replaces `{% octicon ... %}` with inline SVG HTML before the Liquid parser sees it. Added preprocessing call in `engine.rs` parse() and build_partials() functions.
- **TDD step 4**: All 14 octicon tests PASS. Full test suite passes (3203 tests, 0 failures).
- Clippy clean, fmt clean
- **DOM verification**:
  - DTC: 790/790 (no regression)
  - government-github: 10/21 (up from 5/21, +5 pages matching)
- **Files created**: `src/template/octicon_tag.rs`
- **Files modified**: `src/template/mod.rs` (added module), `src/template/engine.rs` (added preprocessing call in parse() and build_partials(), updated existing octicon test expectation)
