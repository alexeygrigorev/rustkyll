# Issue 447: muan-blog meta tag content with quotes (14 diffs)

## Problem
notes/2023-01-25-mm.html has 14 diffs — meta tag content with quotes
is parsed as multiple HTML attributes instead of a single value.

## Scope
Fix HTML attribute escaping in meta tag generation when content
contains quote characters.

## Baseline
DTC 790/790. muan-blog 2194/2218.
