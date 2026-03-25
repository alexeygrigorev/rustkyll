# Issue 354: Hydeout category URL casing and pagination path

## Problem

Two URL generation issues found with the Hydeout theme:

1. **Category URL casing**: Jekyll generates lowercase category paths (e.g., `edge case/`, `markup/`) while rustkyll preserves the original case from front matter (e.g., `Edge Case/`, `Markup/`). This causes category-based post URLs to differ.

2. **Pagination path**: Jekyll generates pagination pages at root (`/page2/`, `/page3/`) per the default `paginate_path: '/page:num'` setting, while rustkyll generates them at `/blog/page2/` etc.

3. **Future date posts**: Rustkyll includes posts with future dates (e.g., year 9999), while Jekyll skips them by default. This affects the homepage post listing.

Related to issue #241 (Hydeout theme support).

## Impact

- 21 files only in Jekyll / 22 files only in rustkyll (due to case differences and pagination path)
- Homepage post listing shows wrong first post (future-dated post instead of most recent real post)
