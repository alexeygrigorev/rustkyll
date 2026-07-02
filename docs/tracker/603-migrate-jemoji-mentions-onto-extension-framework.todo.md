# Issue 603: migrate `jemoji` / `mentions` onto the extension framework

Follow-up to #600 (extension framework + wikilinks). Explicitly deferred as
out-of-scope in #600 to avoid regression risk on the DTC DOM baseline; tracked
here.

## Problem

rustkyll's `jemoji` and `mentions` HTML post-processors are currently wired into
the generator with hardcoded `if enabled { process_x() }` branches
(`src/jemoji.rs`, `src/mentions.rs`, call sites in `src/generator.rs`). #600
introduced a proper `HtmlTransform` trait + `Registry`, but intentionally left
`jemoji`/`mentions` as-is to keep the 100% DTC DOM baseline safe.

Moving them behind the trait proves the abstraction is sufficient for the
existing transforms and removes the ad-hoc branches.

## Scope

- Reimplement `jemoji` and `mentions` as `HtmlTransform` extensions using the
  #600 framework (reuse their existing skip-tag / code-pre logic).
- Preserve their current activation semantics: they must keep working for DTC
  and every other site **exactly as today**, whether that means auto-enabling
  them (to preserve current behavior) or documenting an explicit `extensions:`
  entry — decide and document, but do NOT silently change any site's output.
- Remove the hardcoded generator branches once the transforms run via the
  registry.

## Acceptance Criteria

- [ ] `jemoji` and `mentions` run via the `Registry` / `HtmlTransform` path;
      the old hardcoded `if enabled` branches in `src/generator.rs` are removed.
- [ ] **Byte-identical output** for DTC and all existing sites — this is the
      core constraint. No emoji/mention behavior change.
- [ ] DTC DOM match count stays at **exactly 788/790** (the #600 baseline).
      Verify with `bash scripts/recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io`.
- [ ] Run `bash scripts/recount-all-dom.sh` across ALL sites and confirm no
      regression on any site.
- [ ] `./scripts/cargo-safe test` green, clippy clean, `cargo fmt --check` clean.
- [ ] TDD: test-first, log fail-then-pass.

## Test Scenarios

- Existing `jemoji`/`mentions` unit tests continue to pass (moved/adapted).
- Emoji + mention on a fixture page produce identical HTML before/after the
  migration.
- Ordering relative to other extensions is deterministic and documented.

## Dependencies

- #600 (extension framework + wikilinks) must be `.done.md` first.

## Notes

- Deferred from #600 per grooming ("Kept as-is to avoid regression risk on the
  100% DTC baseline; a later issue can move them behind the trait").
