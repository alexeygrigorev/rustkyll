# Issue 143: Fix URL percent-encoding for spaces in image/thumbnail URLs

## Problem

Jekyll percent-encodes spaces in URLs as `%20`, but rustkyll leaves them as literal spaces. This affects `thumbnailUrl` and `image` fields in JSON-LD for podcast pages with spaces in filenames.

Example:
- Jekyll: `hybrid%20search.jpg`
- Rustkyll: `hybrid search.jpg`

4 instances in 1 file.

Discovered in issue #119 DOM diff audit.

## Acceptance criteria

- URLs with spaces are percent-encoded as `%20`
- No regressions in URL generation

## Log

### [SWE] 2026-03-16
- Root cause: `relative_url` and `absolute_url` Liquid filters did not percent-encode spaces in URL paths. Jekyll does this automatically.
- The 4 affected instances are in the podcast layout template where `{{ page.image | relative_url }}` is used for `image` and `thumbnailUrl` fields in JSON-LD. The one podcast episode with a space in its image filename (`hybrid search.jpg`) had unencoded spaces.
- Fix: Added `encode_url_spaces()` helper that replaces spaces with `%20`, called at the end of both `relative_url` and `absolute_url` filter evaluation.
- TDD approach: wrote 3 failing tests first, then implemented the fix, tests pass.
- Tests added: 4 unit tests (3 in relative_url, 1 in absolute_url)
- Build: all tests pass, clippy clean, fmt clean
- Files modified:
  - `src/template/filters/relative_url.rs` -- added `encode_url_spaces()` and applied it; added 3 tests
  - `src/template/filters/absolute_url.rs` -- applied `encode_url_spaces()`; added 1 test
  - `src/template/filters/mod.rs` -- changed `relative_url` module visibility to `pub(crate)` so `absolute_url` can use the shared helper
