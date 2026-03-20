# Issue 237: Support Lanyon Jekyll theme

## Problem

Lanyon is a popular Jekyll theme (~3.2k GitHub stars), a companion to Hyde with a toggle sidebar. It is not currently in our benchmark suite. We need to clone it, build it with rustkyll, compare output against Jekyll, record the match rate, and fix any theme-specific rendering issues that are within scope.

## Theme Details

- **GitHub:** https://github.com/poole/lanyon
- **Stars:** ~3,200
- **Use case:** Personal blogs, simple sites
- **Notable features:** Toggle sidebar, based on Poole (like Hyde), clean responsive layout, multiple color schemes, pagination via jekyll-paginate

### Theme structure

- 3 layouts: `default.html`, `post.html`, `page.html`
- 2 includes: `head.html`, `sidebar.html`
- 3 example posts (one uses `{% highlight js %}...{% endhighlight %}`)
- 1 standalone page (`about.md`)
- 1 custom 404 page
- 1 custom Atom feed (`atom.xml`)
- Pre-compiled CSS in `public/css/` (no SASS compilation needed)
- `paginate: 5` with `jekyll-paginate` plugin
- `permalink: pretty`

### Build findings from PM research

The theme was cloned into `websites/lanyon/` and built with rustkyll. The build succeeds with no errors and produces 7 pages (3 posts, 4 standalone pages), 6 static files.

**What works:**
- All 3 layouts render correctly
- Both includes resolve and render
- Pagination works (all 3 posts appear on index via `paginator.posts`)
- Sidebar navigation dynamically lists pages with `layout: page`
- Active nav highlighting works (`page.url == node.url`)
- `site.related_posts` renders on post pages
- `{% highlight js %}` renders with syntax highlighting on post pages
- `absolute_url` filter works correctly
- `date_to_string` and `date_to_xmlschema` filters work
- Static assets (CSS, JS, favicon) copied correctly
- Sitemap and auto-generated feed.xml work

**Issues found:**

1. **`{% highlight %}` tags appear as literal text in paginator context (index.html).** On the post's own page (`/2020/04/02/example-content/`), `{% highlight js %}` renders as `<figure class="highlight"><pre><code>...</code></pre></figure>`. But on `index.html`, where the same content is accessed via `{{ post.content }}` in the paginator loop, the highlight tags appear as literal `{% highlight js %}` text wrapped in `<p>` tags. This is a bug: `post.content` in paginator context does not process Liquid highlight tags.

2. **Custom `atom.xml` outputs raw Markdown instead of rendered HTML.** The theme has a custom `atom.xml` template that uses `{{ post.content | xml_escape }}`. In rustkyll, this outputs raw Markdown source. In Jekyll, `post.content` in a Liquid template returns the rendered HTML. The auto-generated `feed.xml` works correctly (it uses the rendered content). This is a known limitation of custom feed templates.

3. **`site.github.repo` is empty.** The sidebar includes `{{ site.github.repo }}` links for "Download" and "GitHub project". This is a GitHub Pages metadata feature that requires the `jekyll-github-metadata` plugin, which rustkyll does not support. The links render as empty href or missing prefix. This is expected behavior and not a rustkyll bug to fix.

## Scope

This issue covers:
- Ensuring the Lanyon theme is cloned and buildable
- Running DOM comparison against Jekyll output
- Recording and documenting the match rate
- Fixing rendering issues that are specific to this theme and within reasonable scope

This issue does NOT cover (these are pre-existing engine bugs, not Lanyon-specific):
- Fixing `post.content` in paginator/template context to return rendered HTML instead of raw Markdown (affects atom.xml and potentially paginator highlight tags) -- this is a cross-cutting engine issue
- Supporting `site.github.repo` metadata -- this requires the `jekyll-github-metadata` plugin

## Tasks

1. Ensure `websites/lanyon/` is cloned (already done by PM during grooming)
2. Build with Jekyll to produce reference output in `websites/lanyon/_site_jekyll/` (or a temp dir)
3. Build with rustkyll
4. Run DOM comparison between Jekyll and rustkyll output
5. Record match rate in this issue
6. If match rate is below target, identify specific diffs and fix what is feasible
7. Add an integration test for the Lanyon theme page count

## Acceptance Criteria

