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

## Log

### [SWE] 2026-03-16
- Root cause: `add_inline_code_classes()` in `src/kramdown.rs` unconditionally added `class="language-plaintext highlighter-rouge"` to all inline `<code>` tags. Jekyll does NOT add classes to inline code elements -- only fenced code blocks (handled separately by `wrap_fenced_code_blocks`) get the kramdown wrapper divs.
- Fix: Removed the `add_inline_code_classes()` function entirely and its call sites in `postprocess()` and `postprocess_for_filter()`.
- Updated 6 tests in `src/kramdown.rs` and 1 test in `src/template/filters/markdownify.rs` that previously asserted the incorrect behavior.
- Tests now assert inline `<code>` elements remain bare (no extra class attributes).
- Fenced code block wrapping (in `<pre>`) is completely unaffected -- those are handled by `wrap_fenced_code_blocks()`.
- Build: 1244 lib tests + all integration tests pass, 0 failures
- Clippy: clean (no warnings)
- Fmt: clean
- Files modified: `src/kramdown.rs`, `src/template/filters/markdownify.rs`
