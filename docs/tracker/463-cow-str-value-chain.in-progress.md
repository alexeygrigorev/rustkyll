# Issue 463: Use Cow<str> in Liquid value chain to reduce allocations

## Problem

The Liquid runtime clones strings frequently when passing values
through filters and template rendering. Most strings pass through
unchanged but are cloned at each step.

## Investigation

- Re-checked vendored `liquid-core` on `2026-04-03`.
- The value chain already uses borrowed types:
  - `vendor/liquid-core/src/model/value/cow.rs` defines `ValueCow`
  - `vendor/liquid-core/src/model/scalar/mod.rs` defines `ScalarCow`
  - `KStringCow` is already the string carrier through the scalar/value path
- The concrete `into_owned()` hotspot this issue was originally aiming at in
  runtime lookup has already been addressed:
  - `vendor/liquid-core/src/runtime/stack.rs` `StackFrame::get()` now returns
    the borrowed `ValueCow` directly instead of forcing `into_owned()`
- Remaining `into_owned()` calls mostly sit behind `RefCell`-based frames or
  true owned conversions, so this issue as written no longer matches the code.

## Approach

Replace `String` with `Cow<'a, str>` in the Liquid value types
(vendored liquid-core). This avoids allocation when a value is
just passed through without modification.

## Outcome

This issue is stale in its current form. The Liquid value path already uses
borrowed Cow-based types, and the main runtime lookup clone path has already
been fixed by earlier work. Any further allocation work should start from a new
profile and target a specific remaining owned conversion rather than repeating
the broad "replace String with Cow" plan.

## Expected Impact

10-15% reduction in allocation overhead. Lower priority than
#461 and #462 but compounds with them.

## Acceptance Criteria

- [x] Investigate whether the broad Cow refactor is still needed
- [ ] If a new hotspot is found, spin a narrower follow-up issue with a measured baseline
- [ ] DTC DOM stays at 790/790 for any future follow-up change
- [ ] All existing tests pass for any future follow-up change

## Dependencies

- Standalone, but should be done after #461 and #462 to stack gains
