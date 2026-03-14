# Issue 80: Fix CI failure -- empty tools pages

## Problem

CI integration test `test_dtc_output_no_empty_html_files` fails because `tools/modelstore.html` and `tools/obsei.html` are 0 bytes. These are collection items in `_tools/` that have `output: true` in `_config.yml` but:

- No default layout is assigned for the tools collection
- The markdown body is empty (files contain only front matter)

Jekyll also produces empty or near-empty files for these (just a newline). The current test flags any HTML file under 100 bytes as a failure, which is too aggressive for legitimately empty collection items.

## Goal

Make the CI test pass without masking real problems. Collection items with no layout and no body content should produce output that matches Jekyll's behavior (empty or a single newline), and the test should not flag these as failures.

## Approach

The preferred approach is option 3 from the original issue: make the rendering engine output at least a newline (matching Jekyll) for collection items with no layout and empty body, AND adjust the test to distinguish between "legitimately minimal output" and "broken generation that produced 0 bytes when it should have produced real content."

Specifically:

1. **Rendering fix**: When a collection item has `output: true`, no layout, and an empty body, the rendered output should be a newline character (`\n`), matching Jekyll's behavior. This is a generic fix -- it should apply to any collection, not just tools.

2. **Test fix**: Update `test_dtc_output_no_empty_html_files` so it does not fail on files that are legitimately small. The test should distinguish between:
   - Files that should have real content (they have a layout) but are empty/tiny -- these are bugs
   - Files that have no layout and no body content -- these are expected to be near-empty

   A reasonable approach: change the test to skip files that are under some small threshold (e.g., 10 bytes) only if Jekyll also produces an equivalently small file for the same path, OR simply lower the "empty" threshold from 100 to 0 bytes (since the rendering fix ensures at least 1 byte). The simplest correct fix is to change the threshold from `< 100` to `== 0`, since a 0-byte file is always a bug (Jekyll never produces 0-byte output for `output: true` items) but a small file can be legitimate.

## Dependencies

None

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes (non-ignored tests)
- [ ] `cargo test -- --ignored test_dtc_output_no_empty_html_files` passes (the specific failing test)
- [ ] `tools/modelstore.html` and `tools/obsei.html` in the generated output are not 0 bytes -- they must contain at least a newline, matching Jekyll's behavior
- [ ] The fix is generic: any collection item with `output: true`, no layout, and an empty body produces at least a newline -- not hardcoded to specific filenames or the tools collection
- [ ] The empty-file test still catches real 0-byte files (i.e., the threshold is not simply removed or set absurdly high)
- [ ] No regressions: `cargo test -- --ignored test_dtc_output_no_raw_liquid_tags` and `test_dtc_homepage_has_expected_content` still pass
- [ ] `cargo clippy -- -D warnings` passes

## Test Scenarios

### Unit: Empty-body collection item rendering
- Create a collection item with front matter only (no body content) and `output: true`, no layout. Verify the rendered output is a newline (`\n`), not 0 bytes.
- Create a collection item with front matter and a layout but empty body. Verify it renders through the layout (not empty).
- Create a collection item with front matter, no layout, and a non-empty body. Verify the body content is rendered.

### Integration: CI empty-file detection
- Build the DTC site and verify `tools/modelstore.html` and `tools/obsei.html` exist and are not 0 bytes.
- Build the DTC site and verify the empty-file check passes (the existing `test_dtc_output_no_empty_html_files` test).
- Verify that if a file were genuinely 0 bytes (a real bug), the test would still catch it (this can be validated by code inspection of the test's threshold logic).

## Log

### [SWE] 2026-03-14
- Root cause: `generate_collection_pages_cached` in generator.rs line 698 outputs `item.html_content.clone()` for items with no layout. When body is empty, html_content is "", producing 0-byte files.
- Fix 1 (rendering): When no layout and html_content is empty, output "\n" instead of empty string. Applied to both the normal path (line 697) and the error fallback path (line 740). Fix is generic -- applies to any collection, not hardcoded to tools.
- Fix 2 (test): Changed threshold in `test_dtc_output_no_empty_html_files` from `< 100` to `== 0`. A 0-byte file is always a bug (Jekyll never produces them for output:true items), but small files (1-2 bytes) are legitimate for empty-body items.
- Tests added: 3 unit tests in generator::tests:
  - test_empty_body_no_layout_produces_newline: verifies "\n" output
  - test_nonempty_body_no_layout_produces_content: verifies body content preserved
  - test_empty_body_with_layout_renders_through_layout: verifies layout wrapping still works
- Build: 1093 tests pass, 0 fail, clippy clean, fmt clean
- Files modified: src/generator.rs, tests/integration_performance.rs

### [QA] 2026-03-14
- cargo test: all non-ignored tests pass
- cargo clippy -- -D warnings: clean
- cargo fmt --check: clean
- test_dtc_output_no_empty_html_files (ignored): PASS
- test_dtc_output_no_raw_liquid_tags (ignored): PASS
- test_dtc_homepage_has_expected_content (ignored): PASS
- Full site build verified: tools/modelstore.html and tools/obsei.html are 1 byte each (newline)
- No 0-byte HTML files in generated output
- Acceptance criteria:
  - [x] cargo build compiles: PASS
  - [x] cargo test passes (non-ignored): PASS
  - [x] test_dtc_output_no_empty_html_files passes: PASS
  - [x] tools/modelstore.html and tools/obsei.html are not 0 bytes (1 byte each): PASS
  - [x] Fix is generic (checks item.html_content.is_empty(), not hardcoded): PASS
  - [x] Empty-file test still catches 0-byte files (threshold == 0): PASS
  - [x] No regressions on other ignored tests: PASS
  - [x] clippy clean: PASS
- VERDICT: PASS

### [PM] 2026-03-14
- Reviewed code diff and QA report
- Verified independently: cargo test (all pass), cargo clippy -- -D warnings (clean)
- Verified 3 new unit tests pass: test_empty_body_no_layout_produces_newline, test_nonempty_body_no_layout_produces_content, test_empty_body_with_layout_renders_through_layout
- Acceptance criteria check:
  - [x] cargo build compiles: verified
  - [x] cargo test passes (non-ignored): verified (all pass)
  - [x] test_dtc_output_no_empty_html_files passes: verified by QA (ignored test requires DTC site)
  - [x] tools/modelstore.html and tools/obsei.html not 0 bytes: verified by QA (1 byte each)
  - [x] Fix is generic: confirmed by code review -- checks item.html_content.is_empty(), no hardcoded names
  - [x] Empty-file test still catches 0-byte files: confirmed -- threshold is == 0
  - [x] No regressions on other ignored tests: verified by QA
  - [x] clippy clean: verified
- No descoped criteria. All 8 acceptance criteria met.
- VERDICT: ACCEPT
