# Issue 203: Fix missing HTML elements in rustkyll output (126 pages)

## Problem

126 pages are missing HTML elements that Jekyll includes. Mostly mlwiki.org (103), DTC (17), government-github (2), jekyll-docs (2), opensource-guide (2).

## Goal

Generate all expected HTML elements matching Jekyll output.

## Approach (TDD)

1. Write failing tests with sample HTML from affected pages
2. Fix markdown rendering or template output
3. Verify tests pass
