# Issue 211: Investigate smart quote differences in muan-blog

## Problem

Descoped from issue 209. The DOM comparison for muan-blog shows `text_differs` on pages where the only difference is in smart quote/apostrophe characters. For example, on `no-yc/index.html`:

- Jekyll: `doesn\u2019t` (U+2019 RIGHT SINGLE QUOTATION MARK)
- Rustkyll: `doesn\u2019t` (appears to also be U+2019, but comparison tool reports a difference)

This needs investigation to determine:
1. Whether the difference is in the actual Unicode character (e.g., U+2019 vs U+02BC MODIFIER LETTER APOSTROPHE)
2. Whether it is a comparison tool artifact
3. Whether it affects other sites besides muan-blog
4. The specific markdown input patterns that produce different output

## Impact

Low -- cosmetic text difference only. Does not affect page structure, links, or functionality.

## Dependencies

- Issue 209 (fix muan-blog systematic) should be done first to isolate remaining differences.
