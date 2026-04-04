# Issue 574: Chirpy index pagination shows only 2 of 4 posts

## Problem

Chirpy's index.html should show all 4 posts (Customize the Favicon, Getting Started, Write a New Post, Text and Typography) but rustkyll only renders 2 unique posts (Getting Started, Text and Typography), each duplicated. Jekyll correctly shows all 4.

This was identified during issue #567 acceptance review as a pre-existing pagination bug unrelated to timezone handling.

## Expected Behavior

Chirpy index.html should list (newest first):
1. Customize the Favicon (Aug 10 UTC)
2. Getting Started (Aug 9)
3. Write a New Post (Aug 8)
4. Text and Typography (Aug 8)

## Current Behavior

Only Getting Started and Text and Typography appear, each duplicated.

## Scope

- Investigate why Chirpy's jekyll-paginate-v2 pagination drops 2 of 4 posts
- Fix so all posts appear in correct order on the index

## Dependencies

None (issue #567 already fixed the timezone sorting).

## Log
