# Issue 245: TeXt multiline include and partial-template resolution

## Status: ALREADY RESOLVED

This issue was resolved as part of prior work on the include tag parser. The multiline include syntax is now handled correctly.

## Verification

- DOM comparison of text-theme: 11/11 pages match (0 differences).
- All pages listed in the original issue render correctly:
  - `index.html` -- matches Jekyll output
  - `about.html` -- matches Jekyll output
  - `archive.html` -- matches Jekyll output
  - `2018/06/01/header-image.html` -- matches Jekyll output
  - `2018/07/01/welcome.html` -- matches Jekyll output
- The multiline include in `_includes/snippets/prepend-baseurl.html` (which calls `_includes/snippets/prepend-path.html`) is resolved correctly.

## Dependencies

- Issue #238

