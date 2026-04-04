# Issue 568: Hydeout front matter excerpt not displayed on paginated pages

## Problem

When a post defines `excerpt:` in its front matter, Jekyll uses that string as `post.excerpt`. Rustkyll instead overwrites the front matter excerpt with the auto-generated one from content, causing the wrong text to appear on paginated listing pages.

### Concrete example

In `websites/hydeout/_posts/2012-02-04-layout-excerpt-defined.md`:
```yaml
excerpt: "This is a user-defined post excerpt. It should be displayed in place of the auto-generated excerpt or post content on index pages."
```

The hydeout index layout uses:
```liquid
{% if post.excerpt %}
  {{ post.excerpt }}
{% else %}
  {{ post.content }}
{% endif %}
```

**Jekyll output (page2/index.html):** Shows `This is a user-defined post excerpt...` as plain text.

**Rustkyll output (page2/index.html):** Shows `<p>This is the start of the post content.</p>` -- the auto-generated excerpt from the first paragraph of content, ignoring the front matter value entirely.

## Root Cause

In `src/pagination.rs`, `collection_item_to_liquid_full()`:
1. Line 145-147: Front matter is copied to the Liquid object, including `excerpt: "user-defined text"`
2. Line 178-179: If `item.excerpt_html` is `Some(...)`, it **overwrites** the front matter excerpt with the auto-generated HTML excerpt

The auto-generated `excerpt_html` is always populated (from the first paragraph of content), so it always overwrites the front matter value. The same issue exists in `src/generator.rs` line 956-963 but is guarded by `if !item.front_matter.contains_key("excerpt")` -- the pagination code lacks this guard.

Similarly, `src/plugin_generators.rs` line 303 has the same pattern.

## Scope

- Fix `collection_item_to_liquid_full()` in `src/pagination.rs` to respect front matter `excerpt` -- only override with auto-generated excerpt when front matter does NOT contain `excerpt`
- Verify the same guard exists in `src/plugin_generators.rs`
- The front matter excerpt should be passed through as-is (plain text), matching Jekyll behavior

## Affected Sites

- hydeout: page2/index.html shows wrong excerpt (6 diffs on this page)
- Any site using front matter `excerpt:` with paginated listing pages

## Baseline

- DTC: 789/790 matched (163 total diffs). Must not regress.
- Hydeout: 23/34 matched (458 total diffs). Must improve.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new ones
- [ ] When a post has `excerpt:` in front matter, `post.excerpt` in paginator context returns the front matter value (not auto-generated)
- [ ] When a post does NOT have `excerpt:` in front matter, auto-generated excerpt still works
- [ ] Hydeout page2/index.html shows "This is a user-defined post excerpt..." text
- [ ] DTC DOM match count does not drop below 789/790
- [ ] Hydeout DOM match count improves (currently 23/34)

## Test Scenarios

### Unit: Front matter excerpt precedence in pagination
- Create a CollectionItem with front_matter containing `excerpt: "custom"` AND excerpt_html set to auto-generated HTML
- Call `collection_item_to_liquid_full()` and verify `post.excerpt` equals the front matter value, not the auto-generated one
- Create a CollectionItem without front_matter excerpt, verify auto-generated excerpt is used

### Integration: Hydeout paginated excerpt rendering
- Build hydeout site, verify page2/index.html contains "user-defined post excerpt" text
- Verify page1 (index.html) still renders excerpts correctly for posts without front matter excerpt

## Dependencies

None.
