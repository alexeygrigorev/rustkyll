# Issue 499: Fix asterisk emphasis inside code blocks

## Problem

`fix_literal_asterisk_emphasis()` in kramdown.rs converts `*` to `<em>`
inside `<pre>` and `<code>` blocks. E.g., SQL `COUNT(*)` becomes
`COUNT(<em>)`. Should skip code blocks entirely.

## Scope

Add `in_code` tracking (for `<code>` and `<pre>` tags) to
`fix_literal_asterisk_emphasis()`, matching the pattern already used
by `fix_literal_underscore_emphasis()`.

## Affected Sites

- mlwiki.org (+7 pages if fixed)

## IMPORTANT

Do NOT broaden partial-loose list wrapping — that regressed DTC 790→753.
Only fix the asterisk-in-code issue.

## Baseline

DTC 790/790. mlwiki 576/644. Must not regress DTC.
