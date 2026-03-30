# Issue 540: site.tags and site.categories iteration returns empty

## Problem

Tag and category archive pages that use `{% for tag in site.tags %}` with
`tag[1].size` render as nearly-empty pages in rustkyll. On the basically-basic
example site:

- `tags/index.html`: 4.8 KB (rustkyll) vs 363 KB (Jekyll)
- `categories/index.html`: similarly truncated

Jekyll provides `site.tags` as an array of pairs: `[["tagname", [post1, post2, ...]], ...]`.
Liquid templates iterate over these pairs using `tag[0]` for the name and `tag[1]`
for the posts array.

## Root Cause

`site.tags` and `site.categories` are either not populated or not structured as the
expected array-of-pairs format that Jekyll provides. When templates iterate over
`site.tags`, they get no data, resulting in empty archive pages.

## Acceptance Criteria

- [ ] `site.tags` returns array-of-pairs `[["tagname", [post1, post2, ...]], ...]`
- [ ] `site.categories` returns array-of-pairs `[["catname", [post1, post2, ...]], ...]`
- [ ] Tag archive pages render post lists for each tag
- [ ] Category archive pages render post lists for each category
- [ ] Basically-basic tags/index.html file size is within 10% of Jekyll output
- [ ] DTC DOM baseline must not regress

## Dependencies

- Discovered in #355 (basically-basic triage)
