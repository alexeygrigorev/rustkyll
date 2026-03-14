# Issue 47: Remove Site-Specific Hardcoding

## Problem

The codebase contains logic and values specific to datatalksclub.github.io rather than being generic Jekyll-compatible behavior. rustkyll must work as a generic Jekyll replacement for any site, not just DataTalks.Club.

## Scope

This issue covers **production code only** -- hardcoded site-specific references in non-test code. Test code that uses `datatalksclub.github.io/` as a fixture site is acceptable (it is the reference site we test against), but tests should not assert DTC-specific values when they are testing generic functionality.

### Findings from audit

**Production code issues (MUST fix):**

1. `src/main.rs:21` -- CLI about string says "A static site generator for DataTalks.Club". Should be generic (e.g., "A fast static site generator compatible with Jekyll").

2. `src/feed.rs:79` -- Generator URI is hardcoded to `https://github.com/DataTalksClub/rustkyll`. This should either be configurable, use a generic rustkyll URL, or omit the org-specific path.

3. `src/main.rs:306` -- Hardcoded `collections.get("people")` to pass to `generate_collection_pages_with_people`. The concept of a "people" collection for JSON-LD author resolution is DTC-specific. This should be generalized -- for example, by looking up which collection is configured as the author source via `_config.yml`, or by passing all collections and letting the JSON-LD code find authors dynamically.

4. `src/generator.rs` -- The function `generate_collection_pages_with_people` has "people" in its name and API design. The parameter should be renamed to something generic like `author_items` or the function should accept a configurable collection name for author lookup.

**Doc comments (SHOULD fix, low priority):**

5. `src/config.rs:74`, `src/sitemap.rs:5,16,22`, `src/collection.rs:32,48`, `src/template/filters/mod.rs:4` -- Doc comments that reference datatalks.club or DataTalks.Club as examples. Replace with generic examples like `example.com` or `myblog.com`. These are not blocking but should be cleaned up for a generic tool.

**Test code (OK to keep, minor cleanup):**

- Tests that load `datatalksclub.github.io/` as a fixture are fine -- that is the reference site.
- Tests that assert DTC-specific values (like `assert_eq!(config.name, "DataTalks.Club")`) are integration tests that verify parsing of the reference site config -- these are acceptable.
- Test function names like `test_dtc_defaults_backward_compat` are acceptable as they document intent.

## Requirements

- Remove all site-specific hardcoding from production (non-test) Rust code
- The CLI `--help` output must not mention any specific site
- The Atom feed generator URI must not be org-specific
- Author/people resolution for JSON-LD must not hardcode collection name "people"
- Doc comments should use generic examples
- All existing tests must continue to pass
- No new features -- this is a cleanup/refactor only

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo fmt -- --check` passes with no formatting issues
- [ ] `cargo clippy -- -D warnings` passes with no warnings
- [ ] `cargo test` passes (all existing tests still pass)
- [ ] `grep -rn '"A static site generator for DataTalks' src/` returns no results
- [ ] `grep -rn 'DataTalksClub/rustkyll' src/feed.rs` returns no results (generator URI is generic)
- [ ] `grep -rn '\.get("people")' src/main.rs` returns no results (no hardcoded collection name in main)
- [ ] The function `generate_collection_pages_with_people` is renamed or refactored to not reference "people" in its API
- [ ] Doc comments in production code use generic examples (e.g., `example.com`) instead of `datatalks.club`
- [ ] Running `./target/debug/rustkyll --help` shows a generic description, not mentioning any specific site
- [ ] Building the reference site (`datatalksclub.github.io/`) still produces correct output (no regression)

## Test Scenarios

### Unit: CLI description is generic
- Run the binary with `--help` and verify the about text does not mention "DataTalks.Club" or any specific organization

### Unit: Feed generator URI is generic
- Generate an Atom feed and verify the `<generator>` element does not contain org-specific URLs
- The generator URI should reference rustkyll generically (e.g., `https://github.com/rustkyll/rustkyll` or just `rustkyll` with no URI, similar to how Jekyll does it)

### Unit: Author resolution is configurable
- Create a test with a collection named something other than "people" (e.g., "authors" or "team") configured as the author source
- Verify JSON-LD author resolution works with that collection name
- Verify that the system does not assume "people" anywhere in production code paths

### Integration: Reference site still builds correctly
- Build `datatalksclub.github.io/` with the refactored code
- Verify output is identical to before the refactor (no regressions)
- This test should be `#[ignore]` (full-site generation)

### Grep audit: No site-specific strings in production code
- Run grep for `datatalks`, `DataTalks`, `alexeygrigorev`, `mlbookcamp` in `src/` excluding test modules
- Verify zero matches in non-test, non-doc-example code

## Dependencies

None.

## Notes

- The `datatalksclub.github.io/` directory is the reference Jekyll site used for testing. References to it as a test fixture path are expected and acceptable.
- Test code using DTC-specific values (like checking `config.name == "DataTalks.Club"` after parsing the reference config) is fine -- those tests verify that the reference site parses correctly.
- The key principle: production code must be generic. A user with a completely different Jekyll site must be able to use rustkyll without encountering DataTalks.Club-specific behavior.
