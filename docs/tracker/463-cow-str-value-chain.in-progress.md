# Issue 463: Use Cow<str> in Liquid value chain to reduce allocations

## Problem

The Liquid runtime clones strings frequently when passing values
through filters and template rendering. Most strings pass through
unchanged but are cloned at each step.

## Approach

Replace `String` with `Cow<'a, str>` in the Liquid value types
(vendored liquid-core). This avoids allocation when a value is
just passed through without modification.

## Expected Impact

10-15% reduction in allocation overhead. Lower priority than
#461 and #462 but compounds with them.

## Acceptance Criteria

- [ ] Measurable >10% allocation reduction (measure with DHAT or similar)
- [ ] DTC DOM stays at 790/790
- [ ] All existing tests pass

## Dependencies

- Standalone, but should be done after #461 and #462 to stack gains
