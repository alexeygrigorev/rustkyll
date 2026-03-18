# Issue 197: Fix remaining syntax highlighting differences (574 pages)

## Problem

574 pages have syntax highlighting token class differences. Largest: large-docs-site (500, JSON strings), mlwiki.org (47), DTC (11), mlbookcamp-page (6), plus 1 page each on 10 theme sites.

## Goal

Match Rouge token classes exactly for all languages used across benchmark sites.

## Approach (TDD)

1. large-docs-site (500): Issue 193 is investigating. JSON string token merging.
2. mlwiki.org (47): Mostly XML/HTML code blocks. Investigate remaining token diffs.
3. DTC (11): Various languages. Sample and fix per-language.
4. mlbookcamp-page (6): Bash/YAML/Python code blocks.
5. Theme sites (1 each): Usually JavaScript code blocks.

## Acceptance Criteria

- [ ] large-docs-site JSON tokens fixed (issue 193)
- [ ] mlwiki.org remaining token diffs categorized and fixed
- [ ] DTC syntax diffs fixed
- [ ] Theme site code block diffs fixed
