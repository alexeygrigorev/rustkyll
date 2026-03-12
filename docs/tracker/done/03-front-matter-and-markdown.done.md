# Issue 03: Front Matter and Markdown Parsing

## Description

Parse Markdown files with YAML front matter (delimited by `---`). Split front matter from content body. Parse front matter as YAML into a flexible key-value structure. Convert Markdown body to HTML.

## Dependencies

- Issue 01 (project setup) -- must be `.done.md` first

## Scope

- `src/frontmatter.rs` module (or `src/content/frontmatter.rs` if the engineer prefers a `content` submodule)
- Parse `---` delimited YAML front matter
- Handle files with no front matter (treat entire file as markdown body, empty front matter)
- Markdown-to-HTML conversion using `pulldown-cmark` or `comrak`
- Support for `<!--more-->` excerpt separator
- Unit tests with representative content from the Jekyll site

### Out of Scope

- Collection loading (Issue 05)
- Template rendering or layout wrapping
- Permalink generation

## Technical Details

### Front Matter Format

The Jekyll site uses `---` delimited YAML front matter at the top of `.md` files. The front matter block starts with `---` on the first line (optionally preceded by a blank line inside the block) and ends with a second `---`. Everything between is YAML. Everything after is the Markdown body.

### YAML Value Types Found in the Site

The parser must handle all of these YAML value types observed in `datatalksclub.github.io/`:

| Type | Example | Where Found |
|------|---------|-------------|
| String | `title: "Machine Learning Bookcamp"` | All collections |
| Unquoted string | `layout: post` | All collections |
| Single-quoted string | `title: 'Customer Segmentation...'` | Posts |
| Date | `start: 2020-12-14 00:00:00` | Books, courses |
| Date string | `date: '2020-11-29'` | Posts |
| Inline list | `authors: [alexeygrigorev]` | Books |
| Block list | `authors:\n- nishantmohan` | Posts |
| Nested objects | `links:\n  - text: ...\n    link: ...` | Books, podcast |
| Deeply nested | `archive:\n- name: ...\n  replies:\n  - name: ...` | Books |
| Nested object with sub-keys | `ids:\n  anchor: ...\n  youtube: ...` | Podcast |
| Boolean (implicit) | N/A but YAML supports it | Edge case |
| Null / empty | `description:` (with no value) | Some files |

### Recommended Data Model

```rust
use std::collections::HashMap;

/// Represents a parsed document with front matter and body
pub struct Document {
    pub front_matter: FrontMatter,
    pub content: String,      // raw markdown body
    pub excerpt: Option<String>, // content before <!--more-->, if present
}

/// YAML front matter as a flexible key-value map
/// Use serde_yaml::Value or a similar flexible enum to handle
/// the variety of YAML types (strings, lists, nested maps, dates, etc.)
pub type FrontMatter = HashMap<String, serde_yaml::Value>;
```

The engineer may choose a different representation (e.g., wrapping `serde_yaml::Value` in a newtype), but the key requirement is that the front matter structure is flexible enough to hold any YAML value type without needing to know the schema in advance.

### Excerpt Handling

Three posts in `_posts/` use `<!--more-->` as an excerpt separator. When present:
- `excerpt` should contain the markdown content before the separator
- `content` should contain the full markdown body (including the part before the separator)
- When absent, `excerpt` should be `None`

### Markdown Conversion

- Convert markdown body to HTML
- Must handle: headings, paragraphs, links, images, bold/italic, blockquotes, code blocks, lists, horizontal rules, HTML passthrough (the site embeds raw HTML like `<figure>`, `<div>`, etc.)
- `pulldown-cmark` is recommended (lightweight, well-maintained). `comrak` is also acceptable (GFM support).
- Raw HTML embedded in markdown (e.g., `<figure>`, `<img>`, `{% include ... %}`) must be passed through unchanged. Liquid tags like `{% include youtube.html video_id="..." %}` will be handled later by the template engine; the markdown parser should not strip them.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes with no warnings
- [ ] `cargo fmt --check` shows no formatting issues
- [ ] A public function parses a string containing `---` delimited front matter + markdown body and returns a structured result (front matter map + content string + optional excerpt)
- [ ] Front matter is parsed into a flexible key-value structure that preserves YAML types (strings, lists, nested maps, dates)
- [ ] Files with no front matter are handled gracefully (empty front matter map, entire content as body)
- [ ] Files with empty front matter (`---\n---\ncontent`) are handled (empty map, content as body)
- [ ] The `<!--more-->` excerpt separator is detected and the excerpt is extracted when present
- [ ] Markdown body is converted to HTML correctly (headings, links, images, lists, code blocks, blockquotes)
- [ ] Raw HTML in markdown is passed through to the output unchanged
- [ ] Liquid-like tags (e.g., `{% include ... %}`) in markdown are not stripped or mangled
- [ ] `cargo test` passes with all tests (minimum 12 tests)
- [ ] No `unwrap()` in library code -- all errors use `Result` types

## Test Scenarios

### Unit: Front matter splitting

- Parse a file with standard front matter (`---\ntitle: Hello\n---\nBody`), verify title extracted and body correct
- Parse a file with no front matter (just markdown), verify empty front matter and full body returned
- Parse a file with empty front matter (`---\n---\nBody`), verify empty map and body returned
- Parse a file with `---` appearing in the body (e.g., a horizontal rule), verify it is not confused with front matter delimiters
- Parse a file where front matter has a blank line after the opening `---` (as seen in the real site files), verify it still parses correctly

### Unit: YAML value types

- Parse front matter with a simple string value (`title: "Test"`), verify string extracted
- Parse front matter with an inline list (`authors: [alice, bob]`), verify list of two items
- Parse front matter with a block list (`tags:\n- analytics\n- clustering`), verify list of two items
- Parse front matter with a nested object (`ids:\n  anchor: ABC\n  youtube: XYZ`), verify nested map
- Parse front matter with a date value (`start: 2020-12-14 00:00:00`), verify it is preserved (as string or datetime)
- Parse front matter with a null/empty value (`description:`), verify it is handled as null/None

### Unit: Excerpt extraction

- Parse content with `<!--more-->` separator, verify excerpt contains text before separator
- Parse content without `<!--more-->`, verify excerpt is None
- Parse content where `<!--more-->` appears at the very beginning, verify excerpt is empty string or None

### Unit: Markdown to HTML conversion

- Convert a heading (`## Hello`) to `<h2>Hello</h2>`
- Convert a paragraph with **bold** and *italic* to correct HTML
- Convert a link `[text](url)` to `<a href="url">text</a>`
- Convert a code block to `<pre><code>` tags
- Convert a blockquote to `<blockquote>` tags
- Verify raw HTML (`<figure><img src="..."></figure>`) is passed through unchanged
- Verify Liquid-like tags (`{% include youtube.html video_id="..." %}`) are preserved in output

### Integration: Real content from the Jekyll site

- Parse a representative `_posts/` file (with front matter containing title, authors list, tags list, date, and `<!--more-->` separator), verify all fields extracted correctly and HTML generated
- Parse a representative `_people/` file (with short, title, picture, linkedin), verify all fields extracted
- Parse a representative `_books/` file (with nested archive/replies structure), verify deeply nested YAML is preserved
- Parse a representative `_podcast/` file (with nested ids and links maps), verify nested structure correct
