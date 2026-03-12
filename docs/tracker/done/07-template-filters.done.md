# Issue 07: Template Filters (Custom Jekyll Filters)

## Description

Implement the 6 custom Liquid filters used by the Jekyll site that are NOT provided by the `liquid` crate's stdlib or `liquid-lib`'s jekyll feature. These filters are needed before layout/include rendering (Issue 08) can work.

Issue 06 already integrated the `liquid` crate which provides all standard Liquid filters (where, sort, reverse, map, first, last, size, join, uniq, compact, slice, append, prepend, default, strip, strip_html, strip_newlines, truncate, newline_to_br, split, plus, minus, times, divided_by, modulo, replace, remove, date, escape, downcase, upcase, concat) plus Jekyll-specific filters from `liquid-lib` (slugify, push, pop, unshift, array_to_sentence_string).

This issue covers ONLY the 6 missing custom filters:
1. `where_exp` -- filter arrays using a Liquid expression (like `where` but with arbitrary expressions)
2. `jsonify` -- convert a value to its JSON representation
3. `date_to_string` -- format a date as "DD Mon YYYY" (e.g., "01 Jan 2024")
4. `date_to_xmlschema` -- format a date as ISO 8601 (e.g., "2024-01-01T00:00:00+00:00")
5. `markdownify` -- convert Markdown text to HTML
6. `relative_url` -- prepend the site's baseurl to a path

## Dependencies

- Issue 06 (template engine core) -- DONE

## Scope

### In scope
- Create `src/template/filters/mod.rs` -- module for custom filters, re-exports
- Create `src/template/filters/where_exp.rs` -- `WhereExp` filter
- Create `src/template/filters/jsonify.rs` -- `Jsonify` filter
- Create `src/template/filters/date_to_string.rs` -- `DateToString` filter
- Create `src/template/filters/date_to_xmlschema.rs` -- `DateToXmlschema` filter
- Create `src/template/filters/markdownify.rs` -- `Markdownify` filter
- Create `src/template/filters/relative_url.rs` -- `RelativeUrl` filter
- Register all 6 filters in `TemplateEngine::builder()` so they are available in all templates
- Add any needed dependencies to `Cargo.toml` (e.g., `serde_json` for jsonify, `chrono` for date filters)
- Comprehensive unit tests for each filter (25+ tests total)
- Integration tests verifying filters work within template rendering via `TemplateEngine`

### Out of scope
- Filters already provided by the `liquid` crate or `liquid-lib` (no need to reimplement)
- Layout chain rendering and includes (Issue 08)
- Anything not actually used by the templates in `datatalksclub.github.io/`

## Filter Specifications

### 1. `where_exp` -- Expression-based array filtering

**Jekyll syntax:** `array | where_exp: "item", "item.field op value"`

**Actual usage patterns found in the site:**
- `site.posts | where_exp: "post", "post.authors contains page.short"` (author.html)
- `site.data.events | where_exp: "event", "event.draft != true"` (events.md, index.md, author.html)
- `site.data.events | where_exp: "event", "event.time > site.time"` (events.md, index.md)
- `site.data.events | where_exp: "event", "event.speakers contains page.short"` (author.html)
- `site.books | where_exp: "book", "book.end > site.time"` (books.md, index.md)
- `site.books | where_exp: "book", "book.authors contains page.short"` (author.html)
- `page.tracks | where_exp: "track", "track.end >= site.time"` (conferences)
- `page.tracks | where_exp: "track", "track.date >= site.time"` (conferences)

**Expression operators used:** `contains`, `!=`, `>`, `<`, `>=`, `<=`

**Implementation approach:** This is the most complex filter. It takes two string arguments: the variable name and a Liquid expression. For each element in the input array, it must evaluate the expression with the element bound to the variable name. The expression must also have access to the template's runtime context (e.g., `page.short`, `site.time`).

The filter must:
- Accept two string arguments: variable name and expression string
- For each array element, bind the element to the variable name in a temporary context
- Parse and evaluate the expression string as a Liquid condition
- Keep elements where the expression evaluates to truthy
- Have access to the surrounding template runtime context (for variables like `site.time`, `page.short`)

**Note:** Since the `liquid` crate's `Filter::evaluate` receives a `&dyn Runtime`, the filter has access to the runtime context. The expression evaluation will need to use `liquid-core`'s expression parsing capabilities, or implement a simpler expression evaluator that handles the operators actually used (`contains`, `!=`, `>`, `<`, `>=`, `<=`, `==`).

