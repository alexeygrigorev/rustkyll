# Issue 605: clean up pre-existing `clippy --all-targets` warnings (newer toolchain)

Surfaced during #604 QA/acceptance (2026-07-02). Pre-existing tech debt,
**unrelated to #604** (which was a test-only flaky-test fix). Tracked here per
the "No Silent Descoping" rule so the gap is not lost.

## Problem

The project-standard lint gate (per `CLAUDE.md`: `cargo clippy -- -D warnings`,
no `--all-targets`) is clean. However `cargo clippy --all-targets -- -D warnings`
reports **63 errors** on committed `main`, all in files untouched by recent work.
They come from a newer clippy toolchain (rust-1.95.0) flagging patterns that the
older gate did not.

Verified during #604: the count is 63 both before AND after #604 (via
`git stash`), and none of the 63 reference the files #604 modified — so #604
introduces zero new lints. These are purely pre-existing.

## Scope

Bring the codebase clean under `cargo clippy --all-targets -- -D warnings` on the
current toolchain, without changing any runtime behavior. This is a lint-only /
mechanical cleanup.

## Known offending lint categories (from #604 QA)

The 63 are a mix, including (non-exhaustive):

- `useless_vec` (~9) — e.g. `kramdown_parser/tests.rs`
- unused-import
- `unnecessary_get_then_check`
- `field_reassign_with_default`
- others in `pagination.rs`, `generator.rs`, etc.

Run `./scripts/cargo-safe clippy --all-targets -- -D warnings 2>&1` to get the
full current list before starting.

## Acceptance Criteria

- [ ] `./scripts/cargo-safe clippy --all-targets -- -D warnings` exits 0 (clean),
      except for any warnings originating in third-party dependencies
      (e.g. `liquid-lib`), which are out of scope — document any such exclusions.
- [ ] `./scripts/cargo-safe clippy -- -D warnings` remains clean (no regression to
      the project-standard gate).
- [ ] `./scripts/cargo-safe test` full suite passes (no behavior change).
- [ ] `cargo fmt --check` clean.
- [ ] Changes are mechanical lint fixes only — no functional/behavioral changes to
      production code. Note in `## Log` any change that is more than a trivial
      rewrite.
- [ ] DTC DOM match count unchanged — verify with
      `bash scripts/recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io`
      (fixes should be lint-only; most are in test code, but any production-file
      edits must not move the baseline). Record the before/after count in `## Log`.

## Test Scenarios

### Verification
- Capture the full `--all-targets` warning list before and after; confirm the
  count drops to 0 (modulo documented dependency-origin warnings).
- Confirm the full test suite still passes and DOM baseline is unchanged.

## Dependencies

- None. Independent lint-cleanup pass.

## Notes

- Discovered during #604 acceptance (2026-07-02). See #604 `## Log` [QA] and [PM]
  entries for the 63-count observation and the before/after `git stash`
  verification.

## Log

(none yet)
