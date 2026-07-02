# Issue 604: flaky `test_resolve_dynamic_args_with_variable` — global `TRANSLATIONS` race

Surfaced during #601 QA (2026-07-02). This is a **pre-existing** test-only flakiness,
**unrelated to #601** (issue 601 touches wikilink pipe protection, not translations).
Tracked here per the "No Silent Descoping" rule so it is not lost.

## Problem

`src/template/translate_tag.rs::tests::test_resolve_dynamic_args_with_variable`
(line ~730) fails intermittently (~1 in 5 full-suite runs) when the test suite
runs in parallel. It is stable when run in isolation.

## Root Cause (investigated)

Translations are held in a **process-global** shared cell:

```rust
// src/template/translate_tag.rs:31
static TRANSLATIONS: RwLock<Option<TranslationStore>> = RwLock::new(None);
```

mutated via `set_translations(store)` (:45) and `clear_translations()` (:51).

Multiple tests populate/clear this single global concurrently — e.g.
`test_resolve_dynamic_args_with_variable` calls `set_translations(...)` and then
renders `{% translate menu-wallets nav en %}` end-to-end through the Liquid
engine, but other parallel tests (in `translate_tag.rs` and the
`wallet_generator` / `template_generators` code paths) call
`set_translations` / `clear_translations` in between. When another test clobbers
or clears `TRANSLATIONS` while this test is between its `set_translations` and
its Liquid render, the lookup for `menu-wallets` in the `nav` category misses
and the end-to-end assertion (Test 3, line ~762+) fails.

This is the same class of global-state test race already worked around for the
`MARKDOWNIFY_*` flags (see `frontmatter.rs` `MarkdownifyOptions` comment) and for
the #601 `PROTECT_WIKILINK_PIPES` flag (serialized via a test-local mutex + RAII
guard in `src/generator.rs`).

## Scope

Make the translations-dependent tests deterministic under parallel execution.
Do NOT change production translation behavior (the global cell is fine at
runtime — it is set once per build and only read during rendering).

## Recommended Approach

Serialize all tests that touch the global `TRANSLATIONS` cell, mirroring the
pattern already used for `PROTECT_WIKILINK_PIPES` in `src/generator.rs`:

1. Add a test-local `static TRANSLATIONS_LOCK: std::sync::Mutex<()>` in the
   `translate_tag.rs` test module (and any other module whose tests call
   `set_translations` / `clear_translations`).
2. Each such test acquires the lock (`.lock().unwrap_or_else(|e| e.into_inner())`)
   for its whole body so no two overlap.
3. Use a panic-safe RAII guard that calls `clear_translations()` on drop so a
   failing test cannot leak state into the next.

Acceptable alternative: refactor the tests to inject a `&TranslationStore`
directly (as `apply_replacements` already accepts) instead of going through the
global for the assertions that do not require the Liquid engine, and only take
the serialized-global path for the genuine end-to-end render.

## Acceptance Criteria

- [ ] `test_resolve_dynamic_args_with_variable` passes deterministically: run the
      full `./scripts/cargo-safe test` suite 10 times in a row with zero failures
      (document the run in `## Log`).
- [ ] No production code path (non-test) is changed — the fix is confined to test
      code / test harness, or an additive test-only injection API.
- [ ] All other translate/wallet/template tests still pass.
- [ ] `./scripts/cargo-safe clippy -- -D warnings` clean, `cargo fmt --check` clean.
- [ ] DTC DOM match count unchanged (test-only change; must stay at the current
      baseline — verify with
      `bash scripts/recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io`
      once the DTC source is present, otherwise note it is a test-only change with
      no production impact).

## Test Scenarios

### Reproduction
- Run the full suite repeatedly (e.g. 10x) on committed `main` and observe the
  intermittent failure of `test_resolve_dynamic_args_with_variable`. Record the
  approximate failure rate.

### Fix verification
- After serialization, run the full suite 10x with zero failures.
- Verify a deliberately-panicking test body still leaves `TRANSLATIONS` cleared
  afterward (RAII guard drop), so subsequent tests are unaffected.

## Dependencies

- None. Pure test-harness fix; independent of #600 / #601.

## Notes

- Discovered by #601 QA (2026-07-02). See #601 `## Log` [QA] entry for the
  observation (~1 of 5 runs) and root-cause note. Did not block #601 acceptance.
