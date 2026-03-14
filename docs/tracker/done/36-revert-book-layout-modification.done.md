# Issue 36: Revert Book Layout Modification and Generate JSON-LD in Rustkyll

## Problem

Issue #18 added a JSON-LD `Book` + `BreadcrumbList` block directly into `datatalksclub.github.io/_layouts/book.html`. This modifies the reference Jekyll site, which should remain untouched. The JSON-LD generation should be handled purely by rustkyll's rendering code (post-processing or injection during page generation).

Currently, `book.html` has 70 lines of JSON-LD Liquid template code appended (lines 71-139) that were not in the original layout. The original layout ends at `{% include footer.html %}` / `</body>` with no JSON-LD block.

## Requirements

1. Revert `datatalksclub.github.io/_layouts/book.html` to its original state (run `cd datatalksclub.github.io && git checkout HEAD -- _layouts/book.html`)
2. Implement JSON-LD Book + BreadcrumbList generation in rustkyll's Rust code, injecting it into the rendered HTML output for book pages
3. The generated HTML output must still contain the same JSON-LD structured data as before the revert:
   - `@type: Book` with name, description, image, url, datePublished, author array (with Person objects), publisher
   - `@type: BreadcrumbList` with Home > Books > [Book Title] breadcrumb
4. No files in `datatalksclub.github.io/` should be modified by rustkyll development going forward
5. All existing JSON-LD tests must continue to pass

## Implementation Approach

The JSON-LD injection should happen as a post-processing step after the Liquid template is rendered but before writing the HTML file. When the page layout is `book`, rustkyll should:

1. Build a JSON-LD object from the page's front matter fields (`title`, `description`, `cover`, `start`, `authors`, `url`) and site config (`url`, `name`, `people` collection)
2. Serialize it to a `<script type="application/ld+json">` block
3. Inject the block before the closing `</body>` tag in the rendered HTML

This keeps the source layout files clean and makes JSON-LD generation a rustkyll feature rather than a template concern.

## Dependencies

- Issue #18 (JSON-LD schemas) -- DONE. This issue reverts the layout modification made during #18.
- Issue #11 (books pages) -- DONE. Book page rendering must be working.

## Acceptance Criteria

- [ ] `datatalksclub.github.io/_layouts/book.html` matches its original committed state (no JSON-LD block, no diff against `HEAD` in the datatalksclub.github.io repo)
- [ ] Running `cd datatalksclub.github.io && git diff HEAD -- _layouts/book.html` produces no output
- [ ] `rustkyll build` on the datatalksclub.github.io site still produces book pages with JSON-LD structured data
- [ ] The generated book page HTML contains a `<script type="application/ld+json">` block with:
  - [ ] `@type: Book` with `name` matching the book's title
  - [ ] `@type: Book` with `url` matching `site.url + page.url`
  - [ ] `@type: Book` with `author` array containing `Person` objects with `name` fields
  - [ ] `@type: Book` with `publisher` containing `Organization` with site name
  - [ ] `@type: BreadcrumbList` with 3 items: Home, Books, [Book Title]
- [ ] The JSON-LD output is valid JSON (parseable by a JSON parser, not malformed due to template rendering artifacts)
- [ ] No other files in `datatalksclub.github.io/` are modified
- [ ] `cargo test` passes with tests covering the book JSON-LD injection
- [ ] `cargo clippy -- -D warnings` passes
- [ ] The JSON-LD injection mechanism is generic enough to support other page types in the future (not hardcoded to book layout only)

## Test Scenarios

### Unit: JSON-LD generation for book pages

- Given a book page with title "Machine Learning Bookcamp", authors ["alexeygrigorev"], cover "images/books/ml-bookcamp.jpg", start date "2021-11-15", verify the generated JSON-LD contains `@type: Book` with the correct name, image URL, datePublished, and author
- Given a book page with no description field, verify the JSON-LD omits the `description` property (not null, not empty string)
- Given a book page with multiple authors, verify the author array contains a Person entry for each
- Verify the BreadcrumbList has exactly 3 items with positions 1, 2, 3 and correct names/URLs

### Unit: JSON-LD injection into HTML

- Given rendered HTML with a `</body>` tag, verify the JSON-LD `<script>` block is injected before `</body>`
- Given rendered HTML without a `</body>` tag, verify graceful handling (append at end or skip)
- Verify the injected JSON-LD is valid JSON by parsing it

### Integration: Full book page build

- Build the datatalksclub.github.io site with rustkyll
- Pick 3 book pages from the output and verify each contains a valid JSON-LD block
- Compare the JSON-LD content against what the old (modified) layout would have produced -- the fields and structure should match
- Verify book pages still render all other content correctly (title, authors, cover image, links, archive Q&A)

### Regression: Other page types unaffected

- Verify post pages still have their existing JSON-LD (Article schema) if they had it before
- Verify podcast pages still have their existing JSON-LD (PodcastEpisode schema) if they had it before
- Verify author pages are not affected by the book JSON-LD changes

### Integration: Layout file integrity

- After reverting, confirm `_layouts/book.html` has exactly 70 lines (the original) and contains no `schema.org` or `ld+json` references
- Run `cd datatalksclub.github.io && git status` and confirm `_layouts/book.html` is not listed as modified

## Output Verification

When reviewing this issue, the PM must:

1. Build the site: `cargo run --release -- build --source datatalksclub.github.io/ --destination /tmp/rustkyll-output`
2. Inspect at least 2 generated book pages (e.g., `/tmp/rustkyll-output/books/some-book.html`)
3. Verify the JSON-LD block is present and contains the expected schema.org types
4. Verify the JSON-LD is valid JSON (copy the block and parse it)
5. Verify `datatalksclub.github.io/_layouts/book.html` has no JSON-LD content

## Notes

- The revert command is: `cd datatalksclub.github.io && git checkout HEAD -- _layouts/book.html`
- The original book.html has no JSON-LD -- it is a pure layout template (HTML + Liquid)
- Other layouts (post, author, podcast) may also have JSON-LD in their template files from issue #18 -- check if those need similar treatment in future issues
- The JSON-LD injection approach should be designed so it can be extended to other page types without modifying their layout files
