# Issue 28: Generalized Front Matter Defaults

## Problem

Rustkyll only reads `layout` from defaults. Jekyll defaults can set any front matter key (e.g., `comments: true`, `author_profile: true`, `read_time: true`).

## Requirements

- Apply all key-value pairs from matching defaults, not just `layout`
- Defaults should be overridden by per-page front matter (front matter takes precedence)
- Match defaults by scope type and path, consistent with Jekyll behavior
- All existing tests must continue to pass

## References

- Issue #22 compatibility research, gap #10
