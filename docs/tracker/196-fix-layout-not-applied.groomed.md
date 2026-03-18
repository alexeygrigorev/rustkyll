# Issue 196: Fix layout/template not applied (773 pages)

## Checklist Category

**Layout/template not applied** -- 773 pages

## Problem

773 pages render without their layout template. Output has raw content (h1, p, ul) at root level instead of inside the proper html/head/body structure.

Breakdown by site:
- opensource-guide (337): Translated pages (ar/, es/, fa/, etc.) -- layout not resolved
- jekyll-docs-docs (109): Various pages missing layout
- documentation-theme-jekyll (97): Complex Liquid features in templates failing silently
- DTC/docs (57): Data file references in templates failing
- choosealicense.com (55): Layout not applied
- just-the-docs (47): Gem-based theme layouts not resolved
- alexeygrigorev/snippets (17): Include files not found
- academicpages (16): Gem-based theme
- so-simple-theme (11): Gem-based theme
- little-book-of-metals-ru (9): Layout inheritance broken
- muan-blog (7): Specific pages (index.html, notes.html, film.html, de-DE/)
- beautiful-jekyll (5): Gem-based theme
- government-github (6): Layout not applied

## Goal

Fix layout resolution so all pages render with their correct layout structure (html > head + body).

## Dependencies

- Issue 171 (fix layout not applied: liquid conditionals) -- done
- Issue 174 (fix config defaults layout assignment) -- done
- These previous fixes addressed some layout issues but 773 pages remain.

## Sub-tasks

### Sub-task 1: Investigation (do this FIRST)

For each affected site, determine the specific failure mode:

1. **opensource-guide (337)**: Read `docs/comparison/dom-details/opensource-guide.txt`. Note: translated pages like `ar/best-practices/index.html` show `child[1]: tag_name_differs - expected: 'head', actual: 'div'`. Check if these pages have a front matter `layout` field. Check if the layout file exists. Build the site and inspect stderr for layout resolution warnings.

2. **muan-blog (7)**: The `notes.html` page shows raw Liquid template code (`{% assign notes = ...`). This means Liquid processing itself is failing, not just layout. The `film.html` and `index.html` pages show `child[1]: tag_name_differs - expected: 'head', actual: 'div'`.

3. **documentation-theme-jekyll (97)**: Check if these use `_data/` file references in Liquid that fail.

4. **Gem-based themes** (just-the-docs 47, academicpages 16, so-simple-theme 11, beautiful-jekyll 5): Check if `_layouts/` from the gem are being resolved.

5. Document each failure mode with specific root cause before writing code.

### Sub-task 2: Fix the top failure mode(s)

Based on investigation, fix the highest-impact failure modes. Each distinct root cause may warrant its own sub-issue if complex.

### Sub-task 3: Fix remaining failure modes or create sub-issues

If a failure mode requires significant work (e.g., full gem theme support), create a new `.todo.md` issue for it rather than trying to fix everything in one issue.

## TDD Test Scenarios

### Test 1: Layout applied to page with front matter layout field (write FIRST, verify it fails)

```rust
#[test]
fn test_layout_applied_to_translated_page() {
    // Setup: Create a minimal site with:
    //   _layouts/default.html containing <html><head>...</head><body>{{ content }}</body></html>
    //   A page ar/index.html with front matter: layout: default
    //   The page content: <div>Hello</div>
    //
    // Assert: Generated HTML starts with <html><head>...</head><body>
    //   and contains the page content inside <body>.
    //
    // This tests the pattern seen in opensource-guide translated pages.
    // Verify it FAILS before implementing.
}
```

### Test 2: Liquid processing completes even with complex template features

```rust
#[test]
fn test_liquid_processes_assign_and_loops() {
    // Setup: Create a page with Liquid template code:
    //   {% assign items = site.posts | sort: "date" | reverse %}
    //   {% for item in items %}...{% endfor %}
    //   layout: default
    //
    // Assert: Generated HTML does NOT contain raw Liquid tags like {% assign
    //   and the layout is applied (html > head > body structure).
    //
    // This tests the muan-blog notes.html failure pattern.
    // Verify it FAILS before implementing.
}
```

### Test 3: Layout inheritance chain works

```rust
#[test]
fn test_layout_inheritance_chain() {
    // Setup: Create layouts:
    //   _layouts/default.html: <html><body>{{ content }}</body></html>
    //   _layouts/page.html: layout: default, content: <main>{{ content }}</main>
    //   _layouts/article.html: layout: page, content: <article>{{ content }}</article>
    //   A page with layout: article
    //
    // Assert: Full chain applied: html > body > main > article > content
    //
    // Verify it FAILS before implementing if chain is broken.
}
```

### Test 4 (integration, #[ignore]): Build opensource-guide and verify layouts

```rust
#[test]
#[ignore]
fn test_opensource_guide_layouts_applied() {
    // Build opensource-guide site
    // Check ar/best-practices/index.html has <head> as first child
    // Check body element exists
}
```

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with new tests for each failure mode
- [ ] Investigation documents specific root cause for each site's layout failures
- [ ] Pages that have `layout:` in front matter and a corresponding `_layouts/*.html` file render with that layout
- [ ] No pages output raw Liquid template code (like `{% assign ...`)
- [ ] Layout inheritance chains (page -> default) work correctly
- [ ] For failure modes that cannot be fixed in this issue (e.g., full gem theme _layouts resolution), new `.todo.md` issues are created
- [ ] The number of layout-not-applied pages decreases by at least 50% (from 773)
