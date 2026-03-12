# Issue 05: Collection Loader

## Description

Load Jekyll collections (`_people/`, `_books/`, `_podcast/`, `_posts/`, `_courses/`, `_conferences/`, `_tools/`) from the filesystem. Each collection item has front matter metadata and a content body. Also load standalone pages (*.md in root).

## Dependencies

- Issue 02 (config parsing -- need collection definitions) -- DONE
- Issue 03 (front matter parsing) -- DONE

## Scope

- `src/collection.rs` module
- `CollectionItem` struct with: slug, front matter fields, raw content, HTML content, URL
- Load all items for a collection directory
- Generate URLs from permalink patterns (`:collection`, `:title`)
- Handle `_posts/` naming convention (`YYYY-MM-DD-title.md`)
- Load standalone `.md` pages from root directory
- Skip files whose names start with `_` (e.g. `_template.md`, `_s12e08.md`)
- Unit tests with actual content files from `datatalksclub.github.io/`

## Key Design Decisions

### CollectionItem struct

Must contain at minimum:
- `slug: String` -- the filename stem (e.g. `16rahuljain` from `16rahuljain.md`, `segmentation` from `2020-11-29-segmentation.md`)
- `front_matter: FrontMatter` -- parsed YAML front matter (reuse `HashMap<String, serde_yaml::Value>` from issue 03)
- `content: String` -- raw markdown body
- `html_content: String` -- markdown converted to HTML
- `excerpt: Option<String>` -- content before `<!--more-->` if present
- `url: String` -- generated URL path (e.g. `/people/16rahuljain.html`)
- `date: Option<String>` -- extracted date for posts (from `YYYY-MM-DD-title.md` filename or front matter)
- `collection_name: String` -- which collection this item belongs to (e.g. `people`, `posts`)

### URL Generation

Permalink patterns from `_config.yml`:
- Collections use `/:collection/:title.html` (e.g. `/people/16rahuljain.html`, `/books/20201214-ml-bookcamp.html`)
- Posts use `/blog/:title.html` (from the global `permalink` config field, e.g. `/blog/segmentation.html`)
- `:collection` is replaced with the collection name (e.g. `people`, `books`, `podcast`)
- `:title` is replaced with the slug (filename without `.md` extension; for posts, the slug is the part after `YYYY-MM-DD-`)
- Standalone pages use `/:title.html` or their front matter `permalink` if specified

### Posts Naming Convention

Posts follow `YYYY-MM-DD-title.md`:
- Date is extracted from filename: `2020-11-29-segmentation.md` -> date `2020-11-29`, slug `segmentation`
- The front matter `date` field may also be present and should be stored, but the filename is the canonical source for the date
- If the filename does not match the date pattern, treat the whole stem as the slug with no date

### Files to Skip

- Files whose name starts with `_` (templates and drafts: `_template.md`, `_s12e08.md`, `_theme-park-crowd-modeling...md`)
- Non-`.md` files (though currently all collection files are `.md`)

### Standalone Pages

Root-level `.md` files like `index.md`, `articles.md`, `books.md`, `events.md`, `people.md`, `podcast.md`, `courses.md`, `slack.md`, `support.md`, `tools.md`. These are NOT part of any collection. They should be loaded as a separate list. Skip `README.md`.

### Error Handling

- Use `Result` types, no `unwrap()` in library code
- If a single file fails to parse, log/collect the error but continue loading the rest of the collection
- Return a clear error type (e.g. `CollectionError`) that wraps IO and parse errors

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `src/collection.rs` exists and is registered in `src/lib.rs`
- [ ] `CollectionItem` struct has fields: slug, front_matter, content, html_content, excerpt, url, date, collection_name
- [ ] A function loads all items from a collection directory (e.g. `load_collection("people", &config)`)
- [ ] A function loads standalone pages from the root directory
- [ ] Underscore-prefixed files are skipped (e.g. `_template.md`)
- [ ] Post filenames are parsed to extract date and slug (`2020-11-29-segmentation.md` -> date=`2020-11-29`, slug=`segmentation`)
- [ ] URLs are generated from permalink patterns: `:collection` and `:title` are substituted
- [ ] Posts use the global permalink pattern (`/blog/:title.html`)
- [ ] Collections use their configured permalink pattern (`/:collection/:title.html`)
- [ ] Loading the real `_people/` directory returns 424+ items (427 total minus templates/underscore files)
- [ ] Loading the real `_books/` directory returns 98+ items (99 minus `_template.md`)
- [ ] Loading the real `_podcast/` directory returns 193+ items (196 minus 3 underscore-prefixed files)
- [ ] Loading the real `_posts/` directory returns 55 items
- [ ] Each loaded item has non-empty slug, content (or at least non-panicking), and a valid URL
- [ ] `cargo test` passes with 15+ tests covering all scenarios below

## Test Scenarios

### Unit: Post filename parsing
- Parse `2020-11-29-segmentation.md` -> date=`2020-11-29`, slug=`segmentation`
- Parse `2021-01-01-ml-deployment-lambda.md` -> date=`2021-01-01`, slug=`ml-deployment-lambda`
- Parse `non-date-filename.md` -> date=None, slug=`non-date-filename`

### Unit: URL generation from permalink patterns
- Pattern `/:collection/:title.html` with collection=`people`, title=`alexeygrigorev` -> `/people/alexeygrigorev.html`
- Pattern `/:collection/:title.html` with collection=`books`, title=`20201214-ml-bookcamp` -> `/books/20201214-ml-bookcamp.html`
- Pattern `/blog/:title.html` with title=`segmentation` -> `/blog/segmentation.html`

### Unit: Skip underscore-prefixed files
- Given a list of filenames including `_template.md` and `alexeygrigorev.md`, only the non-underscore file is loaded

### Integration: Load real _people/ collection
- Load from `datatalksclub.github.io/_people/`
- Verify count is 424+ (skipping `_template.md` and any other underscore files)
- Verify a known item (e.g. slug=`alexeygrigorev`) has expected front matter fields (title, short, picture)
- Verify URL is `/people/alexeygrigorev.html`

### Integration: Load real _books/ collection
- Load from `datatalksclub.github.io/_books/`
- Verify count is 98+
- Verify a known item (e.g. slug=`20201214-ml-bookcamp`) has title `Machine Learning Bookcamp`
- Verify URL is `/books/20201214-ml-bookcamp.html`

### Integration: Load real _podcast/ collection
- Load from `datatalksclub.github.io/_podcast/`
- Verify count is 193+
- Verify underscore files (`_template.md`, `_s12e08.md`, `_theme-park-crowd-modeling...md`) are excluded

### Integration: Load real _posts/ directory
- Load from `datatalksclub.github.io/_posts/`
- Verify count is 55
- Verify a known post (e.g. `2020-11-29-segmentation.md`) has slug=`segmentation`, date=`2020-11-29`
- Verify URL is `/blog/segmentation.html` (uses global permalink, not collection permalink)

### Integration: Load real _courses/, _conferences/, _tools/
- Verify each loads the expected number of items (1, 2, 2 respectively, minus any underscore files)

### Integration: Load standalone pages
- Load root `.md` files from `datatalksclub.github.io/`
- Verify `index.md` is loaded with title `Welcome to DataTalks.Club`
- Verify `README.md` is excluded
- Verify count is 10 (articles, books, courses, events, index, people, podcast, slack, support, tools)

### Edge cases
- Empty collection directory (or nonexistent) returns empty vec, not an error
- File with no front matter still loads (empty front matter map, full content as body)
- File with front matter but no body content loads correctly
