# Issue 57: Further performance optimization with quality verification

## Priority

HIGH -- DTC site currently builds in 6s, target is under 2s (10x faster than Jekyll's 19.4s).

## Problem

Issue #49 eliminated the O(n^2) deep-cloning bottleneck, bringing DTC from 300s to 6s. But 6s is only 3.2x faster than Jekyll, not the 10x target. The remaining bottleneck is the liquid crate's template interpreter.

## Goal

Get DTC site build time under 2s while preserving output correctness. This likely requires writing a custom Liquid template renderer (Option B from issue #49).

## Approaches

The engineer should choose the approach (or combination) that achieves the target. All are valid as long as acceptance criteria are met.

### Option A: Optimize within the liquid crate

- Profile the liquid crate's hot paths (template parsing, value cloning on stack access)
- Patch or fork the liquid crate to reduce allocations
- Pre-compile templates to avoid re-parsing

### Option B: Write a custom Liquid renderer

Replace the liquid crate with a purpose-built Liquid-compatible renderer:
- Parse templates once, compile to an efficient AST or bytecode
- Zero-copy variable resolution using references into the site context
- Avoid cloning values on stack push/pop
- Only implement the Liquid subset that Jekyll actually uses

### Option C: Parallel template rendering

- Use rayon to render pages in parallel across CPU cores
- Requires the renderer to be thread-safe (no shared mutable state)
- Could be combined with A or B for multiplicative gains

## Scope

### In scope

- Performance improvements to template rendering (the main bottleneck)
- Any approach (A, B, C, or combination) that reaches the target
- Updating `tests/integration_performance.rs` with tighter time assertions
- Maintaining all existing test coverage and output correctness

### Out of scope

- Changes to non-template phases (data loading, collection loading, static file copying) -- these are already fast
- New features or Liquid tags/filters beyond what is already implemented
- The Playwright visual comparison -- this is a separate future issue and is NOT required here
- The structural comparison script -- this is a separate future issue and is NOT required here

## Dependencies

- Issue 49 (done) -- the CachedSiteContext optimization this builds on

## Acceptance Criteria

### Performance targets

- [ ] DTC site (`websites/DataTalksClub/datatalksclub.github.io`) builds in under 2 seconds on a release build (median of 3 runs)
- [ ] kids-horror-stories-ru (`websites/alexeygrigorev/kids-horror-stories-ru`) builds in under 0.5 seconds on a release build (median of 3 runs)
- [ ] `scripts/benchmark.sh --site DataTalksClub/datatalksclub.github.io` confirms the speedup is at least 8x vs Jekyll

### Correctness -- no regressions

- [ ] `./scripts/cargo-safe test` passes (all non-ignored tests)
- [ ] `./scripts/cargo-safe test -- --ignored` passes for all existing integration_performance tests
- [ ] DTC site build produces the same number of HTML files as before the change (verify with `find _site -name '*.html' | wc -l`)
- [ ] DTC site build produces the same file tree structure as before (no missing or extra files)
- [ ] Spot-check at least 5 representative DTC HTML output files (homepage, a blog post, a course page, an event page, a person page) -- content must match pre-optimization output

### Code quality

- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] No `unwrap()` in library code (src/lib.rs, src/*.rs, src/template/*.rs)
- [ ] If the liquid crate is replaced, the new renderer handles all Liquid features exercised by existing tests (tags: for, if, unless, case, assign, capture, include, comment, raw, highlight; filters: all filters in src/template/engine.rs)

### Build hygiene

- [ ] `./scripts/cargo-safe build` compiles without errors
- [ ] `./scripts/cargo-safe build --release` compiles without errors
- [ ] No new compile-time warnings

## Test Scenarios

### Unit: Template rendering correctness (if custom renderer)

If a custom renderer is written (Option B), these unit tests are required:

- Parse and render `{{ variable }}` with string, integer, boolean, nil values
- Parse and render `{{ variable | filter }}` for each filter currently registered in `TemplateEngine::builder()`
- Parse and render `{% for item in array %}...{% endfor %}` with forloop variables (index, index0, first, last, length)
- Parse and render `{% if condition %}...{% elsif %}...{% else %}...{% endif %}`
- Parse and render `{% unless condition %}...{% endunless %}`
- Parse and render `{% case var %}{% when val %}...{% endcase %}`
- Parse and render `{% assign var = expr %}` and `{% capture var %}...{% endcapture %}`
- Parse and render `{% include file.html param=value %}` with parameter passing
- Parse and render `{% comment %}...{% endcomment %}` (output is empty)
- Parse and render `{% raw %}...{% endraw %}` (content passed through literally)
- Nested tag combinations (for inside if, include inside for, etc.)
- Whitespace control with `{%-` and `-%}` trim markers
- Error handling: undefined variable resolves to nil/empty, not a crash
- Error handling: missing include file produces a clear error, not a panic

### Unit: Performance micro-benchmarks

- Render a template with 1000 for-loop iterations over a simple array, verify it completes in under 10ms
- Render a template that accesses deeply nested object properties (site.data.people[0].name), verify no excessive cloning
- Render 100 different templates sequentially, verify total time is under 100ms

### Integration: Full-site build timing (mark with #[ignore])

- Build DTC site, assert elapsed time < 3 seconds (generous ceiling; the 2s target is for median of 3 runs via benchmark script, but a single test run can have variance)
- Build kids-horror-stories-ru site, assert elapsed time < 1 second
- Build DTC site, compare output file count against a known baseline (assert within +/-5 files to allow for minor content changes)

### Integration: Output correctness after optimization

- Build DTC site before and after optimization, diff the file trees -- no files should be missing
- Build DTC site, verify homepage HTML contains expected structural elements (title tag, nav, main content area)
- Build DTC site, verify a known blog post HTML contains the post title, date, and body content
- Build kids-horror-stories-ru, verify output file count matches pre-optimization baseline

### Regression: Existing test suite

- All tests in `tests/` must continue to pass without modification (unless the test itself tested internal APIs that changed, in which case the test must be updated to test equivalent behavior)
- If internal APIs change (e.g., TemplateEngine interface), update call sites and tests to match, but do not reduce test coverage

## Notes

- The engineer should profile before optimizing. Use `cargo flamegraph` or `perf` to confirm the bottleneck is in template rendering before choosing an approach.
- If Option B (custom renderer) is chosen, it is acceptable to implement only the Liquid subset used by the test sites, but the subset must be documented.
- The performance targets assume a machine with at least 4 cores and 16GB RAM. If the CI machine is slower, the test time assertions should use a multiplier.
- Full-site build tests must remain `#[ignore]` to keep the default test suite fast (per project convention).
