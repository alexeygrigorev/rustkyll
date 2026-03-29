# Issue 537: Inline code missing `language-plaintext` class (minima, kramdown 2.4)

## Problem

Minima uses kramdown 2.4.0 which adds `language-plaintext` to inline code elements.
Rustkyll produces `<code class="highlighter-rouge">` instead of
`<code class="language-plaintext highlighter-rouge">`.

### Example

Jekyll (correct):
```html
<code class="language-plaintext highlighter-rouge">_posts</code>
```

Rustkyll (wrong):
```html
<code class="highlighter-rouge">_posts</code>
```

### Affected pages

- `welcome-to-jekyll.html` (2 inline code instances)
- `this-post-demonstrates-post-content-styles.html` (1 inline code instance)

## Root Cause

Issue 470 was completed and should handle this. However, minima's kramdown version
(2.4.0) should trigger the `language-plaintext` class addition. Investigate whether
the minima site's Gemfile.lock is being parsed correctly to determine the kramdown
version, or whether the flag is not propagating for this site.

## Dependencies

- Issue 470 (done) -- the fix should already exist but may not be working for minima

## Scope

- Investigate why minima's inline code does not get `language-plaintext` class
- Fix the kramdown version detection or flag propagation for minima
- Verify without breaking sites that use kramdown >= 2.5 (which should NOT have the class)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` produces no changes
- [ ] `cargo test` passes
- [ ] DTC DOM baseline: 790/790 (must not regress)
- [ ] Minima: inline `<code>` elements have `class="language-plaintext highlighter-rouge"`
- [ ] Sites using kramdown >= 2.5: inline `<code>` has `class="highlighter-rouge"` (no `language-plaintext`)
- [ ] At least 2 new unit tests

## Test Scenarios

### Unit: kramdown version detection
- Gemfile.lock with kramdown 2.4.0 -> `language-plaintext` class added
- Gemfile.lock with kramdown 2.5.0 -> `language-plaintext` class NOT added

### Integration: minima build
- Build minima, verify inline code in `welcome-to-jekyll.html` has `language-plaintext`

## Baselines

- DTC: 790/790
- Minima: this fix should eliminate 3 diffs across 2 pages
