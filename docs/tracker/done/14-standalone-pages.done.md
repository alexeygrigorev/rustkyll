# Issue 14: Standalone Pages

## Description

Generate HTML for standalone `.md` pages in the site root: `index.md` (homepage), `articles.md`, `books.md`, `people.md`, `podcast.md`, `events.md`, `courses.md`, `slack.md`, `support.md`, `tools.md`.

These pages are already loaded by `collection::load_pages()` (from issue 05). Each has YAML front matter with a `layout` key (`page` or `home`) and Liquid template code in the body. The body content must be rendered through the template engine (to process Liquid tags/filters) and then wrapped in the specified layout.

## Dependencies

- Issue 05 (collection loader) -- DONE (provides `load_pages()` and `Page` struct)
- Issue 08 (layout and includes) -- DONE (provides `LayoutEngine`)
- Issue 13 (events rendering) -- must be DONE first (events.md and index.md depend on event filtering/rendering)

## Scope

### What already exists

- `collection::load_pages()` loads all root `.md` files and returns `Vec<Page>` with front matter, content, html_content, url, slug
- `collection::Page` struct with slug, front_matter, content, html_content, url
- `LayoutEngine` can render content through a layout with `render_page()`
- `build_site_context()` produces the full `site` namespace with all collections and data
- All includes (`event.html`, `authors.html`, `subscribe.html`, `subscribe-main.html`, `head.html`, `header.html`, `footer.html`) are loaded as partials
- Layouts `page` and `home` exist in `_layouts/`

### What needs to be implemented

1. **`generate_standalone_pages()` function in `generator.rs`**: A new public function that takes `Vec<Page>`, a `LayoutEngine`, site context, and output directory, then renders each page and writes the HTML output file.

   For each page:
   - Resolve the layout from the page's front matter `layout` key
   - The page body contains Liquid template code (e.g., `{% for post in site.posts %}`) mixed with HTML and markdown. It must be rendered through the template engine FIRST (to evaluate Liquid), then the result is inserted into the layout via `{{ content }}`
   - Write the output to `<output_dir>/<slug>.html` (e.g., `_site/events.html`, `_site/index.html`)

2. **Template rendering of page body**: Unlike collection items where `html_content` is pre-rendered markdown, standalone pages have Liquid code in their body that must be evaluated at render time. The `content` field (raw markdown+Liquid) should be passed to `LayoutEngine::render_page()`, NOT the `html_content` field. The layout engine must:
   - Parse and render Liquid tags in the body (using the full site context)
   - Convert any remaining markdown to HTML (or the page body is already mostly HTML with inline Liquid)
   - Wrap the result in the layout

3. **Page context**: Each page needs a `page` namespace in the template context containing its front matter fields (title, description, image, layout, permalink). This is already handled by `render_page()` which builds the page context from front matter.

### Pages and their template requirements

| Page | Layout | Liquid features used |
|------|--------|---------------------|
| `index.md` | `home` | `site.data.events` with `where_exp`, `sort`; `site.podcast` with `sort`, `reverse`, `limit`; `site.data.sponsors` iteration; `site.books` with `where_exp`, `sort`; `site.posts` with `limit`; includes: `subscribe-main.html`, `event.html`, `authors.html` |
| `events.md` | `page` | `site.data.events` with `where_exp`, `sort`, `reverse`; includes: `event.html` |
| `articles.md` | `page` | `site.posts` iteration; `site.people` with `where` filter; includes: none directly (inline author lookup) |
| `books.md` | `page` | `site.books` with `where_exp`, `sort`, `reverse`; includes: `book.html`, `authors.html` |
| `people.md` | `page` | `site.people` iteration; `site.people.size` |
| `podcast.md` | `page` | `site.podcast` with `map`, `uniq`, `sort`, `reverse`, `where`; includes: `authors.html` |
| `tools.md` | `page` | `site.tools` iteration; includes: `authors.html` |
| `courses.md` | `page` | No Liquid (static content) |
| `slack.md` | `page` | includes: `subscribe.html` |
| `support.md` | `page` | No Liquid (static content) |

