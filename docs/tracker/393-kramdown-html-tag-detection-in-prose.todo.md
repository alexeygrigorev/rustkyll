# Issue 393: Fix kramdown HTML tag detection in prose text

## Problem

The kramdown parser treats angle brackets in prose (e.g., `<TensorFlow`, `<br />`
as inline HTML tags) more aggressively than needed. This causes 10+ page
regressions when using the kramdown parser for markdownify.

## Scope

1. Only recognize valid HTML tags, not arbitrary angle bracket content
2. `<br />` tags from `newline_to_br` should be handled correctly
3. Non-HTML angle brackets in prose should be treated as literal text

## Dependencies

- Prerequisite for #390 (kramdown parser in markdownify)
