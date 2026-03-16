# Issue 157: Fix inline code class — need language-plaintext (136 diffs)

## Problem

Jekyll adds `class="language-plaintext highlighter-rouge"` to inline `<code>`. Rustkyll only adds `class="highlighter-rouge"`. Missing the `language-plaintext` part. 136 DOM diffs.

## Acceptance criteria

- Inline code gets `class="language-plaintext highlighter-rouge"` matching Jekyll
- 136 DOM diffs eliminated
- TDD: failing test, fix, test passes

## Implementation log

- Changed `add_inline_code_classes()` in `src/kramdown.rs` to emit `class="language-plaintext highlighter-rouge"` instead of `class="highlighter-rouge"`
- Updated doc comments to match new behavior
- Updated all test assertions in `src/kramdown.rs` and `src/template/filters/markdownify.rs`
- Removed stale negative assertions that checked inline code should NOT have `language-plaintext`
- All 1303 tests pass, clippy clean, fmt clean
