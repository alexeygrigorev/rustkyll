# Issue 230: Fix jekyll-docs attribute diffs

## Problem

jekyll-docs matches only 14/125 (11%). Has 742 attribute_differs, 212 extra_attribute, 106 missing_attribute, plus 44 liquid leaks. The attribute diffs are likely class attributes or data attributes that rustkyll generates differently.

## Scope

1. Build jekyll-docs site and compare against Jekyll reference
2. Investigate attribute_differs patterns (742 diffs) -- likely class or data attributes
3. Investigate extra_attribute (212) and missing_attribute (106) patterns
4. Fix liquid leaks (44) -- raw Liquid tags appearing in output
5. Fix systematic attribute generation patterns

## Acceptance Criteria

- [ ] Attribute diffs are root-caused and systematic patterns fixed
- [ ] Extra and missing attributes are investigated and resolved
- [ ] Liquid leaks (raw Liquid tags in output) are eliminated
- [ ] Match rate improves substantially from 11%
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all new and existing tests

## Log

- 2026-03-18: Created from cross-site comparison analysis.
