# Issue 157: Fix inline code class — need language-plaintext (136 diffs)

## Problem

Jekyll adds `class="language-plaintext highlighter-rouge"` to inline `<code>`. Rustkyll only adds `class="highlighter-rouge"`. Missing the `language-plaintext` part. 136 DOM diffs.

## Acceptance criteria

- Inline code gets `class="language-plaintext highlighter-rouge"` matching Jekyll
- 136 DOM diffs eliminated
- TDD: failing test, fix, test passes
