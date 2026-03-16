# Issue 152: Fix kramdown paragraph wrapping cascade (~1184 diffs)

## Problem

When a structural diff exists (missing/extra <p> tag), all downstream text nodes appear shifted in the DOM comparison, creating cascade diffs. ~1184 diffs are secondary effects.

Also: Jekyll wraps content in <p> inside <li>, <figcaption>, <blockquote> where rustkyll doesn't in some cases.

## Goal

Fix the remaining paragraph wrapping mismatches. The cascade diffs will resolve automatically.

## Acceptance criteria

- Paragraph wrapping matches Jekyll for li, figcaption, blockquote
- Cascade diffs eliminated
- DOM diff count drops significantly
