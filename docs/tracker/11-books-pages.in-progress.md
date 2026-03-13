# Issue 11: Books Collection Pages

## Description

Generate HTML pages for `_books/` using the `book.html` layout. Each book gets a page at `/books/:title.html` with cover, title, authors, date range, description, links, and Q&A archive.

This is a page generation wiring issue following the same pattern as issues 09 and 10: load the books collection via `collection::load_collection("books", ...)`, build the site context, render each book through the `book.html` layout via `LayoutEngine::render_page`, and write HTML files to the output directory.

## Dependencies

- Issue 05 (collection loader) -- DONE
- Issue 08 (layout and includes) -- DONE
- Issue 09 (people pages -- established the `generate_collection_pages` pattern and site context building) -- DONE
- Issue 10 (blog posts -- established the `where` filter for author lookups) -- DONE

## Scope

### In scope

- A `generate_book_pages` function (or equivalent) that orchestrates:
  1. Loads the `_books/` collection using `load_collection("books", ...)`
  2. Loads the `_people/` collection (needed for `authors.html` include which resolves author short names to full names via `site.people`)
  3. Builds a site context object containing `site.people` (for author name resolution in the `authors.html` include)
  4. For each book, resolves the layout (`book` by default from `_config.yml` collection defaults, or explicit `layout` in front matter)
  5. Calls `LayoutEngine::render_page("book", content, front_matter, site_context)` for each book
  6. Writes the rendered HTML to `<output_dir>/books/<slug>.html`
