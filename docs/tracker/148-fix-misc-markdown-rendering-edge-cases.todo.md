# Issue 148: Fix miscellaneous markdown rendering edge cases

## Problem

Various small markdown rendering differences between kramdown (Jekyll) and pulldown-cmark (rustkyll) that result in missing or extra HTML elements. ~50 instances across ~20 files.

Includes:
- Missing/extra `<figcaption>` elements (3 instances)
- Missing/extra `<blockquote>` elements (3 instances)
- Missing/extra `<img>` elements (2 instances)
- Missing/extra `<h1>`-`<h3>` elements (12 instances)
- Missing/extra `<pre>` elements (4 instances)
- Missing/extra `<strong>`/`<em>` elements (37 instances)
- Various other structural mismatches

These are edge cases where kramdown and pulldown-cmark parse markdown differently.

Discovered in issue #119 DOM diff audit.

## Acceptance criteria

- Investigate each sub-category and fix where the rustkyll output is clearly wrong
- Document any intentional differences between kramdown and pulldown-cmark
- No regressions
