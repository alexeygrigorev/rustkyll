# Issue 105: Fix whitespace in Liquid include output causing paragraph splits

## Priority

HIGH — affects 6 of 22 DTC pages (homepage, articles, books, podcast, events, tools). This is the #1 blocker for pixel-perfect match.

## Problem

When Liquid {% include %} output contains blank lines inside HTML block elements (li, p, div), pulldown-cmark treats blank lines as paragraph separators, splitting content into multiple <p> tags. Jekyll/kramdown does not do this — it preserves the inline flow.

This causes visible spacing differences on pages that use includes inside list items.

## Root cause

The Liquid-to-markdown-to-HTML pipeline passes include output (which is HTML) through the markdown converter. Blank lines inside the HTML get interpreted as markdown paragraph breaks.

## Acceptance criteria

- Pages 1-5, 9 from issue #93 achieve 0% pixel diff
- No extra vertical spacing from paragraph splits in include output
- Legitimate markdown paragraphs still work
- All existing tests pass
