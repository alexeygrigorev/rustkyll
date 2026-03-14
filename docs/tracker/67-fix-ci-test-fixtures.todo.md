# Issue 67: Fix CI — create test fixtures and make tests pass without DTC site

## Problem

CI fails with 56 test failures because tests depend on `datatalksclub.github.io/` which is gitignored and not present in CI. See GitHub Actions run failures and Copilot PR #2.

## Goal

Make all tests pass in CI without the DTC site directory present. Tests that require the real site should gracefully skip in CI.

## Reference

Copilot PR #2 (https://github.com/alexeygrigorev/rustkyll/pull/2) has a working approach:
- Creates `tests/fixtures/` with a minimal spec-compliant Jekyll site
- Redirects lib tests from real DTC site to fixtures
- Adds skip guards to integration tests that need real sites
- Review this PR, take what's useful, adapt as needed

## Approach

1. Review Copilot PR #2 diff
2. Either merge it directly (if quality is acceptable) or cherry-pick the useful parts
3. Ensure all tests pass both locally (with DTC site) and in CI (without it)
4. Integration tests that need real sites should skip gracefully, not fail

## Dependencies

None

## Acceptance criteria

- CI pipeline passes (0 test failures)
- `./scripts/cargo-safe test` passes locally
- Tests that need real sites skip gracefully in CI (not fail)
- Test fixtures are committed and minimal (not a copy of the full DTC site)
- No reduction in test coverage for code that can be tested with fixtures
- Clippy clean, fmt clean
