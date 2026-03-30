# Issue 346: Jasper2 custom author/tag page generators are not executed

## Problem

Jasper2 ships Ruby Jekyll plugins that generate author and tag archive pages:
- `_plugins/jekyll-autgenerator.rb` -- generates `/author/<name>/index.html` and `/author/<name>/feed.xml` pages for each author defined in `_data/authors.yml`, with pagination support
- `_plugins/jekyll-tagsgenerator.rb` -- generates `/tag/<slug>/index.html` and `/tag/<slug>/feed.xml` pages for each tag used in posts, with pagination support

Jekyll executes these generator plugins during the build, producing the full author/tag page trees. rustkyll currently does not execute site-local generator plugins, so the Jasper2 output is missing all author and tag archive pages plus their Atom feeds.

## Root Cause

rustkyll has no mechanism to interpret or emulate Ruby generator plugins from `_plugins/`. These plugins use the `Jekyll::Generator` API to programmatically create new `Page` objects and inject them into `site.pages`.

## Scope

1. Implement a generic mechanism to detect and emulate common generator plugin patterns (author pages, tag pages) based on the plugin source or configuration.
2. For Jasper2 specifically:
   - Generate `/author/<name>/index.html` for each author in `_data/authors.yml`, using the `author.html` layout
   - Generate `/author/<name>/feed.xml` for each author, using the `feed.xml` layout
   - Generate `/tag/<slug>/index.html` for each tag in `site.tags`, using the `tag.html` layout
   - Generate `/tag/<slug>/feed.xml` for each tag, using the `feed.xml` layout
   - Support pagination on author/tag index pages (matching `jekyll-paginate` behavior)
3. The implementation must be generic (not Jasper2-hardcoded) -- it should work for any site that uses similar author/tag generator patterns.
4. Set the correct context variables on generated pages: `grouptype`, `author`/`tag`, and `pager` for paginated pages.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with tests covering author and tag page generation
- [ ] Building `websites/jasper2/` produces `/author/<name>/index.html` pages for each author in `_data/authors.yml`
- [ ] Building `websites/jasper2/` produces `/tag/<slug>/index.html` pages for each tag used in posts
- [ ] Each generated author page uses the `author.html` layout and has the correct `author` and `grouptype` context variables
- [ ] Each generated tag page uses the `tag.html` layout and has the correct `tag` and `grouptype` context variables
- [ ] Author and tag Atom feeds (`feed.xml`) are generated for each author/tag
- [ ] Pagination works on author/tag pages consistent with `jekyll-paginate` behavior (100 posts per page as per Jasper2 config)
- [ ] The implementation is generic -- driven by plugin detection or config, not hardcoded site names
- [ ] The Jasper2 DOM comparison improves from the #240 baseline
- [ ] DTC DOM count remains at 788/790 or above

## Test Scenarios

### Unit: author page generation
- Given a site with `_data/authors.yml` containing 2 authors and 5 posts split between them, verify that 2 author index pages and 2 author feed pages are generated
- Verify each generated author page has `grouptype: "author"` and the correct `author` value in its context
- Verify posts are sorted by date descending on author pages

### Unit: tag page generation
- Given a site with posts tagged `"rust"`, `"python"`, and `"web"`, verify that 3 tag index pages and 3 tag feed pages are generated
- Verify tag slugs are lowercased/slugified consistently
- Verify each generated tag page has `grouptype: "tag"` and the correct `tag` value in its context

### Unit: pagination
- Given 150 posts for one author with `paginate: 100`, verify 2 pages are generated: `/author/<name>/index.html` and `/author/<name>/page2/index.html`
- Verify the `pager` object has correct `page`, `total_pages`, `previous_page`, `next_page` fields

### Integration: Jasper2 author/tag pages
- Build `websites/jasper2/` with rustkyll and verify `/author/` and `/tag/` directories exist in output
- Inspect a generated author page for correct layout rendering and post listing
- Inspect a generated tag page for correct layout rendering and post listing
- Verify the Atom feeds contain valid XML with post entries

## Dependencies

- Issue #240 (must be `.done.md` or `.in-progress.md`)
