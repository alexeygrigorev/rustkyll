# Issue 249: Fix JavaScript syntax highlighting Rouge compatibility

## Problem

Descoped from issue 229. All 10 GitHub Pages theme sites share the same JavaScript code block in `index.md`:

```js
var fun = function lang(l) {
  dateformat.i18n = require('./lang/' + l)
  return true;
}
```

Rustkyll's syntect-based highlighter produces different token classes and span boundaries compared to Rouge (Jekyll's highlighter), causing ~61 DOM differences per page.

### Specific differences

1. **Function name class**: Rouge uses `nf` (name.function) for `lang` in `function lang(l)`. Rustkyll produces `nx` (name.other) because `src/syntax.rs` line 60 explicitly overrides `source.js entity.name.function` to `nx`. This override is incorrect for function declarations -- Rouge only uses `nx` for identifier references, not declarations.

2. **String delimiter splitting**: Rouge splits single-quoted strings into three tokens: `'` (class `dl` = string delimiter) + content (class `s1`) + `'` (class `dl`). Syntect emits the entire `'./lang/'` as a single `s1` token. This is a fundamental difference in tokenization granularity that requires changes to `accumulate_and_emit` in `src/syntax.rs`.

3. **Cascading span boundary differences**: Because the string tokenization produces different span boundaries, the text content of subsequent spans also differs (e.g., `+` operator ends up in the wrong span).

## Scope

1. Fix the `entity.name.function` mapping for JavaScript to produce `nf` instead of `nx` for function declarations
2. Implement string delimiter splitting in the syntax highlighter to emit separate `dl` spans for quote characters
3. Verify the fix resolves all ~61 diffs per theme site page

## Dependencies

- Issue 229 (site.github fixes) -- should be done first but not strictly required

## Log

- 2026-03-19: Created, descoped from issue 229.
