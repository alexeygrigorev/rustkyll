# Issue 27: site.categories and site.tags

## Problem

Posts have categories and tags in front matter, but rustkyll does not build `site.categories` or `site.tags` mappings.

## Requirements

- Build `site.categories` as a hash mapping category name → array of posts in that category
- Build `site.tags` as a hash mapping tag name → array of posts with that tag
- Extract categories from both front matter `categories` field and post path
- Expose both in the template context
- All existing tests must continue to pass

## References

- Issue #22 compatibility research, gap #8
