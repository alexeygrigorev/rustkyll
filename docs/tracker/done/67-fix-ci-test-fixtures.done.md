# Issue 67: Fix CI -- create test fixtures and make tests pass without DTC site

## Problem

CI fails with 56 test failures because tests depend on `datatalksclub.github.io/` which is gitignored and not present in CI. See GitHub Actions run failures and Copilot PR #2.

The 56 failures break down into two categories:

1. **Library tests** (in `src/*.rs`) that hardcode `site_dir()` pointing to `datatalksclub.github.io/` -- these include tests in `collection.rs`, `config.rs`, `data.rs`, `feed.rs`, `generator.rs`, `sitemap.rs`, and `template/layout.rs`
2. **Integration tests** (in `tests/integration_*.rs`) that depend on the real DTC site for rendering full pages -- these include tests in `integration_books.rs`, `integration_build.rs`, `integration_context.rs`, `integration_events.rs`, `integration_jsonld.rs`, `integration_pages.rs`, `integration_people.rs`, `integration_performance.rs`, `integration_podcast.rs`, `integration_posts.rs`, and `integration_templates.rs`

## Goal

Make all tests pass in CI without the DTC site directory present. Tests that require the real site should gracefully skip in CI.

## Reference

Copilot PR #2 (https://github.com/alexeygrigorev/rustkyll/pull/2) has a working approach:
- Creates `tests/fixtures/` with a minimal spec-compliant Jekyll site
- Redirects lib tests from real DTC site to fixtures
- Adds skip guards to integration tests that need real sites
- Review this PR with `gh pr diff 2`, take what is useful, adapt as needed

## Approach

1. Review Copilot PR #2 diff using `gh pr diff 2`
2. Either cherry-pick/merge it directly (if quality is acceptable) or manually implement the approach
3. Create `tests/fixtures/` -- a minimal but complete Jekyll site with all required collections, layouts, includes, data files, and pages
4. Redirect all `site_dir()` / `data_dir()` helpers in lib tests (`src/*.rs`) to point to `tests/fixtures/` instead of `datatalksclub.github.io/`
5. Adjust count assertions in lib tests to match fixture sizes (e.g., `>= 424` people becomes `>= 2`)
6. Add `if !site_dir().exists() { return; }` skip guards to all integration tests that depend on the real DTC site
7. Ensure all tests pass both locally (with DTC site) AND in CI (without it)

## Dependencies

None

## Acceptance Criteria

All of these must be met. Do not silently drop any.

### Fixture site (`tests/fixtures/`)

- [ ] `tests/fixtures/` directory exists and is committed to the repo
- [ ] Contains a valid `_config.yml` with all 6 collections (books, people, conferences, podcast, courses, tools), defaults, excludes, and standard fields (url, name, title, permalink, baseurl)
- [ ] Contains `_layouts/` with all 6 layouts: `home.html`, `page.html`, `post.html`, `book.html`, `author.html`, `podcast.html`
- [ ] Contains `_includes/` with all include files referenced by tests (at minimum: `anchor.html`, `authors.html`, `book.html`, `breadcrumbs.html`, `charts.html`, `event.html`, `faq-accordion.html`, `footer.html`, `head.html`, `header.html`, `mathjax.html`, `meta.html`, `navigation.html`, `pagination.html`, `related-posts.html`, `seo-tag.html`, `social.html`, `subscribe-main.html`, `subscribe.html`, `youtube.html`, and `course-structured-data/` subdir)
- [ ] Contains collection content: at least 2 people, 2 books, 2 podcast episodes, 3 posts (including one with tags for group_by tests), 1+ conferences, 1+ courses, 1+ tools
- [ ] Contains `_data/` with: `events.yml`, `events_extra.yml`, `header.yml`, `navigation.yml`, `sponsors.yml`, and `faqs/` subdirectory with FAQ YAML files
- [ ] Contains at least 5 root `.md` pages (e.g., `index.md`, `about.md`, `blog.md`, `books.md`, etc.)
- [ ] Fixture site is minimal -- not a copy of the full DTC site, just enough content to exercise all code paths tested by lib tests

