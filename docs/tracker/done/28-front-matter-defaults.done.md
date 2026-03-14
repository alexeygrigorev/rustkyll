# Issue 28: Generalized Front Matter Defaults

## Problem

Rustkyll only reads `layout` from the `defaults` config entries. Jekyll defaults can set any front matter key (e.g., `comments: true`, `author_profile: true`, `read_time: true`, `sidebar`, `toc`, etc.). Currently, `DefaultValues` is a struct with only a `layout: String` field, and `default_layout_for()` only returns the layout. Any other default values specified in `_config.yml` are silently ignored.

## Requirements

- Change `DefaultValues` from a struct with only a `layout` field to a generic key-value map that captures all values
- Apply all key-value pairs from matching defaults to pages/collection items, not just `layout`
- Front matter on individual pages/items takes precedence over defaults (defaults are only applied when the key is absent from the page's own front matter)
- Match defaults by scope `type` and `path`, consistent with Jekyll behavior
- Later (more specific) defaults override earlier ones, consistent with Jekyll's ordering
- The existing `default_layout_for()` method should continue to work (or be updated to extract `layout` from the generic map)
- All existing tests must continue to pass

## Scope

- `src/config.rs` -- change `DefaultValues` to use a generic map (e.g., `HashMap<String, serde_yaml::Value>` or `serde_yaml::Mapping`); update `default_layout_for()` accordingly
- `src/config.rs` -- add a method like `defaults_for(type_name, path)` that returns all matching default values merged together
- `src/generator.rs` -- in `resolve_layout()` and/or `generate_collection_pages()`, apply all defaults (not just layout) to the page front matter before rendering
- Tests in both modules

## Dependencies

- No strict dependencies. The `DefaultConfig`, `DefaultScope`, and `DefaultValues` structs already exist and parse from `_config.yml`. This issue extends them to be more general.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] `DefaultValues` accepts arbitrary key-value pairs, not just `layout`
- [ ] A config with `defaults: [{scope: {type: "posts"}, values: {layout: "post", comments: true, read_time: true}}]` correctly parses all three values
- [ ] When rendering a post that has no `comments` key in its own front matter, the default `comments: true` is applied and available as `page.comments` in templates
- [ ] When a post has `comments: false` in its own front matter, the default is NOT applied -- front matter wins
- [ ] `default_layout_for()` (or its replacement) still correctly returns the layout from defaults
- [ ] `resolve_layout()` still works correctly with the new structure
- [ ] Scope matching works: a default scoped to `type: "posts"` does not apply to `type: "people"`
- [ ] Path-based scope matching works: a default scoped to `path: "special/"` only applies to items whose source path starts with that prefix
- [ ] When multiple defaults match (e.g., one with empty path and one with specific path), later/more-specific entries override earlier ones for the same key
- [ ] The DTC site's existing defaults (layout for people, books, podcast) continue to work correctly
- [ ] Default values are available in templates via `page.<key>` (e.g., `page.author_profile`, `page.read_time`)

## Test Scenarios

### Unit: Config parsing of generalized defaults

- Parse YAML with a default that sets `layout`, `comments`, and `read_time` -- verify all three values are captured
- Parse YAML with a default that sets only `layout` -- verify backward compatibility (layout is accessible)
- Parse YAML with a default that sets only non-layout keys (e.g., `author: "Default Author"`) -- verify they are captured
- Parse YAML with multiple defaults for the same type with different paths -- verify all are parsed
- Parse YAML with an empty `values:` section -- verify it parses without error (empty map)

### Unit: default_layout_for backward compatibility

- Using the DTC config, verify `default_layout_for("people")` still returns `Some("author")`
- Using the DTC config, verify `default_layout_for("books")` still returns `Some("book")`
- Using the DTC config, verify `default_layout_for("podcast")` still returns `Some("podcast")`
- Verify `default_layout_for("courses")` still returns `None`

### Unit: defaults_for method (all matching defaults merged)

- Config has default `{scope: {type: "posts"}, values: {layout: "post", comments: true}}` -- calling `defaults_for("posts", "")` returns map with both `layout` and `comments`
- Config has default for type "posts" and another for type "people" -- calling `defaults_for("posts", "")` returns only the posts defaults
- Config has two defaults for "posts": one with empty path setting `layout: "post"`, another with `path: "special/"` setting `layout: "special-post"` -- calling `defaults_for("posts", "special/my-post")` returns `layout: "special-post"` (path-specific wins)
- Config has no defaults -- calling `defaults_for("posts", "")` returns an empty map

### Unit: Applying defaults to front matter

- Item has no front matter keys, defaults set `layout: "post"` and `comments: true` -- after applying defaults, both keys are present
- Item has `layout: "custom"` in front matter, defaults set `layout: "post"` -- after applying, `layout` is `"custom"` (front matter wins)
- Item has `comments: false` in front matter, defaults set `comments: true` -- after applying, `comments` is `false` (front matter wins)
- Item has no front matter, no defaults match -- front matter stays empty

### Unit: Scope matching

- Default scoped to `type: "posts", path: ""` matches a post at any path
- Default scoped to `type: "posts", path: "2021/"` matches a post with source path `_posts/2021-01-01-my-post.md` (path prefix match within the collection directory)
- Default scoped to `type: "posts", path: "2021/"` does NOT match a post with source path `_posts/2020-12-01-other.md`
- Default scoped to `type: "people"` does NOT match a post

### Integration: DTC site defaults

- Load the real DTC config, verify that all three existing defaults (people->author, books->book, podcast->podcast) work with the generalized system
- Build a post from the DTC site, verify `resolve_layout()` still returns the correct layout

### Integration: Template rendering with defaults

- Create a minimal site with config defaults setting `comments: true` for posts
- Create a post with no `comments` in its front matter
- Create a layout template that renders `{{ page.comments }}`
- Build the site and verify the generated HTML contains `true`

## References

- Issue #22 compatibility research, gap #10
- Jekyll docs: https://jekyllrb.com/docs/configuration/front-matter-defaults/
