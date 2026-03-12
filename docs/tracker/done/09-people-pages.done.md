# Issue 09: People Collection Pages

## Description

Generate HTML pages for the `_people/` collection using the `author.html` layout. Each person gets a page at `/people/:title.html` with their profile, social links, related articles, events, and books.

This issue is the **page generation wiring**: load people via `collection::load_collection` -> build site context with collections and data -> render each person through `LayoutEngine::render_page` with the `author` layout -> write HTML to output directory.

## Dependencies

- Issue 05 (collection loader) -- DONE
- Issue 08 (layout and includes) -- DONE
- Issue 15 (filters: where_exp, jsonify, etc.) -- DONE

## Scope

### In scope

- A `generate_people_pages` function (or equivalent) that orchestrates:
  1. Load the `_people/` collection via `load_collection("people", ...)`
  2. Build a site context object containing `site.posts`, `site.books`, `site.data.events` (needed by the `author.html` layout for related content lookups)
  3. For each person, merge their front matter with the collection defaults (`layout: author`) from `_config.yml`
  4. Render each person through `LayoutEngine::render_page("author", ...)` passing the person's markdown content, front matter, and site context
  5. Write the rendered HTML to `<output_dir>/people/<slug>.html`
- Profile rendering: picture (`/{{ page.picture }}`), title (`{{ page.title }}`), bio content (`{{ content }}`)
- Social links: conditionally render twitter, linkedin, github, web links using the front matter fields
- Related articles: the layout uses `site.posts | where_exp: "post", "post.authors contains page.short"` -- the site context must include `site.posts` as an array of objects with `authors`, `title`, and `url` fields
- Related events: the layout uses `site.data.events | where_exp: "event", "event.speakers contains page.short"` -- the site context must include `site.data.events` with `speakers`, `title`, `time`, and `draft` fields
- Related books: the layout uses `site.books | where_exp: "book", "book.authors contains page.short"` -- the site context must include `site.books` as an array with `authors`, `title`, `id`, `start`, `end` fields
- JSON-LD Person schema block in the output (rendered by the layout template itself)
- The `event.html` include is referenced by the layout -- it must be loadable (already handled by issue 08's include system)

### Out of scope

- Full site build orchestration (issue 19)
- Other collection page generation (issues 10, 11, 12)
- Sitemap or RSS (issues 16, 17)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes with all new and existing tests
- [ ] A function exists that takes a site directory, config, and output directory, and generates people HTML pages
- [ ] Running the function against the real `datatalksclub.github.io/` directory produces 424+ HTML files in `<output>/people/`
- [ ] The generated HTML for `alexeygrigorev.html` contains:
  - The name "Alexey Grigorev"
  - The profile image path `images/authors/alexeygrigorev.jpg`
  - A Twitter link to `https://twitter.com/Al_Grigor`
  - A LinkedIn link to `https://linkedin.com/in/agrigorev`
  - A GitHub link to `https://github.com/alexeygrigorev`
  - A web link to `https://alexeygrigorev.com/`
  - The bio content "founder of DataTalks.Club"
- [ ] The generated HTML for `chiphuyen.html` contains:
  - The name "Chip Huyen"
  - Bio content mentioning "Stanford University"
  - Social links for all four platforms (twitter, linkedin, github, web)
- [ ] For a person who has related posts (where `post.authors` contains `page.short`), the "Articles" section appears with linked post titles
- [ ] For a person who has related events (where `event.speakers` contains `page.short`), the "Events" section appears
- [ ] For a person who has related books (where `book.authors` contains `page.short`), the "Books" section appears with linked book titles
- [ ] The JSON-LD `<script type="application/ld+json">` block is present in the output with `@type: Person`, the person's name, and `sameAs` links
- [ ] People with no social links do not produce empty/broken link markup
- [ ] The collection defaults from `_config.yml` (layout: author for type: people) are applied -- no explicit `layout` key needed in each person's front matter

## Test Scenarios

### Unit: Site context building
- Build a site context object from a config, a list of CollectionItems (posts), a list of CollectionItems (books), and a DataTree (events). Verify the context has `site.posts`, `site.books`, and `site.data.events` as arrays/objects accessible by Liquid templates.
- Verify that each post in `site.posts` includes `authors`, `title`, and `url` fields.
- Verify that each book in `site.books` includes `authors`, `title`, `id`, `start`, `end` fields.

### Unit: Front matter defaults merging
- Given a CollectionItem with no `layout` key in front matter and a config with defaults `{ type: people, layout: author }`, verify the resolved layout name is `"author"`.
- Given a CollectionItem with an explicit `layout: custom` in front matter, verify it overrides the default.

### Unit: Output path generation
- Given a person with slug `alexeygrigorev` and output dir `/tmp/site`, verify the output path is `/tmp/site/people/alexeygrigorev.html`.
- Given a person with slug `chiphuyen`, verify output path is `/tmp/site/people/chiphuyen.html`.

### Integration: Render a single person page
- Create a minimal LayoutEngine with a simplified author layout (just `{{ page.title }} {{ content }}`), render a person with known front matter and content, verify the output contains the person's name and bio.
- Render a person with all four social links, verify the output contains twitter, linkedin, github, and web URLs.
- Render a person with no social links, verify no link markup appears.

### Integration: Render with related content
- Set up a site context with one post whose `authors` list contains the person's `short` value. Render the person page and verify the "Articles" section appears with the post title.
- Set up a site context with one event whose `speakers` list contains the person's `short` value. Render and verify the "Events" section appears.
- Set up a site context with one book whose `authors` list contains the person's `short` value. Render and verify the "Books" section appears.
- Set up a site context with no matching posts/events/books. Verify none of the related sections appear.

### Integration: Full generation against real data
- Load the real `datatalksclub.github.io/` site, generate all people pages to a temp directory, verify 424+ HTML files are produced.
- Verify `alexeygrigorev.html` exists and contains expected content (name, social links, bio).
- Verify `chiphuyen.html` exists and contains expected content.
- Verify `andreaskretz.html` exists and contains expected content.
- Spot-check that JSON-LD script blocks are present in generated files.

### Edge cases
- A person file with empty content (front matter only) -- should still render with name and social links, no crash.
- A person file with no `short` field -- related content lookups should gracefully return no matches.
- A person file with only some social links (e.g., only twitter and github) -- only those links should appear.

## Output Verification

After building, manually inspect:
1. `<output>/people/alexeygrigorev.html` -- verify profile picture path, all 4 social links, bio text, and JSON-LD block
2. `<output>/people/chiphuyen.html` -- verify longer bio renders as HTML paragraphs, all social links present
3. At least one person page with related articles/events/books sections populated
4. Compare structure against the original Jekyll-rendered page to ensure layout fidelity
