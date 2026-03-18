# Issue 196: Fix layout/template not applied (773 pages)

## Problem

773 pages render without their layout template. The output has raw content (h1, p, ul) at root level instead of inside the proper html/head/body structure. Affects: opensource-guide (337), jekyll-docs (109), documentation-theme-jekyll (97), DTC/docs (57), choosealicense (55), just-the-docs (47), alexeygrigorev/snippets (17), academicpages (16), so-simple-theme (11), little-book-of-metals-ru (9), muan-blog (7), beautiful-jekyll (5), government-github (6).

## Goal

Fix layout resolution so all pages render with their correct layout.

## Approach

Break by root cause - investigate each site's failure mode:
1. **Gem themes** (just-the-docs, beautiful-jekyll, so-simple-theme, academicpages): Missing _layouts from gem. Need to resolve gem theme layout paths.
2. **Data-driven navigation** (documentation-theme-jekyll, DTC/docs): Templates use Liquid features that fail silently, causing layout to not render.
3. **Include resolution** (snippets, opensource-guide): _includes files referenced in layouts not found.
4. **Layout inheritance** (little-book-of-metals-ru): layout: page -> layout: default chain not working.

Each sub-cause should become its own issue if complex.

## Acceptance Criteria

- [ ] Investigate and categorize all 773 pages by specific failure mode
- [ ] Fix each failure mode or create sub-issues
- [ ] Pages render with correct layout structure
