# Issue 513: government-github -- avatar tag variable resolution and lazy loading

## Problem

The community/index.html page on government-github has ~9493 DOM differences due to multiple bugs in the `{% avatar %}` tag implementation. This single page accounts for 99.4% of all remaining diffs on the site.

Three sub-problems:

### 1. Unknown parameters overwrite the username

The avatar tag parser treats unknown key=value parameters (like `lazy=true`) as literal usernames, overwriting a previously parsed `user=variable`. In `{% avatar user=org size=60 lazy=true %}`:
- Parser sets username_source = Variable("org") for `user=org`
- Parser then hits `lazy=true`, falls into the else branch, sets username_source = Literal("lazy")
- Result: all avatars render as `alt="lazy"` with `src="...githubusercontent.com/lazy?..."`

### 2. No lazy loading support

The cached Jekyll build uses the older jekyll-avatar plugin that generates lazy-loading attributes:
- `src=""` (empty)
- `data-src="https://avatars2.githubusercontent.com/<org>?v=3&s=60"`
- `data-srcset="..."`
- `data-proofer-ignore="true"`

Rustkyll generates eager-loading attributes:
- `src="https://avatars.githubusercontent.com/<org>?v=4&s=60"`
- `srcset="..."`

### 3. URL format mismatch

The cached Jekyll build uses `avatars2.githubusercontent.com` (or `avatars3`) with `v=3`, while rustkyll uses `avatars.githubusercontent.com` with `v=4`.

## Affected Pages

- community/index.html (9493 differences, 1582 avatar images)

## Root Cause

In `src/template/avatar_tag.rs`, the `ParseTag::parse` method:
1. Line 86-88: Unknown key like `lazy` falls into the else branch that sets `username_source = Some(UsernameSource::Literal(id_str))`, overwriting a valid previous `user=` assignment
2. The `render_avatar` function uses modern GitHub avatar URLs (`avatars.githubusercontent.com`, `v=4`) and direct `src`/`srcset` instead of lazy-loading attributes

## Solution

1. Fix parser: only set `username_source` in the else branch if it hasn't been set yet (or better, explicitly handle only `user` and `size` keys, ignoring unknown keys like `lazy`)
2. When `lazy=true` is specified, render with `data-src`/`data-srcset`/`data-proofer-ignore` attributes and empty `src`
3. Match the cached Jekyll URL format: use `avatars.githubusercontent.com` (the subdomain number varies per org in the cached build, but using the generic `avatars.githubusercontent.com` should be acceptable since GitHub redirects these)

NOTE: The `v=3` vs `v=4` and exact subdomain differences may not matter for DOM comparison if the comparator treats URL content as opaque. The critical fixes are (1) correct org name resolution and (2) matching attribute structure (lazy vs eager).

## Acceptance Criteria

- [ ] `{% avatar user=org size=60 lazy=true %}` resolves `org` from the Liquid context, not "lazy"
- [ ] Unknown parameters (like `lazy=true`) are silently ignored without overwriting `user=`
- [ ] When `lazy=true` is specified, output uses `data-src`/`data-srcset` instead of `src`/`srcset`
- [ ] When `lazy=true` is specified, `src` is empty and `data-proofer-ignore="true"` is present
- [ ] Avatar `alt` attribute contains the actual org name (e.g., `alt="argob"`) not `alt="lazy"`
- [ ] community/index.html renders with correct org names for all 1582 avatars
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes
- [ ] DTC DOM baseline must not drop below 790/790
- [ ] government-github DOM score improves (community page diffs drop dramatically)

## Test Scenarios

### Unit: Avatar tag parser
- Parse `{% avatar user=org size=60 lazy=true %}` -- verify username_source = Variable("org"), not Literal("lazy")
- Parse `{% avatar user=org lazy=true size=60 %}` -- verify parameter order doesn't matter
- Parse `{% avatar username size=40 %}` -- verify literal username still works
- Parse `{% avatar user=org %}` -- verify defaults (size=40, lazy=false)

### Unit: Avatar rendering with lazy loading
- Render avatar with lazy=true: verify `data-src`, `data-srcset`, empty `src`, `data-proofer-ignore`
- Render avatar with lazy=false (default): verify `src`, `srcset` directly

### Integration: government-github community page
- Build government-github, verify community/index.html contains `alt="argob"` (first org)
- Verify avatar count matches (1582 avatars)
- Verify no avatar has `alt="lazy"`

## Dependencies

- None (independent of other government-github issues)
