# Issue 93: Pixel-perfect match for all DTC pages

## Priority

CRITICAL — this is the project's core deliverable. The DTC site must look identical to Jekyll.

## Pages to verify

Every page listed below must achieve 0% Playwright pixel diff against Jekyll output (only dynamic timestamps excepted). This is the acceptance criteria — all pages must pass.

### Listing/index pages
1. `/` — Homepage (index.md)
2. `/articles` — Articles listing
3. `/books.html` — Books listing
4. `/podcast.html` — Podcast listing
5. `/events.html` — Events listing
6. `/courses.html` — Courses listing
7. `/people.html` — People listing
8. `/support.html` — Support page
9. `/tools.html` — Tools listing
10. `/slack.html` — Slack page
11. `/slack/guidelines.html` — Slack guidelines

### Blog posts (sample 3)
12. `/blog/segmentation.html` — A blog post with tags and content
13. `/blog/practical-guide-better-code.html` — Another blog post
14. `/blog/data-roles.html` — Third blog post

### Book detail pages (sample 2)
15. `/books/ml-bookcamp.html` — ML Bookcamp book page
16. `/books/20210111-reinforcement-learning.html` — RL book page

### Podcast episode pages (sample 2)
17. `/podcast/ab-testing-and-product-experimentation.html` — Podcast episode
18. `/podcast/ai-for-ecology-biodiversity-and-conservation.html` — Another episode

### People detail pages (sample 2)
19. `/people/alexeygrigorev.html` — Person page
20. `/people/aaishamuhammad.html` — Another person page

### Course pages (sample 1)
21. `/courses/2021-winter-ml-zoomcamp.html` — Course page

### Conference pages (sample 1)
22. `/conferences/2021-feb.html` — Conference page

### Feeds and sitemap
23. `/feed.xml` — Atom feed (valid XML, no Liquid tags)
24. `/sitemap.xml` — Sitemap (valid XML, same URLs as Jekyll)

## Total: 24 pages/resources to verify

## Acceptance criteria

For pages 1-22:
- Playwright screenshot comparison: 0% pixel diff (pixel-perfect)
- Structural HTML comparison: same elements, same attributes, same content
- No raw Liquid tags in output
- No missing content sections
- No extra `<p>` wrapping (issue #92)
- CSS styling matches (same classes, same visual appearance)

For pages 23-24:
- Valid XML
- Same entries/URLs as Jekyll output

## How to verify

1. Build DTC site with Jekyll: `cd datatalksclub.github.io && bundle exec jekyll build`
2. Build DTC site with rustkyll: `rustkyll build --source datatalksclub.github.io`
3. Serve both on different ports
4. Run Playwright on all 22 HTML pages
5. Validate XML for feed and sitemap

## Dependencies

- Issue 84 (kramdown compatibility) done
- Issue 85 (fenced code blocks) in progress
- Issue 92 (paragraph wrapping) to do

## This issue is DONE when

All 24 pages pass their respective checks. Not 23/24. Not "most pages pass". ALL 24. Any failing page means the issue is not done — either fix it or the issue stays open.
