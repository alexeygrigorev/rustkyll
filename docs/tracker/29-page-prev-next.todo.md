# Issue 29: page.previous and page.next

## Problem

Jekyll provides `page.previous` and `page.next` for posts, allowing prev/next navigation. Not implemented in rustkyll.

## Requirements

- Sort posts by date
- Inject `page.previous` and `page.next` into each post's template context
- Each should be a full post object (with `url`, `title`, etc.)
- First post has no `previous`, last post has no `next`
- All existing tests must continue to pass

## References

- Issue #22 compatibility research, gap #11
