# Issue 443: jekyll-vitepress-theme syntax highlighting and scripts

## Problem

jekyll-vitepress-theme (0/17, 354 diffs) has multiple issues:
1. Missing 3 `<script>` tags in head
2. Code block span classes wrong (Rouge token mapping)
3. Version string 'v1.1.1' rendered as 'auto'
4. Custom includes showing raw Liquid syntax

## Scope

Investigate and fix each sub-issue for this theme.
