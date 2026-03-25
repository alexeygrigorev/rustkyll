# Issue 356: Hyphenated front matter keys cause Liquid evaluation errors

## Problem

When a Liquid template references a front matter variable with a hyphen in its name (e.g., `page.feature-img`), rustkyll's Liquid evaluator interprets the hyphen as a subtraction operator instead of part of the variable name. This causes the template render to fail, falling back to raw content without layout wrapping.

Jekyll treats `page.feature-img` as a hash lookup for the key `feature-img` on the `page` object.

Discovered in the Type theme (`websites/type-theme/`), where `_layouts/post.html` uses `page.feature-img` to conditionally display a feature image. The affected page (`2014/11/29/feature-images.html`) renders as fallback-only content.

Related to issue #244 (Type theme support).

## Impact

Any Jekyll site using hyphenated front matter keys will have broken template rendering for pages that reference those keys.

## Possible Fix

Update the Liquid variable name parser to allow hyphens in dot-access property names (e.g., treat `page.feature-img` as accessing key `feature-img` on `page` rather than `page.feature` minus `img`).
