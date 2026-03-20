# Issue 283: Kramdown parser Phase 4 - Integration into rustkyll

## Problem

Once the kramdown parser passes all conformance tests, we need to integrate it into rustkyll's rendering pipeline, replacing pulldown-cmark for markdown-to-HTML conversion.

## Scope

- Wire `kramdown_parser::to_html()` into `src/frontmatter.rs` markdown rendering paths
- Remove or gate pulldown-cmark usage behind a feature flag
- Remove kramdown postprocessing hacks in `src/kramdown.rs` that the native parser handles correctly
- Ensure DTC site builds correctly with the new parser
- Run full DOM comparison

## Dependencies

Depends on Issue #282 (Phase 3) being complete.

## Acceptance Criteria

- [ ] `cargo build` compiles
- [ ] `cargo test` passes
- [ ] DTC DOM comparison improves (target: significant reduction in remaining diffs)
- [ ] All benchmark sites still build without errors
- [ ] kramdown.rs postprocessing simplified (remove hacks that native parser handles)
- [ ] No regressions on sites that currently match 100%
