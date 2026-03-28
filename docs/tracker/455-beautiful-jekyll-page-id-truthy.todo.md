# Issue 455: beautiful-jekyll — page.id evaluates as falsy in Liquid

## Problem

beautiful-jekyll (3/5, 60%) has 2 post pages with 66 diffs, all from
one root cause: `{% if page.id %}` evaluates as falsy for posts.

Jekyll sets page.id for posts (e.g., `/2020/02/26/flake-it-till-you-make-it`).
Rustkyll computes it correctly but the Liquid engine treats it as falsy.

This causes posts to render with `og:type: website` instead of `article`,
and missing `og:article:author` and `og:article:published_time` tags.

## Root Cause

Either:
1. page.id is not being set in the Liquid context for collection items
2. page.id is set but as nil/empty instead of the actual ID string

## Scope

Fix page.id to be truthy (non-empty string) for collection items in
the Liquid context. Check src/generator.rs or src/collection.rs where
page variables are set for Liquid rendering.

## Baseline

DTC 790/790. beautiful-jekyll 3/5.
Target: 5/5 (100%).
