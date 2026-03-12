# Issue 10: Blog Posts

## Description

Generate HTML pages for `_posts/` using the `post.html` layout. Each post gets a page at `/blog/:title.html` with title, subtitle, authors, date, content, and Article JSON-LD schema.

This is a page generation wiring issue: load the posts collection (already done in `collection.rs`), render each post through the `post.html` layout (already done in `template/layout.rs`), and write the resulting HTML files to the output directory.

## Dependencies

- Issue 05 (collection loader) -- DONE
- Issue 08 (layout and includes) -- DONE

## Scope

### In Scope

- A `generate_posts` function (or equivalent) that:
  1. Loads the `_posts/` collection using `load_collection("posts", ...)`
  2. For each `CollectionItem`, determines the layout (`post` by default, or from front matter `layout` field)
  3. Builds a site context `Object` that includes `site.people` (loaded from `_people/` collection) so that the `where` filter in the post layout can resolve author names
  4. Calls `LayoutEngine::render_page(layout, content, front_matter, site_context)` for each post
  5. Writes the rendered HTML to the output directory at the correct path (e.g., `_site/blog/segmentation.html`)
- A Liquid `where` filter (not `where_exp`, the simpler `| where: "field", value` form) -- the post layout uses `site.people | where: "short", a | first` to look up authors
- Title display uses `page.h1` falling back to `page.title` (matching the Jekyll layout)
- Subtitle rendering when `page.subtitle` is present
- Author byline with links to `/people/:short.html`
- Date formatting via `date_to_string` filter (already implemented)
- Full markdown content rendered to HTML and inserted as `{{ content }}`
- `{% include youtube.html %}` already works through the includes system (issue 08)
- `{% include subscribe.html %}` already works through the includes system
- JSON-LD Article schema block with:
  - headline, alternativeHeadline (subtitle)
  - image, datePublished, dateModified
  - author array with name, url, description, image, sameAs (social links)
  - publisher (Organization)
  - BreadcrumbList (Home > Blog > Post)
- JSON-LD uses `jsonify` filter (already implemented) and `date_to_xmlschema` filter (already implemented)

### Out of Scope

- Full site build orchestration (issue 19)
- Sitemap entries (issue 16)
- RSS feed entries (issue 17)
- Complex JSON-LD schemas beyond Article and BreadcrumbList for posts (issue 18 covers the general system)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] A `where` Liquid filter is implemented that supports `array | where: "field", "value"` syntax (returns filtered array)
- [ ] A function exists that takes a site directory and output directory, loads posts, renders them through layouts, and writes HTML files
- [ ] Running the function against `datatalksclub.github.io/` produces 55 HTML files under `<output>/blog/`
- [ ] Each generated HTML file is non-empty and contains valid HTML structure (`<html`, `<body`, `</html>`)
- [ ] The generated file at `blog/segmentation.html` contains:
  - The h1 heading text from the post's front matter
  - The subtitle "Build a 5D RFM+ framework"
  - An author link to `/people/nishantmohan.html`
  - The date string "Nov 29, 2020" (or equivalent `date_to_string` output)
  - The rendered markdown content (e.g., "Background" heading)
  - A JSON-LD `<script>` block with `"@type": "Article"`
  - A BreadcrumbList in the JSON-LD with "Home", "Blog", and the post title
- [ ] The generated file at `blog/mlops-10-minutes.html` contains:
  - Author link to `/people/alexeygrigorev.html`
  - The author name "Alexey Grigorev" resolved from `site.people`
  - Tags `mlops`, `team`, `process` in JSON-LD keywords
- [ ] The generated file at `blog/hiring-process-for-data-professionals.html` contains:
  - Author link to `/people/pavelchernetsov.html`
  - The post content rendered from markdown
- [ ] Posts with `{% include youtube.html video_id="..." %}` render the YouTube iframe embed
- [ ] `cargo test` passes with all new tests (at least 10 new tests)

## Test Scenarios

### Unit: `where` filter

- Test `where` filter on an array of objects: filter by a string field, verify correct items returned
- Test `where` filter with no matches returns empty array
- Test `where` filter on empty array returns empty array
- Test `where` filter on non-array input returns empty array

### Unit: Post output path generation

- Given a `CollectionItem` with slug `segmentation` and URL `/blog/segmentation.html`, verify the output file path is `<output_dir>/blog/segmentation.html`
- Given a slug with hyphens like `mlops-10-minutes`, verify output path is `<output_dir>/blog/mlops-10-minutes.html`

### Integration: Render a single post through post layout

- Load the real `post.html` layout and includes from `datatalksclub.github.io/`
- Load the `segmentation` post from `_posts/`
- Build a site context with `site.people` populated from `_people/`
- Render the post and verify the output contains `<h1>`, subtitle text, author link, date, content, and JSON-LD script block

### Integration: Generate all posts to output directory

- Run the post generation function against `datatalksclub.github.io/`
- Verify 55 HTML files are written to `<output>/blog/`
- Verify each file is non-empty
- Spot-check 3 known posts for expected content

### Integration: Author resolution from site.people

- Render a post that has `authors: [alexeygrigorev]`
- Verify the output contains the author's full name "Alexey Grigorev" (resolved via `site.people | where: "short", "alexeygrigorev"`)
- Verify the author link points to `/people/alexeygrigorev.html`

### Integration: YouTube include in posts

- Render a post containing `{% include youtube.html video_id="..." %}`
- Verify the output contains an `<iframe>` with the YouTube embed URL

### Integration: JSON-LD schema correctness

- Parse the JSON-LD block from a rendered post's HTML
- Verify it contains `"@type": "Article"` with correct headline
- Verify the BreadcrumbList has 3 items (Home, Blog, Post title)
- Verify the author array contains the expected Person objects with name and URL

### Edge case: Post with no subtitle

- Render a post that has no `subtitle` in front matter
- Verify the output does not contain an `<h3>` subtitle element

### Edge case: Post with datepublished vs date

- Verify that when `datepublished` is present in front matter, the JSON-LD uses it for `datePublished`
- Verify `dateModified` uses the `date` field

## Implementation Notes

- The `where` filter is different from `where_exp`. Jekyll's `where` syntax is `array | where: "property", "value"` and returns all items where `item.property == value`. It needs to be registered alongside the existing `where_exp` filter.
- The site context must include `site.people` as an array of objects, where each object has the front matter fields of each person (especially `short`, `title`, `picture`, `bio_short`, `linkedin`, `twitter`, `github`, `web`). This is needed because the post layout does `site.people | where: "short", a | first` to look up author details.
- The `content` field of each person should also be available in the site.people objects (the post layout uses `author.content` in JSON-LD).
- Post content may contain Liquid tags (like `{% include youtube.html %}`) that need to be rendered before being inserted into the layout. The existing `render_page` method already handles this two-step process.
- Output directory structure: posts go to `<output>/blog/<slug>.html` based on the permalink pattern `/blog/:title.html`.