### Output file mapping

| Source | Output | URL |
|--------|--------|-----|
| `index.md` | `_site/index.html` | `/` |
| `events.md` | `_site/events.html` | `/events.html` |
| `articles.md` | `_site/articles.html` | `/articles.html` |
| `books.md` | `_site/books.html` | `/books.html` |
| `people.md` | `_site/people.html` | `/people.html` |
| `podcast.md` | `_site/podcast.html` | `/podcast.html` |
| `tools.md` | `_site/tools.html` | `/tools.html` |
| `courses.md` | `_site/courses.html` | `/courses.html` |
| `slack.md` | `_site/slack.html` | `/slack.html` |
| `support.md` | `_site/support.html` | `/support.html` |

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes with all new and existing tests
- [ ] A new `generate_standalone_pages()` function exists in `generator.rs`
- [ ] All 10 standalone pages generate HTML files in the output directory
- [ ] Each generated HTML file is wrapped in its correct layout (`home` for index, `page` for all others)
- [ ] `index.html` contains "Upcoming events" section with event data from `site.data.events`
- [ ] `index.html` contains "Latest podcast episodes" with links to podcast pages
- [ ] `index.html` contains "Our Sponsors" section with sponsor images/links
- [ ] `index.html` contains "Book of the week" section
- [ ] `index.html` contains "Latest articles" section with links to blog posts
- [ ] `events.html` contains both "Upcoming events" and "Past events" sections
- [ ] `articles.html` lists all posts with author links
- [ ] `books.html` has "Upcoming books" and "Archive" sections
- [ ] `people.html` lists all people with links to their pages
- [ ] `podcast.html` groups episodes by season
- [ ] `tools.html` lists tools with links
- [ ] `slack.html` renders with the subscribe include
- [ ] `courses.html` and `support.html` render their static content
- [ ] All generated HTML contains the expected `<html>`, `<head>`, `<body>` structure from the layout
- [ ] Speaker/author names in all pages link to `/people/SLUG.html`
- [ ] No Liquid template tags remain unrendered in the output (no `{{` or `{%` in final HTML)

## Verification Commands

```bash
export PATH="$HOME/.cargo/bin:/usr/bin:/bin:/usr/local/bin:$PATH"
cargo build
cargo clippy -- -D warnings
cargo test

# Integration tests should generate all pages to a temp dir and verify:
# - File existence
# - HTML structure
# - Key content presence
# - No unrendered Liquid tags
```

## Test Scenarios

### Unit: generate_standalone_pages function

- Call with an empty `Vec<Page>` -- returns `GenerationResult` with 0 generated, 0 skipped, 0 errors.
- Call with a minimal page (layout: `page`, body: `<h1>Hello</h1>`) -- generates one HTML file.
- Call with a page whose layout does not exist -- page is skipped (not an error).

### Unit: Page body Liquid rendering

- Create a page with body `{% for i in (1..3) %}{{ i }} {% endfor %}`. Verify the output contains `1 2 3`.
- Create a page with body `{{ page.title }}` and front matter `title: "Test"`. Verify output contains `Test`.
- Create a page with body `{{ site.name }}`. Verify output contains the site name from config.

### Integration: Generate all 10 real pages

- Load the real site data, build full site context with all collections (people, posts, books, podcast, courses, conferences, tools).
- Call `generate_standalone_pages()` with the real pages from `load_pages()`.
- Verify all 10 HTML files are created in the output directory.
- Verify no generation errors.

### Integration: index.html content verification

- Read the generated `index.html`.
- Verify it contains `<html` (from layout).
- Verify it contains "Upcoming events" or event titles.
- Verify it contains "Latest podcast episodes".
- Verify it contains "Our Sponsors".
- Verify it contains "Book of the week".
- Verify it contains "Latest articles".
- Verify no `{{` or `{%` template tags remain.

