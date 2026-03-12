# Issue 08: Layout and Includes System

## Description

Implement the Jekyll layout wrapping and includes system. Layouts wrap content (`{{ content }}`). Includes are reusable snippets (`{% include file.html param=value %}`). This issue connects the template engine (issues 06, 07) to actual page rendering.

## Dependencies

- Issue 06 (template engine core) -- done
- Issue 07 (template filters) -- done

## Scope

### Layout System

- Load all 6 layouts from `_layouts/` directory: `home.html`, `page.html`, `post.html`, `book.html`, `podcast.html`, `author.html`
- Layout wrapping: given rendered page content and a layout name, substitute `{{ content }}` in the layout template with the rendered content
- Populate `page.*` variables from front matter (title, description, image, authors, links, etc.)
- Populate `site.*` variables (url, name, twitter, collections like `site.people`/`site.posts`/`site.podcast`/`site.books`, data like `site.data.navigation`/`site.data.events`/`site.data.header`, config values)
- Populate `site.time` with the current build time (used in event.html and various pages for past/future comparisons)
- Support layout chaining (a layout referencing another layout via front matter). Note: the DataTalks.Club site does NOT use layout chaining (all 6 layouts are standalone `<html>` documents), but the system should support it for correctness
- Provide a `LayoutEngine` or similar struct that takes: layout name, rendered content string, page context, site context -- and produces the final wrapped HTML

### Include System

- Load all includes from `_includes/` directory (15 files: `head.html`, `header.html`, `footer.html`, `subscribe.html`, `subscribe-main.html`, `authors.html`, `book.html`, `event.html`, `youtube.html`, `anchor.html`, `mathjax.html`, `charts.html`, `related-posts.html`, `faq-accordion.html`, plus the `course-structured-data/` directory with 6 HTML files)
- Register includes as partials in the `liquid` parser using `EagerCompiler<InMemorySource>` via `ParserBuilder::partials()`
- **Critical: Use the Jekyll-compatible include tag**, not the stdlib one. The `liquid-lib` crate provides `liquid_lib::jekyll::IncludeTag` which already supports Jekyll's syntax:
  - Unquoted filenames: `{% include head.html %}`
  - Parameters with `=`: `{% include subscribe.html subscribe="true" %}`
  - Variable parameters: `{% include authors.html authors=page.authors %}`
  - Access via `include.param` inside the included template
- The current `TemplateEngine::builder()` uses `ParserBuilder::with_stdlib()` which registers the stdlib `IncludeTag` (expects quoted filenames: `{% include "file.html" %}`). The engineer must replace it with `liquid_lib::jekyll::IncludeTag` which accepts the Jekyll unquoted syntax
- Support nested includes: `event.html` includes `authors.html`, `book.html` includes `authors.html`, `head.html` includes `mathjax.html` and `charts.html`

### How to Replace the Include Tag

The `ParserBuilder::with_stdlib()` call registers `stdlib::IncludeTag`. Since tag registration works by name ("include"), registering `liquid_lib::jekyll::IncludeTag` after `with_stdlib()` should override it (last registration wins, or use `.tag()` to replace). The engineer should verify this works, or alternatively build the parser manually without calling `with_stdlib()`, registering all needed stdlib components plus the Jekyll include tag. The key is that:

1. `{% include head.html %}` parses (unquoted filename)
2. `{% include subscribe.html subscribe="true" %}` parses (parameter with `=` not `:`)
3. `{% include authors.html authors=page.authors %}` parses (variable value, not just literals)
4. Inside the included template, `include.subscribe`, `include.authors`, etc. are accessible

### Integration Points

- Modify `TemplateEngine` to accept a path to the `_includes/` directory and load all include files as partials
- Provide a method or separate `LayoutEngine` struct that:
  1. Reads layout files from `_layouts/`
  2. For a given page: renders the page content through the template engine (with page/site context), then wraps it in the layout (substituting `{{ content }}`)
  3. The layout rendering itself also goes through the template engine (so `{% include %}` tags in layouts work)
- The `TemplateEngine` must be built with partials so that `{% include %}` tags resolve at render time

## What is NOT in Scope