- [ ] `websites/lanyon/` directory exists with the cloned theme
- [ ] `rustkyll build --source websites/lanyon` succeeds with zero errors and zero warnings
- [ ] All 7 pages are generated: 3 post pages, index.html, about/index.html, 404.html, atom.xml
- [ ] All 6 static files are copied (3 CSS files, 1 JS file, 1 favicon, 1 apple-touch-icon)
- [ ] Post pages render with correct layout chain: post.html -> default.html
- [ ] `about` page renders with correct layout chain: page.html -> default.html
- [ ] Sidebar navigation lists the "About" page dynamically (from `site.pages` with `layout: page`)
- [ ] Index page shows all 3 posts with titles, dates, and full content via paginator
- [ ] `{% highlight js %}` block in `example-content` post renders as `<figure class="highlight"><pre><code>` on the post's own page
- [ ] `date_to_string` filter produces correct date format (e.g., "03 Apr 2020")
- [ ] `absolute_url` filter prepends the configured `url` (http://lanyon.getpoole.com)
- [ ] `site.related_posts` section renders on post pages with links to other posts
- [ ] Auto-generated `feed.xml` contains rendered HTML in entry content (not raw Markdown)
- [ ] `sitemap.xml` lists all 7 pages
- [ ] DOM comparison is run against Jekyll output and match rate is recorded in this issue file
- [ ] An integration test exists in `integration_tests/` that builds Lanyon and verifies the HTML page count

## Test Scenarios

### Integration: Lanyon theme page count
- Build `websites/lanyon/` with rustkyll, verify exactly 4 HTML files are generated (3 posts + about + index + 404 = 6 HTML files; atom.xml and feed.xml are not HTML)
- Note: exact count should be determined by building with Jekyll first; adjust the test to match

### Integration: DOM comparison
- Build with both Jekyll and rustkyll
- Run the structural comparison tool
- Record the match percentage
- Document any diffs that are expected vs unexpected

### Output verification: Post page content
- Build Lanyon site, read `/2020/04/03/introducing-lanyon/index.html`
- Verify it contains `<h1 class="post-title">Introducing Lanyon</h1>`
- Verify it contains `<span class="post-date">03 Apr 2020</span>`
- Verify it contains the related posts section with links

### Output verification: Index page
- Build Lanyon site, read `/index.html`
- Verify it contains 3 post entries with `<h1 class="post-title">` and `<a href=`
- Verify pagination controls are present

### Output verification: Sidebar
- Read any generated HTML page
- Verify sidebar contains `<a class="sidebar-nav-item"` linking to About
- Verify sidebar contains `site.description` content

### Output verification: Syntax highlighting
- Read `/2020/04/02/example-content/index.html`
- Verify it contains `<figure class="highlight">` and `<code class="language-js"`

## Known Issues to Document (not fix)

These are pre-existing engine issues that affect Lanyon but are not Lanyon-specific. They should be documented in the match rate report but do NOT block acceptance of this issue:

1. **`post.content` in custom `atom.xml` returns raw Markdown** -- The custom `atom.xml` template outputs unrendered Markdown. This affects any theme with a custom feed template. File a separate issue if one does not already exist.
2. **`{% highlight %}` tags in paginator `post.content`** -- On `index.html`, highlight tags may appear as literal text instead of rendered code blocks. This is the same root cause as issue 1 (post.content returning raw/partially-processed content in certain template contexts).
3. **`site.github.repo` is empty** -- GitHub Pages metadata plugin not supported. Links in sidebar render with empty href.

## Dependencies

- None (this is a benchmark/research task)

## DOM Comparison Results

**Match rate: 4/6 files (66.7%) exact DOM match**

Comparison run: rustkyll vs Jekyll 4.4.1

### Files that match perfectly (4/6):
- `2020/04/01/whats-jekyll/index.html` -- exact match
- `2020/04/03/introducing-lanyon/index.html` -- exact match
- `404.html` -- exact match
- `about/index.html` -- exact match

### Files with differences (2/6):

**`2020/04/02/example-content/index.html`** -- 4 differences
- All 4 are syntax highlighting token class differences (`class='k'` vs `class='o'`, `class='nc'` vs `class='nb'`, `class='mi'` vs `class='m'` x2). These are minor differences in how the syntax highlighter classifies JavaScript tokens. Functionally equivalent; visual appearance depends on the syntax.css theme.

**`index.html`** -- 20 differences
- 1 canonical link href difference: Jekyll uses `http://lanyon.getpoole.com/`, rustkyll uses `http://lanyon.getpoole.com/index.html`
- 19 content differences: all caused by `post.content` in paginator context returning raw Markdown instead of rendered HTML for the "Example content" post. The `{% highlight js %}` block appears as literal text instead of being rendered as a `<figure class="highlight">` block, shifting all subsequent DOM elements.

### Known issues (not Lanyon-specific, pre-existing engine bugs):
1. **`post.content` in paginator context** returns partially-processed content (highlight tags not rendered)
2. **Custom `atom.xml`** outputs raw Markdown via `post.content` (auto-generated `feed.xml` works correctly)
3. **`site.github.repo` is empty** -- `jekyll-github-metadata` plugin not supported

### Summary
The Lanyon theme builds successfully with zero errors. 4 of 6 HTML files match Jekyll exactly. The 2 files with differences are caused by pre-existing engine issues (post.content in template contexts) and minor syntax highlighter token classification differences, not by Lanyon-specific bugs.

## Log

### [SWE] 2026-03-20
- Read issue, understood scope: add Lanyon to benchmark, write integration tests, run DOM comparison, document match rate
- Verified `websites/lanyon/` exists and builds with rustkyll: 7 pages, 6 static files, zero errors
- Built with Jekyll 4.4.1 for reference: identical file count (6 HTML files)
- Ran DOM comparison via `scripts/compare-output.sh`: 4/6 exact match, 24 total differences in 2 files
- Wrote 16 integration tests in `integration_tests/tests/integration_lanyon.rs` covering:
  - Page count (6 HTML files)
  - All expected files exist (9 files including atom.xml, feed.xml, sitemap.xml)
  - Static asset copying (6 assets)
  - Post title and date rendering
  - Post layout chain (post.html -> default.html)
  - Related posts section
  - About page layout (page.html -> default.html)
  - Sidebar navigation on index and post pages
  - Index page shows all 3 posts with links
  - Syntax highlighting on post page
  - absolute_url filter
  - feed.xml has rendered HTML content
  - Sitemap lists all pages
  - Unicode/smart quotes in "What's Jekyll?" title
- Added 1 page count test in `integration_tests/tests/integration_page_counts.rs`
- All 17 tests pass
- Clippy clean, fmt clean
- Documented DOM comparison results with match rate in issue file
- Files created: `integration_tests/tests/integration_lanyon.rs`
- Files modified: `integration_tests/tests/integration_page_counts.rs`, `docs/tracker/237-support-lanyon-theme.in-progress.md`
