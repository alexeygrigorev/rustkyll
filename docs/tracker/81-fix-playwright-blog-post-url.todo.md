# Issue 81: Fix Playwright blog-post comparison URL

## Problem

The visual comparison Playwright spec (`playwright/tests/visual-compare.spec.ts`) defines the blog-post page with path `/segmentation/`, but this URL does not exist in either the Jekyll or rustkyll build output. The correct path is `/blog/segmentation.html`.

As a result, the blog-post visual comparison in issue #72 compared two identical 404 error pages and reported 0% diff -- a false pass that does not actually test blog post rendering.

## Fix

Update the blog-post path in `playwright/tests/visual-compare.spec.ts` from `/segmentation/` to `/blog/segmentation.html` (or another valid blog post URL).

Re-run the visual comparison for this page and verify the actual pixel diff against Jekyll output.

## Dependencies

- Issue 72 (visual diff investigation) -- done

## Discovered in

PM acceptance review of issue #72.
