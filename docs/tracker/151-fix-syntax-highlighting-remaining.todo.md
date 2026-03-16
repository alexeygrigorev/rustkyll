# Issue 151: Fix remaining syntax highlighting DOM diffs (~1270 diffs, 26 files)

## Problem

Syntect and Rouge produce different token boundaries and CSS classes for code blocks. This is the single largest DOM diff category. Concentrated in code-heavy blog posts.

## Goal

Reduce syntax highlighting DOM diffs to near zero. Investigate specific token mismatches and fix the scope-to-CSS mapping in src/syntax.rs.

## Acceptance criteria

- DOM diffs from syntax highlighting reduced by 80%+
- Affected blog posts achieve 0% pixel diff
- No regressions on other code blocks
