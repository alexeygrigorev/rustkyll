# Issue 88: Fix missing CSS/styling on DTC site

## Problem

The DTC site looks different when built with rustkyll vs Jekyll. Likely causes include missing or incorrectly compiled SCSS, missing CSS classes in HTML output, or CSS files not being served correctly.

## Goal

The DTC site must have identical styling when built with rustkyll. Every CSS class, every stylesheet, every style rule must produce the same visual result.

## Approach

1. Compare the CSS files in _site/ between Jekyll and rustkyll builds
2. Check if all SCSS files are compiled correctly (grass crate)
3. Check if all CSS classes in HTML match Jekyll's output
4. Fix any missing or incorrect styles

## Dependencies

None

## Acceptance criteria

- All CSS files present in rustkyll _site/ match Jekyll _site/
- SCSS compilation produces equivalent CSS
- HTML elements have correct CSS classes (matching Jekyll)
- Visual appearance matches Jekyll when served in browser
- No missing stylesheets (check browser dev tools for 404s on CSS)
