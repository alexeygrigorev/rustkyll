# Issue 533: SEO tag incorrectly handles multi-author posts

## Problem

When a post has multiple authors (as a YAML array), rustkyll emits a `<meta name="author">`
tag with the author names concatenated without any separator. Jekyll's `jekyll-seo-tag`
does NOT emit `<meta name="author">` at all for multi-author posts -- it only emits
the author meta tag for single-author posts.

Additionally, the JSON-LD `author` field incorrectly uses the concatenated string
instead of omitting the author or listing them individually.

### Affected page

`junk/2016/05/20/this-post-demonstrates-post-content-styles.html` in minima
(has `author: ["Bart Simpson", "Nelson Mandela Muntz"]`)

### Example

Jekyll (correct -- no meta author for multi-author):
```html
<meta property="og:locale" content="en_US" />
<meta name="description" content="Lorem ipsum..." />
```
JSON-LD has no `"author"` field.

Rustkyll (wrong):
```html
<meta name="author" content="Bart SimpsonNelson Mandela Muntz" />
<meta property="og:locale" content="en_US" />
```
JSON-LD has `"author":{"@type":"Person","name":"Bart SimpsonNelson Mandela Muntz"}`.

## Root Cause

The `get_author_name()` function in `seo_tag.rs` likely converts an array value to a
string by concatenating elements. When `page.author` is an array, the function should
return None (or handle it specially) rather than concatenating.

## Dependencies

None.

## Scope

- Fix author extraction in SEO tag to detect array authors
- For arrays: do NOT emit `<meta name="author">` (matches Jekyll behavior)
- For arrays: do NOT include `"author"` in JSON-LD (matches Jekyll behavior)
- Single string authors should continue to work as before

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` produces no changes
- [ ] `cargo test` passes
- [ ] DTC DOM baseline: 790/790 (must not regress)
- [ ] Minima multi-author post: no `<meta name="author">` tag
- [ ] Minima multi-author post: JSON-LD has no `"author"` field
- [ ] Single-author posts: `<meta name="author">` still emitted correctly
- [ ] At least 3 new unit tests

## Test Scenarios

### Unit: multi-author detection
- page.author is a string "John" -> return Some("John")
- page.author is an array ["John", "Jane"] -> return None
- page.author is absent -> return None

### Unit: SEO tag with multi-author
- Post with array author -> no `<meta name="author">`, no JSON-LD author
- Post with single author -> `<meta name="author">` present, JSON-LD author present

### Integration: minima build
- Build minima, verify `this-post-demonstrates-post-content-styles.html` has no author meta tag
- Build minima, verify `my-example-post.html` (single author) still has author meta tag

## Baselines

- DTC: 790/790
- Minima: 0/9 (this fix should eliminate ~3 diffs on the multi-author post page)
