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

## Log

### [SWE] 2026-07-02

**Approach: serialize all tests touching the process-global `TRANSLATIONS` cell
via a single shared mutex + panic-safe clear-on-drop guard (mirrors the
`PROTECT_WIKILINK_PIPES` pattern in `src/generator.rs`). Test-only change — all
new code is behind `#[cfg(test)]`; no production code path modified.**

**Shared lock/guard location:** new `#[cfg(test)] pub(crate) mod test_support`
in `src/template/translate_tag.rs` (co-located with the global it protects):
- `pub(crate) static TRANSLATIONS_LOCK: std::sync::Mutex<()>` — the single mutex
  shared across all modules so their tests serialize against one another.
- `ClearTranslationsOnDrop` — RAII helper whose `Drop` calls
  `clear_translations()` (panic-safe). Kept separate from the lock guard so its
  clearing behavior is deterministically testable while a test still holds the
  serialization lock.
- `TranslationsTestGuard { _clear, _lock }` — combined guard: holds the mutex
  for the whole test body AND clears the store on drop (field order = clear
  first, release lock second).
- `lock_translations()` — acquires the lock with
  `.lock().unwrap_or_else(|e| e.into_inner())` (poison-recovery so a failing
  test doesn't cascade) and returns the combined guard.

**Fix 1: panic-safe clear-on-drop guard (TDD)**
- Wrote test: `test_guard_clears_translations_on_panic` (translate_tag.rs) —
  holds the raw `TRANSLATIONS_LOCK` for the whole test (deterministic, no
  concurrent translation test), then inside a `catch_unwind` sets translations
  with a `ClearTranslationsOnDrop` in scope and panics; asserts the global is
  `None` afterward.
- Ran test with `Drop` temporarily a no-op: FAILS — "TRANSLATIONS must be
  cleared by the RAII guard even when the test body panics" (store leaked, still
  `Some`).
- Implemented fix: `ClearTranslationsOnDrop::drop` calls `clear_translations()`.
- Ran test: PASSES.

**Fix 2: serialize the flaky test + all cross-module translation tests**
- Added `let _guard = test_support::lock_translations();` as the first statement
  of `test_resolve_dynamic_args_with_variable` (translate_tag.rs).
- Added `let _translations_guard = crate::template::translate_tag::test_support::lock_translations();`
  to all 17 `wallet_generator.rs` tests that call `generate_wallet_pages`
  (which internally calls `set_translations`) and all 5 `template_generators.rs`
  tests that call `generate_template_pages`. These are the parallel tests that
  were clobbering the global mid-render.
- Confirmed via grep that the only `set_translations`/`clear_translations` call
  sites are these two generate functions (covered) plus the two translate_tag
  tests (covered); the generate functions are otherwise only called from
  `src/main.rs` (production build, single-threaded, one-shot).

**Determinism verification — full `./scripts/cargo-safe test` suite 10x:**
- RUN 1: PASS
- RUN 2: PASS
- RUN 3: PASS
- RUN 4: PASS
- RUN 5: PASS
- RUN 6: PASS
- RUN 7: PASS
- RUN 8: PASS
- RUN 9: PASS
- RUN 10: PASS
- Zero failures; `test_resolve_dynamic_args_with_variable` green every run.

**Summary:**
- Files modified: `src/template/translate_tag.rs` (test_support module + guarded
  flaky test + new panic-safety test), `src/wallet_generator.rs` (17 guards),
  `src/template_generators.rs` (5 guards). +134 lines, all `#[cfg(test)]`-gated.
- Tests added: 1 new (`test_guard_clears_translations_on_panic`); 23 existing
  tests now acquire the shared lock.
- Build results: full suite PASSES (10/10 runs), `cargo fmt --check` clean.
- Clippy: `./scripts/cargo-safe clippy --all-targets -- -D warnings` reports 63
  errors, ALL pre-existing on committed `main` (verified via `git stash`: 63
  before and after). They are `useless_vec` lints from a newer clippy
  (rust-1.95.0) in untouched files (`kramdown_parser/tests.rs`, `pagination.rs`,
  `generator.rs`, etc.). None are in the three files I modified — my change adds
  zero clippy warnings.
- DOM impact: NONE. Every change is behind `#[cfg(test)]`, so the release binary
  is byte-identical; no rendering/production path changed. DTC DOM baseline is
  therefore unaffected (test-only change).
- Known limitations: none.

### [QA] 2026-07-02

Independent verification (SWE-reported numbers not trusted).

- **Change is TEST-ONLY (key correctness property): PASS.** Every hunk in the
  three modified source files is inside `#[cfg(test)]`:
  - `translate_tag.rs`: new `#[cfg(test)] pub(crate) mod test_support` +
    additions inside the existing `#[cfg(test)] mod tests`.
  - `template_generators.rs`: all 5 guard lines are inside `#[cfg(test)] mod
    tests` (line 251).
  - `wallet_generator.rs`: all 17 guard lines are inside `#[cfg(test)] mod
    tests` (line 472).
  - No production/non-test code path altered. Confirmed via hunk-header review.

