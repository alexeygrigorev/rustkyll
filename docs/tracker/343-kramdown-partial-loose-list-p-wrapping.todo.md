# Issue 343: Kramdown partial-loose list paragraph wrapping

## Problem

Jekyll/kramdown has a "partial-loose" list behavior where individual list items followed by blank lines get `<p>` wrapping inside their `<li>`, while other items in the same list do not. This differs from CommonMark's all-or-nothing model where either all items in a list are loose (all get `<p>`) or all are tight (none get `<p>`).

Rustkyll uses pulldown-cmark (CommonMark) for markdown parsing, which does not support per-item loose/tight behavior. An attempt to implement this in issue 337 sub-issue D caused 22+ DOM regressions across blog/book/podcast pages and was reverted.

## Affected pages

- `blog/guidelines-to-get-data-engineer-job-against-odds.html` (3 DOM diffs from this issue: missing `<p>` wrapper in a list item, plus structural diffs from that)

## Origin

Descoped from issue 337 sub-issue D. The SWE attempted a marker-based approach (insert HTML comments in collapse function, then post-process to add `<p>`) but it caused widespread regressions because kramdown's partial-loose heuristics are complex and context-dependent.

## Implementation Notes

This may require a kramdown-specific post-processing pass that analyzes the original markdown structure to determine which list items should be loose, then patches the HTML output accordingly. The challenge is doing this without regressing other pages.

## Dependencies

- None
