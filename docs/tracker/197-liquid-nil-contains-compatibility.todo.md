# Issue 197: Liquid nil-contains and deeper Liquid compatibility

## Problem

After fixing the `shift` filter, `feed_meta` tag, `github_edit_link` tag, lenient math filters, and nil array indexing in issue 196, approximately 337 opensource-guide pages and 57 DTC/docs pages still fail to render because of deeper Liquid compatibility issues.

### Root causes

1. **`nil contains "x"` returns error instead of false**: Ruby Liquid treats `nil contains "anything"` as `false`. Our liquid-lib stdlib's `contains_check` function errors with "Expected string | array | object, found nil". This affects the `jekyll-toc.html` include used by opensource-guide.

2. **DTC/docs just-the-docs theme**: Uses complex Liquid patterns (string indexing, nested variable paths) that fail with various Liquid errors. The just-the-docs theme's layouts require deep Liquid compatibility.

### Affected sites
- opensource-guide: 337 pages (all blocked by nil-contains in jekyll-toc.html)
- DTC/docs: 57 pages (just-the-docs theme complexity)

## Dependencies
- Issue 196 (fix layout not applied) -- done

## Proposed fix

The `nil contains` issue requires either:
1. Vendoring `liquid-lib` and patching `contains_check` in `if_block.rs` to return `false` for nil
2. Or creating a pre-processing pass that handles nil-contains patterns

## Acceptance criteria
- [ ] `nil contains "x"` evaluates to `false` (not error)
- [ ] opensource-guide pages render with layout applied