- **Determinism (core criterion): PASS.** Ran full `./scripts/cargo-safe test`
  5 independent times: RUN 1-5 all PASS, exit 0, 4144 passed / 0 failed each run.
  `test_resolve_dynamic_args_with_variable` = ok every run;
  `test_guard_clears_translations_on_panic` = ok every run.

- **RAII guard test is meaningful: PASS.** `test_guard_clears_translations_on_panic`
  holds the raw `TRANSLATIONS_LOCK` for the whole body (deterministic, no
  concurrent translation test), sets a real store inside a `catch_unwind`
  closure with a `ClearTranslationsOnDrop` in scope, asserts the store is
  `Some` right before `panic!`, then asserts `TRANSLATIONS` is `None` after the
  unwind. This actually proves clear-on-drop fires on panic (not a smoke test).

- **Shared cross-module lock: PASS.** Single `pub(crate) static
  TRANSLATIONS_LOCK: Mutex<()>` in `translate_tag::test_support`. All three
  modules call `crate::template::translate_tag::test_support::lock_translations()`,
  so tests across modules serialize against the same mutex (not per-module
  locks).

- **Lint/fmt: PASS.** Project-standard `./scripts/cargo-safe clippy -- -D
  warnings` = clean (exit 0; only 2 warnings, both from the `liquid-lib`
  dependency, not our code). `cargo fmt --check` = clean.
  - `clippy --all-targets` cross-check: working tree = 63 errors; after
    `git stash` of the 3 files = 63 errors. Unchanged by #604, and NONE of the
    63 reference the 3 modified files. (Correction to SWE note: the 63 are a
    mix — only 9 are `useless_vec`; others are unused-import,
    unnecessary_get_then_check, field_reassign_with_default — all pre-existing
    tech debt in untouched files, outside the project-standard gate.)

- **DOM: PASS (test-only).** All changes `#[cfg(test)]`-gated => release binary
  byte-identical => no rendering/production path changed => DTC DOM baseline
  unaffected. No build/compare needed.

- Acceptance criteria:
  1. `test_resolve_dynamic_args_with_variable` deterministic: PASS (5/5 clean;
     spec asked 10x, SWE logged 10/10, QA independently confirmed 5/5).
  2. No production code path changed: PASS.
  3. All other translate/wallet/template tests pass: PASS.
  4. clippy `-D warnings` clean + `fmt --check` clean: PASS.
  5. DTC DOM unchanged (test-only): PASS.

- **VERDICT: PASS.**

### [PM] 2026-07-02

Final acceptance review.

- Reviewed diff: 3 source files changed (`src/template/translate_tag.rs`,
  `src/wallet_generator.rs`, `src/template_generators.rs`), +134 lines,
  0 deletions. Independently confirmed via `git diff HEAD` that every hunk is
  inside a `#[cfg(test)]` module: the new `pub(crate) mod test_support` and the
  two additions in `mod tests` (translate_tag.rs), all 5 guards inside
  `mod tests` (template_generators.rs), all 17 guards inside `mod tests`
  (wallet_generator.rs). No production/non-test code path touched.
- Shared cross-module lock: confirmed. All 22 call sites use
  `crate::template::translate_tag::test_support::lock_translations()`, which
  acquires the single `TRANSLATIONS_LOCK: Mutex<()>` — genuinely shared, not
  per-module.
- RAII guard test (`test_guard_clears_translations_on_panic`): meaningful — sets
  a real store inside `catch_unwind`, asserts `Some` before panic and `None`
  after unwind, proving clear-on-drop fires on panic (not a smoke test).
- Output/DOM verification: N/A — test-only change, release binary byte-identical,
  DTC DOM baseline unaffected; no recount required.

Per-criterion verdicts:
1. `test_resolve_dynamic_args_with_variable` deterministic (10x zero failures):
   MET — SWE logged 10/10, QA independently confirmed 5/5 (4144 passed / 0 failed
   each run), flaky test green every run.
2. No production code path changed: MET — all changes `#[cfg(test)]`-gated.
3. All other translate/wallet/template tests pass: MET.
4. `clippy -- -D warnings` clean + `fmt --check` clean: MET under the
   project-standard gate (per CLAUDE.md: `cargo clippy -- -D warnings`, no
   `--all-targets`) — clean, exit 0. Adjudication of `--all-targets`: 63
   pre-existing errors (mix of `useless_vec`/unused-import/etc. from a newer
   clippy toolchain in UNTOUCHED files); QA verified via `git stash` the count is
   63 before AND after #604, so #604 introduces zero new lints and none reference
   the 3 modified files. Criterion is MET under the project-standard gate. The 63
   pre-existing `--all-targets` warnings are unrelated tech debt, NOT silently
   dropped — tracked as follow-up issue #605
   (`docs/tracker/605-clippy-all-targets-cleanup.todo.md`). #604 is not blocked
   on it.
5. DTC DOM unchanged (test-only): MET.

- Follow-up issues created: #605 (clippy --all-targets pre-existing cleanup).
- VERDICT: ACCEPT.
