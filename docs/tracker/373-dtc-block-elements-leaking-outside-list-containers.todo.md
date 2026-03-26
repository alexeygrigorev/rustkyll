# Issue 373: DTC block elements leaking outside list containers (Sub-problem B from #370)

## Problem

On `books/20241017-build-large-language-model-from-scratch.html` (7 diffs), `<h3>` and `<p>` elements that belong inside the comment `<ul><li>` container are rendering as siblings outside it. When `markdownify` produces block-level elements (like `<h3>` from markdown `###`), the browser or HTML serialization breaks them out of inline context, causing them to render outside their intended container.

DOM diff shows:
- `ul > li > p`: missing (expected inside `<li>`)
- `ul > li > h3`: missing (expected inside `<li>`)
- `h3`: extra element (leaked outside)
- `p`: extra element (leaked outside)
- `div`: extra element (leaked outside)

## Prior Attempt

Issue #370 attempted a fix (`renest_block_elements_after_list()`) but it caused regressions on machine-learning-zoomcamp and graph-algorithms pages (DOM dropped 780 to 779). The generic block re-nesting was too aggressive and was reverted.

## Scope

1. Fix block element nesting for book archive thread containers without regressing other pages
2. The fix must be generic (not page-specific)
3. DTC DOM baseline must not drop below 780/790

## Affected Pages

- `books/20241017-build-large-language-model-from-scratch.html` (7 diffs)

## Dependencies

- Follow-up from #370
