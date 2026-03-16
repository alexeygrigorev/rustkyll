# Issue 145: Fix extra class attribute on inline code elements

## Problem

Rustkyll adds `class='highlighter-rouge language-plaintext'` to inline `<code>` elements where Jekyll does not add any class. 9 instances across 3 files.

Example:
- Jekyll: `<code>some text</code>`
- Rustkyll: `<code class="highlighter-rouge language-plaintext">some text</code>`

Discovered in issue #119 DOM diff audit.

## Acceptance criteria

- Inline `<code>` elements do not get extra class attributes unless Jekyll also adds them
- No regressions in code block styling
