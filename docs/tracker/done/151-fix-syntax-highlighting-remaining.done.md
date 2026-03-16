# Issue 151: Fix remaining syntax highlighting DOM diffs (~1270 diffs, 26 files)

## Problem

Syntect and Rouge produce different token boundaries and CSS classes for code blocks. This is the single largest DOM diff category. Concentrated in code-heavy blog posts.

## Goal

Reduce syntax highlighting DOM diffs to near zero. Investigate specific token mismatches and fix the scope-to-CSS mapping in src/syntax.rs.

## Acceptance criteria

- DOM diffs from syntax highlighting reduced by 80%+
- Affected blog posts achieve 0% pixel diff
- No regressions on other code blocks

## Log

### [SWE] 2026-03-16
- Investigation: Built DTC site with rustkyll and compared all highlighted code blocks
  (div.highlight, div.highlighter-rouge, pre, code) against Jekyll output
- Found that prior work had already resolved the majority of syntax highlighting diffs
- Remaining issue found: bash line continuation (`\` at end of line) was mapped to `p`
  by the existing `punctuation.separator` rule, which matched Rouge. However, adding
  a new `punctuation.separator.continuation.line` -> `se` mapping introduced 3 regressions
  in blog/ml-deployment-lambda.html. Root cause: Rouge treats `\<newline>` (line
  continuation) as `p` but `\\` (escaped backslash) as `se`. Syntect assigns both the
  scope `punctuation.separator.continuation.line` for the former and
  `constant.character.escape` for the latter.
- Fix: Added `punctuation.separator.continuation.line` -> `p` mapping (before the generic
  `punctuation.separator` -> `p` rule) for explicitness and documentation. Removed
  the stale `dockerfile` alias from find_syntax since syntect has no Dockerfile grammar
  and Rouge treats `docker` as plaintext (no highlighting).
- Added 21 new tests:
  - 10 SQL token mapping tests (SELECT, FROM, WHERE, JOIN, AS, NULL, COUNT, numbers, operators)
  - 1 Docker/Dockerfile test (correctly returns None matching Rouge plaintext behavior)
  - 2 bash line continuation tests (escaped backslash = se, line continuation = p)
  - 8 regression tests from real DTC blog code blocks (Python, Bash, YAML, SQL)
- Final state: 0 syntax highlighting diffs across all 787 HTML files in the DTC site
- Build: 1286 tests pass, 0 fail, clippy clean, fmt clean
- Files modified: src/syntax.rs