### Library tests (`src/*.rs`)

- [ ] All `site_dir()` / `data_dir()` helpers in test modules of `src/collection.rs`, `src/config.rs`, `src/data.rs`, `src/feed.rs`, `src/generator.rs`, `src/sitemap.rs`, and `src/template/layout.rs` point to `tests/fixtures/` instead of `datatalksclub.github.io/`
- [ ] Count assertions are adjusted to match fixture sizes (e.g., people `>= 2`, books `>= 2`, podcast `>= 2`, posts `== 3`, events `> 0`, sitemap entries `> 5`)
- [ ] All lib tests pass with the fixture data -- no test failures, no panics
- [ ] The skip guards that previously existed in `src/data.rs` (checking `if !dir.exists()`) are removed since fixtures are always present

### Integration tests (`tests/integration_*.rs`)

- [ ] Every integration test function that depends on `datatalksclub.github.io/` has a skip guard: `if !site_dir().exists() { return; }` at the top
- [ ] Skip guards are added to ALL affected integration test files: `integration_books.rs`, `integration_build.rs`, `integration_context.rs`, `integration_events.rs`, `integration_jsonld.rs`, `integration_pages.rs`, `integration_people.rs`, `integration_performance.rs`, `integration_podcast.rs`, `integration_posts.rs`, `integration_templates.rs`
- [ ] Integration tests still pass locally when `datatalksclub.github.io/` is present
- [ ] Integration tests skip gracefully (not fail) in CI when the directory is absent

### CI pipeline

- [ ] `cargo test --verbose` in CI produces 0 failures (all lib tests pass, all integration tests skip gracefully)
- [ ] `cargo clippy -- -D warnings` passes in CI
- [ ] `cargo fmt --check` passes in CI
- [ ] The full CI pipeline (`.github/workflows/ci.yml`) goes green

### Code quality

- [ ] No reduction in test coverage for code that CAN be tested with fixtures -- lib tests must still exercise the same code paths, just with smaller data
- [ ] Fixture layouts/includes contain enough content to satisfy all existing test assertions (e.g., `schema.org` in `post.html`, `mc-embedded-subscribe-form` in `subscribe.html`, author lookup in `book.html`)
- [ ] No unwrap-on-None or unwrap-on-Err panics in library code exposed by fixture tests

## Test Scenarios

### Scenario: Lib tests pass with fixture data

- Run `cargo test --lib` -- all tests in `src/` modules pass using `tests/fixtures/`
- `collection.rs` tests: load people (>= 2), books (>= 2), podcast (>= 2), posts (== 3 or adjusted count), conferences, courses, tools
- `config.rs` tests: parse fixture `_config.yml`, verify collections, defaults, excludes, extras
- `data.rs` tests: load fixture `_data/`, verify events, events_extra, header, navigation, sponsors, faqs
- `feed.rs` tests: generate feed from fixture posts
- `generator.rs` tests: build site context from fixtures, verify posts/books/events arrays populated
- `sitemap.rs` tests: generate sitemap entries from fixture collections (> 5 entries)
- `template/layout.rs` tests: load 6 layouts, load 20+ includes, render each layout type

### Scenario: Integration tests skip gracefully in CI

- In a fresh checkout (no `datatalksclub.github.io/`), run `cargo test` -- all integration tests return early without failure
- Verify by checking test output: integration tests should show as "passed" (since skip guard returns cleanly), not "failed" or "ignored"

### Scenario: Integration tests still work locally

- With `datatalksclub.github.io/` present locally, run `cargo test` -- integration tests execute fully and pass
- No regressions in local test behavior

### Scenario: CI pipeline is green

- Push to a branch, open a PR, verify GitHub Actions CI job passes
- All 4 CI steps pass: build, test, clippy, fmt

### Scenario: Fixture site is self-consistent

- The fixture `_config.yml` references collections that have corresponding `_<collection>/` directories
- Layout defaults in `_config.yml` match layouts in `_layouts/`
- Include files referenced in layouts exist in `_includes/`
- People referenced in books/podcast `authors` fields exist in `_people/`
