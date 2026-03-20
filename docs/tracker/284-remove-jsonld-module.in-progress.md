# Issue 284: Remove dead jsonld.rs module

## Problem

`src/jsonld.rs` is dead code. The `inject_jsonld()` function is a no-op that returns HTML unchanged. All helper functions are `#[cfg(test)]` only. The module was originally written to inject DTC-specific JSON-LD (Book schema, breadcrumbs, author resolution) as a post-processing step, but investigation showed Jekyll handles JSON-LD via layout templates, not post-processing.

The module also violates the "no site-specific hardcoding" rule — it has DTC-specific concepts (books, people collection, "/books.html" breadcrumb).

## Fix

Delete `src/jsonld.rs` entirely and remove it from `src/lib.rs`. Remove any `inject_jsonld` calls from the rendering pipeline (likely in `src/layout.rs` or `src/generator.rs`).

## Acceptance Criteria

- [ ] `src/jsonld.rs` deleted
- [ ] All references to `inject_jsonld` removed
- [ ] `cargo build` compiles
- [ ] `cargo test` passes with no regressions
- [ ] No site-specific JSON-LD code remains

## Log

### [SWE] 2026-03-20
- Found that `src/jsonld.rs` was already deleted and `mod jsonld` already removed from `src/lib.rs`
- However, `tests/integration_jsonld.rs` still referenced `rustkyll::jsonld::inject_jsonld` on line 122, causing compilation failure
- Removed the `inject_jsonld` call from `render_item()` function -- since it was a no-op returning HTML unchanged, the function now returns the layout render result directly
- Build: compiles successfully
- Tests: 58 jsonld-related tests pass (0 fail). 90 pre-existing failures in kramdown_parser (unrelated)
- Clippy/fmt: clean for changed file; pre-existing warnings in kramdown_parser/vendor (unrelated)
- Files modified: `tests/integration_jsonld.rs` (removed dead `rustkyll::jsonld::inject_jsonld` call)
