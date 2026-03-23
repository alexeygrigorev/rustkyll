# Issue 275: DTC inline emphasis double-nesting

## Problem

On 9 DTC pages, the markdown parser produces double-nested `<em><em>...</em></em>` or `<strong><strong>...</strong></strong>` where Jekyll produces a single level. Also some `<em>` vs `<strong>` confusion on adjacent `*` markers.

## Affected pages (9)

Blog posts and book review pages with complex inline emphasis patterns.

## Acceptance Criteria

- [ ] Inline emphasis renders matching Jekyll (no double nesting)
- [ ] DOM comparison improves for affected pages
- [ ] Tests verify emphasis rendering edge cases