### 2. `jsonify` -- JSON serialization

**Jekyll syntax:** `value | jsonify`

**Actual usage patterns found in the site:**
- `page.title | jsonify` -- string to JSON string (adds quotes): `"My Title"`
- `page.tags | jsonify` -- array to JSON array: `["tag1","tag2"]`
- `page.description | jsonify` -- string with possible special chars to escaped JSON string
- `author.bio_short | strip_html | jsonify` -- chained after strip_html
- `site.url | jsonify` -- URL string to JSON string
- `page.date | date_to_xmlschema | jsonify` -- chained after date filter

**Implementation:** Convert the Liquid value to its JSON representation. Strings get quoted and escaped, arrays become JSON arrays, objects become JSON objects, numbers stay as-is, nil becomes `null`, booleans become `true`/`false`.

### 3. `date_to_string` -- Human-readable date formatting

**Jekyll syntax:** `date_value | date_to_string`

**Output format:** `"DD Mon YYYY"` (e.g., `"01 Jan 2024"`, `"15 Mar 2023"`)

**Actual usage patterns found in the site:**
- `page.date | date_to_string` (post.html)
- `book.start | date_to_string`, `book.end | date_to_string` (book.html, books.md, author.html)
- `event.time | date_to_string`, `event.end | date_to_string` (event.html include)
- `track.date | date_to_string` (conferences)

**Input formats to handle:** The dates in Jekyll come from YAML front matter as either:
- Date strings: `"2024-01-15"`, `"2024-01-15 10:00:00"`, `"2024-01-15T10:00:00+00:00"`
- Liquid date scalars (from `serde_yaml` date parsing)

**Implementation:** Parse the input as a date string, format as `"%d %b %Y"`. Handle both date-only and datetime inputs.

### 4. `date_to_xmlschema` -- ISO 8601 date formatting

**Jekyll syntax:** `date_value | date_to_xmlschema`

**Output format:** `"2024-01-15T00:00:00+00:00"` (ISO 8601 with timezone)

**Actual usage in the site (only in post.html JSON-LD):**
- `page.datepublished | date_to_xmlschema | jsonify`
- `page.date | date_to_xmlschema | jsonify`

**Implementation:** Parse the input date string, format as ISO 8601. Default timezone to UTC (+00:00) when not specified.

### 5. `markdownify` -- Markdown to HTML conversion

**Jekyll syntax:** `text | markdownify`

**Actual usage patterns found in the site:**
- `faq.answer | markdownify` (faq-accordion.html include)
- `faq.answer | markdownify | strip` (faq-accordion.html, for JSON-LD)
- `thread.text | newline_to_br | markdownify` (book.html)
- `reply.text | newline_to_br | markdownify` (book.html)

**Implementation:** Use the `pulldown-cmark` crate (already in dependencies) to convert Markdown text to HTML. Should handle inline Markdown (bold, italic, links, code) and block elements (paragraphs, lists).

### 6. `relative_url` -- Prepend baseurl

**Jekyll syntax:** `path | relative_url`

**Actual usage patterns found in the site:**
- `author.picture | relative_url` (post.html, podcast.html JSON-LD)
- `page.image | relative_url` (podcast.html JSON-LD)

