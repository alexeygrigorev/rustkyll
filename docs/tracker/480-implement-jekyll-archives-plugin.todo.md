# Issue 480: Implement jekyll-archives generator plugin

## Problem

jekyll-archives generates tag and category archive pages automatically.
Many sites use it (jasper2, made-mistakes, academicpages, etc.). Without
it, tag/category index pages are missing from rustkyll output.

## Scope

Implement the jekyll-archives generator that:
1. Reads `jekyll-archives` config from _config.yml
2. For each tag/category, generates an archive page using the configured layout
3. Supports `type: [tags, categories]` and permalink patterns

## Reference

https://github.com/jekyll/jekyll-archives
