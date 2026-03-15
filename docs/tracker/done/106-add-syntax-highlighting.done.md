# Issue 106: Add syntax highlighting (Rouge-compatible spans)

## Problem

Jekyll uses Rouge for syntax highlighting, producing <span class="c1">, <span class="k">, etc. inside code blocks. rustkyll wraps code blocks in the correct kramdown div structure but does not generate syntax highlighting tokens.

This causes visible differences on blog posts with code blocks.

## Acceptance criteria

- Code blocks with language tags produce syntax-highlighted HTML matching Rouge output
- Blog post /blog/practical-guide-better-code.html achieves 0% pixel diff
- Consider using syntect crate for highlighting
- All existing tests pass

## Log

### [SWE] 2026-03-15

- Added `syntect` crate (v5, with default-syntaxes and regex-onig features)
- Created `src/syntax.rs` module: maps TextMate scopes to Rouge/Pygments CSS classes (c1, kn, k, nf, sd, s2, etc.)
- Integrated into `wrap_fenced_code_blocks()` in `src/kramdown.rs`: code content is unescaped, highlighted via syntect, then re-escaped with Rouge-compatible spans
- Added `html_unescape()` helper in kramdown.rs to reverse HTML entity escaping before passing to syntect
- Plaintext and unknown languages gracefully fall back to unhighlighted output
- Language aliases supported: js->javascript, sh/shell/console->bash, py->python, yml->yaml, etc.
- 11 new unit tests in `src/syntax.rs` covering Python, YAML, bash, aliases, HTML escaping, plaintext fallback
- Build: 1263 tests pass (0 failed, 29 ignored), clippy clean, fmt clean
- Verified output: /blog/practical-guide-better-code.html now has proper `<span class="kn">`, `<span class="sd">`, `<span class="k">`, etc.
- Files created: src/syntax.rs
- Files modified: Cargo.toml, src/lib.rs, src/kramdown.rs