**Implementation:** Prepend `site.baseurl` to the input path. The DataTalks.Club site has no `baseurl` set in `_config.yml`, so the default behavior is to prepend nothing (or `/` if the path doesn't start with `/`). The filter should:
- Read `site.baseurl` from the runtime context if available
- If baseurl is empty/nil, return the input path as-is (possibly ensuring it starts with `/`)
- If baseurl is set, prepend it to the path

## Acceptance Criteria

- [ ] `src/template/filters/mod.rs` exists and re-exports all 6 filter structs
- [ ] `src/template/filters/where_exp.rs` implements `WhereExp` using the liquid-core `Filter` trait and derive macros
- [ ] `src/template/filters/jsonify.rs` implements `Jsonify` using the liquid-core `Filter` trait and derive macros
- [ ] `src/template/filters/date_to_string.rs` implements `DateToString` using the liquid-core `Filter` trait and derive macros
- [ ] `src/template/filters/date_to_xmlschema.rs` implements `DateToXmlschema` using the liquid-core `Filter` trait and derive macros
- [ ] `src/template/filters/markdownify.rs` implements `Markdownify` using the liquid-core `Filter` trait and derive macros
- [ ] `src/template/filters/relative_url.rs` implements `RelativeUrl` using the liquid-core `Filter` trait and derive macros
- [ ] All 6 filters are registered in `TemplateEngine::builder()` via `.filter(...)` calls
- [ ] `TemplateEngine::new()` includes all custom filters and can parse/render templates using them
- [ ] `where_exp` correctly filters arrays using `contains`, `!=`, `>`, `<`, `>=`, `<=` operators
- [ ] `where_exp` has access to the runtime context (can reference variables like `site.time`, `page.short`)
- [ ] `jsonify` correctly serializes strings (with quotes and escaping), arrays, objects, numbers, booleans, and nil
- [ ] `date_to_string` formats dates as "DD Mon YYYY" (e.g., "01 Jan 2024")
- [ ] `date_to_string` handles date-only strings ("2024-01-15") and datetime strings ("2024-01-15 10:00:00")
- [ ] `date_to_xmlschema` formats dates as ISO 8601 with timezone (e.g., "2024-01-15T00:00:00+00:00")
- [ ] `markdownify` converts Markdown to HTML (inline and block elements)
- [ ] `relative_url` prepends baseurl from context, or returns path as-is when no baseurl is set
- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `cargo test` passes with 25+ new tests covering all filters
- [ ] All existing tests from Issue 06 continue to pass

## Test Scenarios

### Unit: `where_exp` filter
- Filter array of objects with `!= true` expression (draft filtering): input `[{draft: true, name: "a"}, {draft: false, name: "b"}]`, expression `"item.draft != true"` -- verify only `{name: "b"}` returned
- Filter with `contains` expression: input `[{authors: ["alice", "bob"]}, {authors: ["carol"]}]`, expression `"item.authors contains \"alice\""` -- verify first item returned
- Filter with `>` comparison on dates/numbers: input `[{time: 10}, {time: 20}]`, expression `"item.time > 15"` -- verify `{time: 20}` returned
- Filter with `>=` comparison: verify inclusive boundary works
- Filter with `<` comparison: verify less-than works
- Filter with `<=` comparison: verify inclusive boundary works
- Filter empty array -- verify returns empty array, no error
- Filter with expression referencing runtime context variable -- verify context access works
- Chaining: `array | where_exp: "x", "expr1" | where_exp: "x", "expr2"` -- verify both filters apply (used in events.md for draft + time filtering)

### Unit: `jsonify` filter
- String input: `"hello"` -> `"\"hello\""` (JSON-quoted)
- String with special chars: `"He said \"hi\""` -> properly escaped JSON string
- Integer input: `42` -> `"42"`
- Float input: `3.14` -> `"3.14"`
- Boolean input: `true` -> `"true"`
- Nil input: -> `"null"`
- Array input: `["a", "b"]` -> `"[\"a\",\"b\"]"`
- Object input: `{name: "Alice"}` -> `"{\"name\":\"Alice\"}"`
- Empty string: `""` -> `"\"\""`

### Unit: `date_to_string` filter
- Date-only string: `"2024-01-15"` -> `"15 Jan 2024"`
- Datetime string: `"2024-03-22 10:00:00"` -> `"22 Mar 2024"`
- ISO datetime: `"2024-12-01T14:30:00+00:00"` -> `"01 Dec 2024"`
- All 12 months: verify correct month abbreviation for at least Jan, Mar, Jun, Sep, Dec
- Day with leading zero: `"2024-01-01"` -> `"01 Jan 2024"`

### Unit: `date_to_xmlschema` filter
- Date-only string: `"2024-01-15"` -> `"2024-01-15T00:00:00+00:00"`
- Datetime string: `"2024-03-22 10:00:00"` -> contains `"2024-03-22T10:00:00"`
- Already ISO format: pass through correctly

### Unit: `markdownify` filter
- Bold text: `"**bold**"` -> contains `"<strong>bold</strong>"`
- Italic text: `"*italic*"` -> contains `"<em>italic</em>"`
- Link: `"[text](url)"` -> contains `"<a href=\"url\">text</a>"`
- Inline code: `` "`code`" `` -> contains `"<code>code</code>"`
- Paragraph wrapping: `"hello"` -> `"<p>hello</p>\n"` (pulldown-cmark wraps in p tags)
- Plain text with no markdown: passes through (wrapped in p tags)

### Unit: `relative_url` filter
- Path without leading slash: `"images/photo.jpg"` -> `"/images/photo.jpg"` (when no baseurl)
- Path with leading slash: `"/images/photo.jpg"` -> `"/images/photo.jpg"` (unchanged)
- With baseurl set in context: `"images/photo.jpg"` with baseurl `"/blog"` -> `"/blog/images/photo.jpg"`
- Empty/nil baseurl: behaves same as no baseurl

### Integration: Filters in template rendering via TemplateEngine
- Render `{{ items | where_exp: "item", "item.draft != true" | size }}` with mixed draft/non-draft items -- verify correct count
- Render `{{ page.title | jsonify }}` -- verify JSON-quoted string output
- Render `{{ page.date | date_to_string }}` -- verify formatted date in rendered output
- Render `{{ page.date | date_to_xmlschema | jsonify }}` -- verify chained filter output (ISO date wrapped in JSON quotes)
- Render `{{ text | markdownify }}` -- verify HTML output in rendered template
- Render `{{ page.image | relative_url }}` -- verify path in rendered output
- Realistic template: render a snippet mimicking post.html JSON-LD with `jsonify` and `date_to_xmlschema` chained -- verify valid JSON-LD fragment

### Edge cases
- `where_exp` on non-array input -- verify graceful handling (return empty array or error)
- `jsonify` on deeply nested structure -- verify correct JSON
- `date_to_string` on invalid date string -- verify graceful error handling (not panic)
- `markdownify` on empty string -- verify no error
- `markdownify` on string that is already HTML -- verify it passes through (pulldown-cmark treats HTML as raw)
- `relative_url` on empty string -- verify no error

## Notes for the engineer

1. **Filter implementation pattern:** Follow the same pattern used by `liquid-lib`'s filters. See `liquid-lib`'s `jekyll/slugify.rs` for a filter with arguments, and `stdlib/filters/html.rs` for simpler no-argument filters. The key derive macros are: `FilterParameters`, `ParseFilter`, `FilterReflection`, `FromFilterParameters`, `Display_filter`, and the `Filter` trait.

2. **`where_exp` is the hardest filter.** The challenge is evaluating an arbitrary Liquid expression for each array element. Options:
   - Parse the expression string into an AST using a small custom expression evaluator that handles the 6 operators used (`contains`, `!=`, `>`, `<`, `>=`, `<=`, `==`). This is simpler and sufficient for the site's actual usage.
   - Alternatively, construct a small Liquid template like `{% if EXPR %}true{% endif %}` and evaluate it for each element. This piggybacks on the liquid crate's parser but may have performance implications.
   - The expression evaluator approach is recommended for clarity and testability.

3. **`jsonify` implementation:** Use `serde_json` crate. Convert Liquid values to `serde_json::Value` first, then serialize. Add `serde_json` to `Cargo.toml`.

4. **Date filter implementation:** Use the `chrono` crate for date parsing and formatting. Add `chrono` to `Cargo.toml`. Parse input strings with multiple format attempts (date-only, datetime with space separator, ISO 8601).

5. **`markdownify` implementation:** The `pulldown-cmark` crate is already in `Cargo.toml`. Use `pulldown_cmark::Parser` and `pulldown_cmark::html::push_html()`.

6. **`relative_url` implementation:** The filter needs access to the runtime context to read `site.baseurl`. The `Filter::evaluate` method receives `&dyn Runtime` which provides variable lookup. Use `runtime.get(&[Scalar::new("site"), Scalar::new("baseurl")])` or similar to read the baseurl.

7. **Registration:** Add all 6 filters to `TemplateEngine::builder()` in `engine.rs`:
   ```rust
   liquid::ParserBuilder::with_stdlib()
       // existing jekyll filters
       .filter(liquid_lib::jekyll::Slugify)
       .filter(liquid_lib::jekyll::Push)
       // ...
       // new custom filters
       .filter(filters::Jsonify)
       .filter(filters::DateToString)
       // etc.
   ```

8. **Module structure:** Add `pub mod filters;` to `src/template/mod.rs`. The filters module should re-export the public filter structs.
