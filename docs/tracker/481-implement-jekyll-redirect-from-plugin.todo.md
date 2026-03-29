# Issue 481: Implement jekyll-redirect-from generator plugin

## Problem

jekyll-redirect-from generates HTML redirect pages for moved content.
Sites like made-mistakes, programming-historian use it. Without it,
redirect pages are missing from rustkyll output.

## Scope

Implement the jekyll-redirect-from generator that:
1. Reads `redirect_from` frontmatter on pages/posts
2. Generates simple HTML redirect pages at the old URLs
3. Reads `redirect_to` frontmatter for external redirects

## Reference

https://github.com/jekyll/jekyll-redirect-from
