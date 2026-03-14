# Issue 37: Implement Missing Jekyll Filters

## Problem

Cross-site testing (Issue 32) revealed that rustkyll does not implement all Jekyll built-in Liquid filters. Sites using these filters fail to build.

Missing filters discovered:
- `normalize_whitespace` -- collapses multiple whitespace characters (spaces, tabs, newlines) into a single space

Additionally, unknown filters cause a hard build failure at parse time (the `liquid` crate rejects unrecognized filter names). Ideally, unknown filters should produce a warning and pass through the value unchanged, rather than crashing the entire build.

## Found In

- `alexeygrigorev/little-book-of-metals-ru` -- uses `normalize_whitespace`
- `alexeygrigorev/mlbookcamp-page` -- uses `erl_encode` (likely a typo for `url_encode`, but the hard failure is the problem)

## Scope

### In Scope

1. **`normalize_whitespace` filter** -- Implement as a custom filter in `src/template/filters/`, register in `TemplateEngine::builder()`. Behavior: replace all runs of whitespace (spaces, tabs, newlines, carriage returns) with a single space, and trim leading/trailing whitespace.

2. **Graceful handling of unknown filters** -- When the Liquid parser encounters an unknown filter name, the build should not crash. Instead, it should log a warning and pass through the input value unchanged. This requires either:
   - Catching parse errors related to unknown filters and retrying with a passthrough filter registered, OR
   - Pre-scanning templates for filter names and registering passthrough stubs for any that are unrecognized, OR
   - Using a wrapper around the liquid parser that intercepts unknown-filter errors

   The chosen approach should be documented in the code.

### Out of Scope

- Implementing every possible Jekyll filter (only `normalize_whitespace` is specifically needed now)
- The DataTalks.Club site itself does not use `normalize_whitespace` -- this is for cross-site compatibility

## Dependencies

- None (existing filter infrastructure from Issues 07 and 30 is already in place)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] The `normalize_whitespace` filter is implemented in `src/template/filters/normalize_whitespace.rs`
- [ ] The `normalize_whitespace` filter is registered in `TemplateEngine::builder()` in `src/template/engine.rs`
- [ ] `{{ "  hello   world\n\t foo  " | normalize_whitespace }}` renders as `"hello world foo"`
- [ ] `{{ "" | normalize_whitespace }}` renders as `""`
- [ ] `{{ "already clean" | normalize_whitespace }}` renders as `"already clean"`
- [ ] Unknown filters (e.g., `{{ "x" | erl_encode }}`) do not cause a hard build failure
- [ ] Unknown filters pass through the input value unchanged and log a warning
- [ ] `cargo test` passes with all new and existing tests

## Test Scenarios

### Unit: normalize_whitespace filter

- Input with multiple spaces between words produces single spaces
- Input with tabs and newlines replaced by single spaces
- Input with leading/trailing whitespace is trimmed
- Empty string input returns empty string
- Already-clean input passes through unchanged
- Input with only whitespace returns empty string

### Unit: Unknown filter handling

- A template using an unknown filter (e.g., `{{ value | nonexistent_filter }}`) renders successfully, outputting the input value unchanged
- A template using an unknown filter with arguments (e.g., `{{ value | fake_filter: "arg" }}`) handles gracefully
- A template combining known and unknown filters (e.g., `{{ value | upcase | nonexistent }}`) applies known filters and passes through for the unknown one

### Integration: Cross-site compatibility

- A template containing `{{ description | normalize_whitespace | truncate: 200 }}` renders correctly when `description` has messy whitespace
- Building a page that uses an unknown filter produces a warning message but still generates output HTML

## Implementation Notes

- Follow the existing pattern in `src/template/filters/` (see `xml_escape.rs` or `newline_to_br.rs` for simple filter examples)
- The filter struct should implement `liquid_core::FilterReflection` and `liquid_core::ParseFilter`
- Register in `TemplateEngine::builder()` alongside existing filters
- For unknown filter handling, consider the `liquid` crate's API -- it may be necessary to use a custom approach since `liquid` validates filters at parse time
