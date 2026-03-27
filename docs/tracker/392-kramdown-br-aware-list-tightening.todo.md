# Issue 392: Add br-aware list tightening to kramdown parser

## Problem

The kramdown parser produces loose lists (content wrapped in `<p>`) by default.
For the markdownify pipeline where input has `<br />` tags from `newline_to_br`,
lists should be tight (no `<p>` wrapping) to match Jekyll output.

## Scope

1. Add a mode to the kramdown HTML converter that produces tight lists
2. When `<br />` tags are present in input, list items should not be wrapped in `<p>`
3. This matches the behavior needed for `newline_to_br | markdownify`

## Dependencies

- Prerequisite for #390 (kramdown parser in markdownify)