### Integration: events.html content verification

- Read the generated `events.html`.
- Verify it contains "Past events" heading.
- Verify it contains `<a href=` links (either registration or youtube).
- Verify it contains `/people/` links (speaker links).

### Integration: articles.html content verification

- Read the generated `articles.html`.
- Verify it lists blog post titles.
- Verify author names are linked to `/people/` pages.

### Integration: books.html content verification

- Read the generated `books.html`.
- Verify it contains "Archive" section.
- Verify book titles appear as links.

### Integration: people.html content verification

- Read the generated `people.html`.
- Verify it contains the people count (e.g., `{{ site.people.size }}`  rendered as a number).
- Verify it lists people names as links.

### Integration: podcast.html content verification

- Read the generated `podcast.html`.
- Verify it contains "Season #" headings.
- Verify episode titles appear as links.

### Integration: tools.html content verification

- Read the generated `tools.html`.
- Verify it lists tool names.

### Integration: Static pages (courses, slack, support)

- Read generated `courses.html` -- verify it contains "Courses".
- Read generated `slack.html` -- verify it contains "Slack" and the subscribe form markup.
- Read generated `support.html` -- verify it contains "Support DataTalks.Club".

### Integration: No unrendered Liquid in any page

- For each of the 10 generated HTML files, assert that the output does NOT contain `{%` or `{{` (indicating unrendered Liquid tags).

## Implementation Guidance

### 1. New function: `generate_standalone_pages()`

Add to `generator.rs`:

```rust
pub fn generate_standalone_pages(
    pages: &[Page],
    config: &SiteConfig,
    layout_engine: &LayoutEngine,
    site_context: &Object,
    output_dir: &Path,
) -> Result<GenerationResult, GeneratorError>
```

For each page:
- Get layout from `page.front_matter.get("layout")` -- skip if missing
- Pass `page.content` (raw Liquid+markdown body) to `layout_engine.render_page()`
- The output path is `output_dir / slug.html` (e.g., `_site/index.html`)
- Use parallel rendering with rayon (same pattern as `generate_collection_pages`)

### 2. Content rendering approach

The page body must be treated like a post body -- it contains Liquid template code that needs evaluation. Pass `page.content` (NOT `page.html_content`) to the layout engine. The layout engine's `render_page` already:
1. Renders the body content through the Liquid parser (evaluating `{% for %}`, `{% include %}`, etc.)
2. Wraps the result in the layout template

### 3. Page context construction

The `render_page` method already takes front matter as `&FrontMatter` and builds the `page.*` namespace. Pass the page's `front_matter` directly. The method also receives `site_context` for the `site.*` namespace.

### 4. Test fixture pattern

Follow the `LazyLock` shared fixture pattern from `integration_people.rs`:
- Create a shared fixture that loads ALL collections and data once
- Generate all 10 pages once in a `LazyLock<TempDir>`
- Individual tests read files from that temp dir

### 5. Markdown in page body

Some pages mix markdown with HTML and Liquid. After Liquid rendering, any markdown syntax should be converted to HTML. Check if `render_page` handles this or if the page body needs explicit markdown conversion after Liquid processing. The Jekyll behavior is: render Liquid first, then convert markdown. If the current `render_page` only handles Liquid, add a markdown conversion step for pages.

### 6. Kramdown attributes

Jekyll uses kramdown which supports `{:target="_blank"}` link attributes. The `events.md` page uses this syntax. If the markdown processor does not support kramdown attributes, these will appear as literal text in the output. For now, this is acceptable -- document it as a known limitation if it occurs. Do NOT block this issue on kramdown compatibility.

## Out of Scope

- Pagination (none of these pages use Jekyll pagination)
- Custom permalinks beyond what front matter specifies
- Kramdown attribute syntax support (nice-to-have, not required)
- RSS/Atom feed generation (issue 17)
- Sitemap generation (issue 16)
