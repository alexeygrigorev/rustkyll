# Issue 296: DTC remaining 133 DOM diff pages

## Problem

DTC matches 657/790 (83%). 133 pages have diffs: JSON-LD content (author descriptions, transcript text), missing elements, attribute diffs.

## Acceptance Criteria

- [ ] DTC DOM match improves significantly (target: 700+/790)
- [ ] No regressions on other sites
- [ ] cargo test passes