- Generating actual collection pages (that's issues 09-14)
- Full site build orchestration (that's issue 19)
- JSON-LD structured data correctness (that's issue 18) -- the layouts contain JSON-LD but it only needs to render without errors, not be semantically validated
- Sitemap/RSS generation (issues 16-17)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] All 6 layout files from `datatalksclub.github.io/_layouts/` can be loaded and parsed without errors
- [ ] All include files from `datatalksclub.github.io/_includes/` can be loaded and registered as partials
- [ ] `{% include head.html %}` works with unquoted filenames (Jekyll syntax)
- [ ] `{% include subscribe.html subscribe="true" %}` works with named parameters using `=`
- [ ] `{% include authors.html authors=page.authors %}` works with variable values passed as parameters
- [ ] Inside included templates, `include.param` resolves to the passed parameter value
- [ ] Nested includes work: rendering `event.html` (which includes `authors.html`) produces correct output
- [ ] Layout wrapping works: given content "Hello" and layout `home.html`, the output contains "Hello" wrapped in the home layout HTML structure
- [ ] `page.*` variables from front matter are accessible in both layouts and includes
- [ ] `site.*` variables are accessible in both layouts and includes
- [ ] The `content` variable in layouts contains the rendered page body
- [ ] `cargo test` passes with at least 15 new tests covering layout loading, include resolution, parameter passing, nested includes, and layout wrapping
- [ ] Rendering a layout that uses includes, filters, conditionals, and for loops (e.g., `post.html` with its JSON-LD section) produces output without errors

## Test Scenarios

### Unit: Layout loading
- Load all 6 layouts from the actual `_layouts/` directory, verify each parses without error
- Attempt to load a non-existent layout, verify graceful error
- Load a layout and verify `{{ content }}` can be substituted

### Unit: Include loading and registration
- Load all includes from `_includes/` directory and register as partials
- Verify the partials count matches the number of include files
- Attempt to include a non-existent file, verify error message is clear

### Unit: Jekyll include syntax compatibility
- Parse and render `{% include head.html %}` (unquoted filename, no params)
- Parse and render `{% include subscribe.html subscribe="true" %}` (string param with `=`)
- Parse and render `{% include youtube.html video_id=page.ids.youtube %}` (variable param)
- Parse and render `{% include event.html event=event speakers=false %}` (multiple params, boolean value)
- Verify `include.subscribe` resolves to `"true"` inside `subscribe.html`
- Verify `include.authors` resolves to the array passed from the calling template

### Unit: Nested includes
- Render `book.html` include which itself includes `authors.html` -- verify both levels resolve
- Render `event.html` include which includes `authors.html` -- verify nested resolution
- Render `head.html` which conditionally includes `mathjax.html` and `charts.html`

### Integration: Layout wrapping with includes
- Wrap simple content in `home.html` layout with a site context -- verify output contains head, header, content, footer HTML
- Wrap content in `page.html` layout -- verify the subscribe include renders inside the layout
- Wrap content in `post.html` layout with page front matter (title, authors, date) -- verify the title, author links, and JSON-LD section render
- Wrap content in `author.html` layout with page context including social links -- verify twitter/linkedin/github links render
- Wrap content in `book.html` layout with page context including cover, authors, dates -- verify book metadata renders

### Integration: Real layout rendering with site data
- Render `header.html` include with `site.data.navigation.top` and `site.data.header.announcement` -- verify navigation links appear
- Render `footer.html` include with `site.github.repository_url` -- verify footer content
- Render `post.html` layout with `site.people` collection to verify author lookup (`site.people | where: "short", a | first`) works inside layouts

### Edge cases
- Layout with no `{{ content }}` marker -- content should still render (layout is used as-is, content may be lost but no crash)
- Empty content wrapped in a layout -- verify layout renders with empty content area
- Include with no parameters -- verify `include` object exists but is empty
- Include parameter with special characters in value
- Layout referencing `page.*` variables that don't exist in front matter -- verify graceful nil handling (no crash, renders empty)

## Output Verification

Since this issue establishes the rendering pipeline but does not yet generate full pages (that requires collections from issues 09-14), output verification should focus on:

- [ ] Manually construct a test that renders a mock blog post through `post.html` layout with realistic page/site context, and verify the output HTML contains: `<html`, `<head>`, `<title>`, the post title, author links, `{{ content }}` substituted, the subscribe form, and the footer
- [ ] Manually construct a test that renders a mock author page through `author.html` layout, and verify social links, articles section structure, and subscribe form appear in the output
- [ ] Verify the output HTML is well-formed (no unresolved `{{ }}` or `{% %}` tags in output, no liquid errors)

## Technical Notes

### Include Tag Override Strategy

The `liquid-lib` crate's `liquid_lib::jekyll::IncludeTag` already implements the full Jekyll include syntax. The code at line 43-46 of `jekyll/include_tag.rs` shows it uses `expect_identifier()` for the filename (which handles unquoted names like `head.html`) and `=` for parameter assignment.

The stdlib `IncludeTag` at `stdlib/tags/include_tag.rs` is the standard Liquid one (requires quoted filenames, uses `:` for params).

Since `ParserBuilder::with_stdlib()` registers the stdlib version, the engineer must ensure the Jekyll version takes precedence. Options:
1. Call `.tag(liquid_lib::jekyll::IncludeTag)` after `with_stdlib()` -- if the registry replaces by name, this works
2. Build the parser without `with_stdlib()`, manually registering everything needed plus the Jekyll include tag
3. Test which approach the `liquid` crate supports

### Partials Loading

The `liquid` crate's partials system uses `EagerCompiler<InMemorySource>`:

```rust
use liquid::partials::{EagerCompiler, InMemorySource};

let mut partials = EagerCompiler::<InMemorySource>::empty();
for (filename, contents) in include_files {
    partials.add(filename, contents);
}

let parser = ParserBuilder::with_stdlib()
    .tag(liquid_lib::jekyll::IncludeTag)
    .partials(partials)
    .build()?;
```

Include files should be registered by their filename (e.g., `"head.html"`, `"header.html"`). For the `course-structured-data/` subdirectory, register as `"course-structured-data/filename.html"`.

### Layout Wrapping Flow

```
1. Parse page content (markdown -> HTML via issue 03)
2. Render page content through template engine (resolve any Liquid tags in the content itself)
3. Look up layout from page front matter (e.g., layout: post)
4. Build context: { page: {front_matter...}, site: {config + collections + data...}, content: rendered_content }
5. Render the layout template with this context
6. If layout has a parent layout (layout chaining), repeat steps 3-5 with the output as new content
7. Return final HTML
```
