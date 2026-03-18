# Issue 195: Fix SEO meta tag differences (og:, twitter:, description)

## Checklist Category

**SEO meta tag differences (og:, twitter:, description)** -- 2236 pages

## Problem

2236 pages have SEO meta tag differences. Breakdown by site:
- muan-blog (2163): description/og:description empty instead of using post content, og:url has `.html` extension, datetime format in templates differs
- opensource-guide (28): meta tag ordering, og:locale, article:published_time differences
- choosealicense.com (16): meta tag content differences
- jekyll-docs-docs (14): meta tag content differences
- government-github (11): meta tag content differences
- DTC (2): title truncation stripping words containing "Data", og:image date in path differs
- aihero (2): meta tag differences

## Goal

Match jekyll-seo-tag meta tag output exactly: same tags, same order, same content.

## Dependencies

- Issue 189 (permalink .html extension) -- in-progress. The muan-blog og:url diffs (2163 pages) are caused by the same `.html` extension bug. Once 189 is done, most muan-blog SEO diffs should resolve.
- Issue 173 (seo tag meta ordering) -- done. Verify remaining ordering issues.

## Sub-tasks

### Sub-task 1: Investigation (do this FIRST)

Analyze the dom-details files to categorize the exact sub-types of SEO meta tag diffs:

1. Read `docs/comparison/dom-details/muan-blog.txt` and grep for `head > meta` diffs. Categorize:
   - How many are og:url with `.html` extension? (covered by issue 189)
   - How many are empty description/og:description?
   - How many are datetime format differences?
   - How many are other?
2. Read `docs/comparison/dom-details/opensource-guide.txt` and categorize meta diffs.
3. Read `docs/comparison/dom-details/DataTalksClub-datatalksclub.github.io.txt` for the 2 DTC pages.
4. Document findings in the issue log before writing any code.

### Sub-task 2: Fix muan-blog description generation

The muan-blog notes have empty `content=''` for description meta tags where Jekyll populates them from post content.

### Sub-task 3: Fix DTC title truncation

The DTC page `how-do-data-professionals-use-data-engineering-tools-and-practices.html` has "Data" stripped from the title. The word "Data" appears to be removed by some filtering logic.

### Sub-task 4: Fix remaining meta tag ordering/content for non-muan sites

Address opensource-guide (og:locale, article:published_time), choosealicense.com, government-github, jekyll-docs.

## TDD Test Scenarios

### Test 1: muan-blog description populated from content (write FIRST, verify it fails)

```rust
#[test]
fn test_seo_description_from_post_content() {
    // Setup: Create a site with a post that has no `description` in front matter
    // but has body content. Site config has no `description` either.
    // The jekyll-seo-tag plugin uses the first 200 chars of content as description.
    //
    // Assert: The generated HTML contains:
    //   <meta name="description" content="first 200 chars of content...">
    //   <meta property="og:description" content="first 200 chars of content...">
    //
    // Verify it FAILS before implementing (description is currently empty).
}
```

### Test 2: Title not truncating words (write FIRST, verify it fails)

```rust
#[test]
fn test_seo_title_preserves_all_words() {
    // Setup: Create a page with title containing "Data" multiple times:
    //   title: "How Do Data Professionals Use Data Engineering"
    //
    // Assert: The <title> and og:title meta tag contain the full title
    //   including both instances of "Data".
    //
    // Verify it FAILS (currently strips "Data" from title).
}
```

### Test 3: Meta tag ordering matches Jekyll for opensource-guide pattern

```rust
#[test]
fn test_seo_meta_tag_ordering_with_locale() {
    // Setup: Create a site with og:locale and article:published_time configured.
    //
    // Assert: Meta tags appear in the same order as jekyll-seo-tag:
    //   og:locale before article:published_time, twitter:card in correct position.
    //
    // Verify it FAILS before implementing.
}
```

### Test 4 (integration, #[ignore]): Build muan-blog and verify meta tags

```rust
#[test]
#[ignore]
fn test_muan_blog_seo_meta_tags() {
    // Build muan-blog site
    // Parse a sample notes page HTML
    // Assert description meta tag is populated (not empty)
    // Assert og:url does not have .html extension (depends on issue 189)
}
```

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with new tests for each sub-task
- [ ] Investigation sub-task documents exact categorization of all 2236 diffs
- [ ] muan-blog description meta tags populated from post content when no explicit description exists
- [ ] DTC title not truncated (word "Data" preserved)
- [ ] Meta tag ordering matches Jekyll for all affected site patterns
- [ ] Any diffs that are actually caused by issue 189 (permalink) are documented as such and not double-counted
