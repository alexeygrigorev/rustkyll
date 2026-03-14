# Issue 42: Support `site.related_posts` and `site.pages`

## Problem

Complex site testing (Issue 35) revealed that some Jekyll sites access `site.related_posts` and `site.pages` in templates. These variables are not currently populated in rustkyll's site context.

- `site.related_posts` -- In Jekyll, defaults to the 10 most recent posts (or LSI-computed related posts if lsi is enabled).
- `site.pages` -- An array of all standalone Page objects (non-collection, non-post pages).

## Affected Sites

- Hyde (poole/hyde) -- uses `site.related_posts` and `site.pages`

## Requirements

- Populate `site.related_posts` in the site context (at minimum, the 10 most recent posts)
- Populate `site.pages` with standalone page objects

## Dependencies

None.