- Title rendering: `{{ page.title }}`
- Authors byline via `{% include authors.html authors=page.authors %}` -- the include uses `site.people | where: "short", a | first` to look up author names and link to `/people/<short>.html`
- Date range: `{{ page.start | date_to_string }}` to `{{ page.end | date_to_string }}`
- Cover image: `<img class="img-border" src="/{{ page.cover }}" />`
- Description content: `{{ content }}` (the book's markdown body rendered to HTML)
- External links list: iteration over `page.links` array, each with `link.text` and `link.link`
- Q&A archive section (conditionally rendered when `page.archive` exists):
  - Iteration over `page.archive` array of thread objects
  - Each thread has `name`, `text`, and `replies` (array of `{name, text}` objects)
  - Thread text uses `{{ thread.text | newline_to_br | markdownify }}` filter chain
  - Reply text uses `{{ reply.text | newline_to_br | markdownify }}` filter chain
- **`newline_to_br` filter implementation**: This filter converts newline characters (`\n`) to `<br />` tags. It is used in the book layout but does NOT currently exist in the codebase. It must be implemented as a new Liquid filter (similar to the existing `markdownify` filter).
- Subscribe CTA include: `{% include subscribe.html subscribe="true" %}` (already handled by issue 08's includes system)
- Static participation instructions block (hardcoded in the layout template)

### Out of scope

- Full site build orchestration (issue 19)
- Other collection page generation (issues 12, 13, 14)
- Sitemap or RSS (issues 16, 17)
- JSON-LD schema for books (the `book.html` layout does not include a JSON-LD block, unlike `author.html` and `post.html`)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `cargo test` passes with all new and existing tests
- [ ] A `newline_to_br` Liquid filter is implemented that converts `\n` characters to `<br />` tags (matching Jekyll's `newline_to_br` behavior)
- [ ] A function exists that takes a site directory, config, layout engine, and output directory, and generates book HTML pages
- [ ] Running the function against the real `datatalksclub.github.io/` directory produces 99 HTML files in `<output>/books/`
- [ ] The generated HTML for `20201214-ml-bookcamp.html` contains:
  - The title "Machine Learning Bookcamp"
  - The author name "Alexey Grigorev" (resolved from `site.people` via the `authors.html` include)
  - A link to `/people/alexeygrigorev.html` for the author
  - The date range text containing "Dec 14, 2020" (start) and "Dec 18, 2020" (end) via `date_to_string`
  - The cover image path `/images/books/20201214-ml-bookcamp/cover.jpg`
  - The description content "Machine Learning Bookcamp: learn machine learning by doing projects"
  - External links: "Book's page on Manning" linking to `http://bit.ly/mlbookcamp`, "Book's page" linking to `https://mlbookcamp.com/`, "Book's GitHub repository" linking to `https://github.com/alexeygrigorev/mlbookcamp-code`
  - A "Questions and Answers" heading (from the archive section)
  - Archive thread names: "Vladimir Finkelshtein", "Wendy Mak", "Neal Lathia"
  - Archive reply names: "Alexey Grigorev" (as a reply within threads)
  - The subscribe CTA section (from the `subscribe.html` include)
- [ ] The generated HTML for `20210201-data-teams.html` contains:
  - The title "Data Teams"
  - The author "Jesse Anderson" resolved from people
  - Multiple Q&A threads with nested replies
- [ ] The Q&A archive text is processed through both `newline_to_br` and `markdownify` filters, so multi-line text renders with `<br />` tags and markdown formatting (bold, links, etc.)
- [ ] Books with empty `replies: []` arrays in archive threads do not crash and render no reply divs for those threads
- [ ] The participation instructions block is present ("To take part in the book of the week event:", Slack registration link, `#book-of-the-week` channel mention)
- [ ] The link to `/books.html` in the footer text is present ("the book of the week page")
- [ ] The link to `/slack.html` in the participation instructions is present
- [ ] At least 12 new tests are added

## Test Scenarios

### Unit: `newline_to_br` filter
- Input `"hello\nworld"` produces `"hello<br />\nworld"` (Jekyll keeps the original newline after the `<br />`)
- Input `"no newlines"` returns `"no newlines"` unchanged
- Input `""` (empty string) returns `""` unchanged
- Input `"line1\nline2\nline3"` produces two `<br />` tags
- Input with `\r\n` (Windows line endings) handles correctly

### Unit: `newline_to_br` + `markdownify` filter chain
- Input `"**bold**\nnew line"` processed through `newline_to_br` then `markdownify` produces HTML with `<strong>bold</strong>` and `<br />`
- Input with markdown links `"[text](url)\nnext"` produces working `<a>` tags and `<br />`

### Unit: Output path generation
- Given a book with slug `20201214-ml-bookcamp` and output dir `/tmp/site`, verify the output path is `/tmp/site/books/20201214-ml-bookcamp.html`

### Integration: Render a single book page
- Load the real `book.html` layout and includes from `datatalksclub.github.io/`
- Load the `20201214-ml-bookcamp` book from `_books/`
- Build a site context with `site.people` populated from `_people/`
- Render the book and verify the output contains:
  - The title "Machine Learning Bookcamp"
  - The cover image path
  - The author name and link
  - The date range
  - External links
  - Q&A archive threads and replies

### Integration: Book with Q&A archive containing nested replies
- Render `20201214-ml-bookcamp` (which has threads with multiple replies)
- Verify the archive section contains thread author names
- Verify nested replies appear inside `book-archive-reply` divs
- Verify that the `newline_to_br | markdownify` filter chain is applied to thread and reply text

### Integration: Book with empty replies array
- Render a book where a thread has `replies: []`
- Verify no `book-archive-reply` divs appear for that thread
- Verify no errors or panics

### Integration: Generate all books to output directory
- Run the book generation function against `datatalksclub.github.io/`
- Verify 99 HTML files are written to `<output>/books/`
- Verify each file is non-empty
- Spot-check 3 known books for expected content (ml-bookcamp, data-teams, reinforcement-learning)

### Integration: Author resolution from site.people
- Render `20201214-ml-bookcamp` with `site.people` populated
- Verify the author "Alexey Grigorev" is resolved from the `alexeygrigorev` short name
- Verify the author links to `/people/alexeygrigorev.html`

### Integration: External links rendering
- Render `20201214-ml-bookcamp` and verify all 3 external links are present with correct text and URLs
- Verify links have `target="_blank"` attribute

### Edge case: Book with no archive
- If a book has no `archive` field in front matter, the Q&A section should not appear (the template uses `{% if page.archive %}`)
- No errors or empty "Questions and Answers" heading should appear

### Edge case: Single author vs multiple authors
- Verify a book with a single author in the `authors` array renders correctly
- If any book has multiple authors, verify all are listed and linked

## Output Verification

After building, inspect the generated HTML:

1. `<output>/books/20201214-ml-bookcamp.html`:
   - Cover image: `<img class="img-border" src="/images/books/20201214-ml-bookcamp/cover.jpg" />`
   - Title: "Machine Learning Bookcamp"
   - Author byline with link to `/people/alexeygrigorev.html`
   - Date range: "14 Dec 2020" to "18 Dec 2020"
   - Three external links with `target="_blank"`
   - Q&A archive with 14+ threads
   - Nested replies within threads
   - Subscribe CTA section
   - Participation instructions with `/slack.html` link

2. `<output>/books/20210201-data-teams.html`:
   - Title: "Data Teams"
   - Author: "Jesse Anderson"
   - Multiple Q&A threads
   - Description content about creating value with data

3. `<output>/books/20210111-reinforcement-learning.html`:
   - Title: "Reinforcement Learning"
   - Author: "Phil Winder"
   - Long Q&A archive section

4. Compare generated HTML structure against `datatalksclub.github.io/_layouts/book.html` to ensure layout fidelity -- verify the CSS classes (`content-book`, `content-book-image-container`, `content-book-description`, `book-archive-thread`, `book-archive-reply`) are present in the output.

## Implementation Notes

- Reuse the existing `generate_collection_pages` function from `src/generator.rs` -- books follow the same pattern as people and posts. The function already handles layout resolution, rendering, and file writing.
- The site context needs `site.people` for the `authors.html` include. Reuse the existing `build_people_array` function from `generator.rs`.
- The `newline_to_br` filter should be implemented as a new file `src/template/filters/newline_to_br.rs` following the same pattern as `markdownify.rs`. Register it in the template engine alongside the other filters.
- The book layout chains `newline_to_br | markdownify` -- the `newline_to_br` runs first (converting `\n` to `<br />\n`), then `markdownify` processes the result as markdown (which will preserve the `<br />` tags since they are valid HTML within markdown).
- The `archive` front matter field is a YAML array of objects. Each object has `name` (string), `text` (string), and `replies` (array of `{name, text}` objects). The YAML-to-Liquid conversion in `yaml_to_liquid` should handle this nested structure automatically.
- Output path: books go to `<output>/books/<slug>.html` based on the collection configuration.
