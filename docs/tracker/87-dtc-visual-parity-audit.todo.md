# Issue 87: DTC website visual parity audit — find and fix all differences

## Priority

HIGH — the DTC site is the primary reference site. It must look identical to Jekyll output when served in a browser.

## Problem

User tested the DTC site with rustkyll v0.1.4 and it looks different from the Jekyll-built version. The Playwright comparison showed 1.8-2.9% pixel diffs on some pages, but there are likely more issues visible to a human reviewer that automated comparison missed.

## Goal

Systematically compare every major section of the DTC site (homepage, blog, books, podcast, events, courses, people, articles) between Jekyll and rustkyll, identify ALL visual differences, and fix them or create tracked issues for each.

## Approach

1. Build the DTC site with both Jekyll and rustkyll
2. Serve both on separate ports
3. Manually browse every major section side-by-side
4. Screenshot and document every visual difference found
5. Categorize each difference:
   - Missing content (sections, sidebars, widgets not rendering)
   - Wrong content (different text, broken links, wrong data)
   - Styling differences (CSS classes missing, wrong spacing, wrong fonts)
   - Layout differences (elements in wrong position, missing structure)
   - Missing images or assets
   - Missing JavaScript functionality
6. Fix what can be fixed in this issue
7. Create separate issues for large fixes

## Pages to audit (at minimum)

- Homepage (index.html)
- A blog post (/blog/segmentation.html)
- Blog listing
- Books page (/books.html) + a book detail page
- Podcast page (/podcast.html) + a podcast episode
- Events page (/events.html)
- Courses page (/courses.html)
- People page (/people.html) + a person detail page
- Articles page (/articles.html)
- Community page
- Navigation (header, footer)
- RSS feed in browser

## Dependencies

- Issue 84 (kramdown compatibility) done

## Acceptance criteria

- Every major DTC page audited side-by-side with Jekyll
- All visual differences documented with screenshots
- Each difference categorized (missing content, styling, layout, etc.)
- Root cause identified for each difference
- Fixes applied where possible
- Follow-up issues created for remaining differences (with screenshots)
- After fixes, re-run Playwright comparison and document updated pixel diff numbers
- Target: <0.5% pixel diff on all pages (ideally 0%)
