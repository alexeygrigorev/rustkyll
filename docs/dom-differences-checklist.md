# DOM Differences Checklist

Comprehensive categorization of all DOM differences between Jekyll and rustkyll output
across 35 benchmark sites. Generated from `docs/comparison/dom-details/` analysis.

**Total files with differences: 4317 categorized** (4319 total non-matching out of 9767 compared;
2 files in documentation-theme-jekyll have diffs not captured in detail files due to a reporting gap).

Each category represents a distinct root cause. Categories are ordered by total page impact
(most affected pages first). Note: a single file may have diffs in multiple categories,
so per-category file counts do not sum to the total.

---

- [ ] **SEO meta tag differences (og:, twitter:, description)** -- 2236 pages

  **Description:** Open Graph, Twitter Card, and description meta tags have different content, ordering, or presence between Jekyll and rustkyll output. This includes property/name attribute mismatches and content value differences.

  **Root cause:** rustkyll's SEO tag generation (equivalent to jekyll-seo-tag plugin) produces meta tags in a different order or with different content. Specific issues include: meta tag ordering differences, og:type values, twitter:card values, and description content not matching Jekyll's seo-tag plugin output.

  **Affected sites:** DataTalksClub-datatalksclub.github.io (2), alexeygrigorev-aihero (2), choosealicense.com (16), government-github (11), jekyll-docs-docs (14), muan-blog (2163), opensource-guide (28)

  **Sample diff** (`DataTalksClub-datatalksclub.github.io/blog/how-do-data-professionals-use-data-engineering-tools-and-practices.html`):
  ```
    head > meta: attribute_differs - expected: "content='How Do Data Professionals Use Data Engineering Tools and Practices? – DataTalks.Club'", actual: "content='How Do Professionals Use Data Engineering Tools and Practices? – DataTalks.Club'"
  ```

- [ ] **Layout/template not applied** -- 773 pages

  **Description:** Pages render as raw markdown/HTML content without the site's layout template wrapping them. The output has content elements (h1, p, ul, etc.) directly at root level instead of inside `<head>` and `<body>` elements within the layout.

  **Root cause:** rustkyll fails to find or apply the correct layout template for these pages. This could be due to: missing `_layouts/` directory resolution for gem-based themes, `_includes` files not found, layout inheritance not working (`layout: page` -> `layout: default`), or data file references in templates failing and causing the template engine to output raw content.

  **Affected sites:** DataTalksClub-docs (57), academicpages (16), alexeygrigorev-little-book-of-metals-ru (9), alexeygrigorev-snippets (17), beautiful-jekyll (5), choosealicense.com (55), documentation-theme-jekyll (97), government-github (6), jekyll-docs-docs (109), just-the-docs (47), muan-blog (7), opensource-guide (337), so-simple-theme (11)

  **Sample diff** (`DataTalksClub-docs/404.html`):
  ```
    child[1]: tag_name_differs - expected: 'head', actual: 'h1'
  ```

- [ ] **Body class attribute differs** -- 764 pages

  **Description:** The `class` attribute on `<body>` elements differs. For example, Jekyll outputs `class='col-pages post-body'` but rustkyll outputs `class='col- post-body'` (missing the collection name in the class).

  **Root cause:** rustkyll's template rendering does not correctly compute page-type CSS classes that depend on the page's collection membership, category, or layout type.

  **Affected sites:** muan-blog (764)

  **Sample diff** (`muan-blog/no-yc/index.html`):
  ```
    body: attribute_differs - expected: "class='col-pages post-body'", actual: "class='col- post-body'"
  ```

- [ ] **Permalink adds .html extension incorrectly** -- 756 pages

  **Description:** Internal links have an extra `.html` extension. Jekyll generates links like `/pages/banners` but rustkyll generates `/pages/banners.html`.

  **Root cause:** rustkyll's permalink/URL generation adds `.html` extensions to page URLs when the site's permalink style should omit them (e.g., `permalink: pretty` or custom permalink patterns without extensions).

  **Affected sites:** muan-blog (756)

  **Sample diff** (`muan-blog/accessibility-statement.html`):
  ```
    body > footer > p > a: attribute_differs - expected: "href='/pages/banners'", actual: "href='/pages/banners.html'"
  ```

- [ ] **Date formatting missing leading zeros** -- 673 pages

  **Description:** Dates formatted in templates lose leading zeros. Jekyll outputs `2023/07/11 15:27` but rustkyll outputs `2023/7/11 15:27`.

  **Root cause:** rustkyll's Liquid date filter (`date: '%Y/%m/%d %H:%M'`) does not pad month, day, and hour values with leading zeros when the format string uses `%m`, `%d`, `%H` etc.

  **Affected sites:** muan-blog (673)

  **Sample diff** (`muan-blog/stories/00452b63-9f35-4b28-5258-f4fe51565300.html`):
  ```
    body > header > div > h1: text_differs - expected: 'Story, 2023/07/11 15:27', actual: 'Story, 2023/7/11 15:27'
  ```

- [ ] **Syntax highlighting differences** -- 574 pages

  **Description:** Code blocks have different syntax highlighting: span elements have different CSS classes (e.g., `class='k'` vs `class='n'`), different text content within spans, or different span structure (missing/extra spans).

  **Root cause:** rustkyll uses a different syntax highlighting engine or version than Jekyll's Rouge highlighter. The tokenization and classification of code tokens differs, producing different CSS class assignments and token boundaries.

  **Affected sites:** DataTalksClub-datatalksclub.github.io (11), alexeygrigorev-mlbookcamp-page (6), alexeygrigorev-mlwiki.org (47), architect-theme (1), cayman-theme (1), dinky-theme (1), hacker-theme (1), large-docs-site (500), leap-day-theme (1), merlot-theme (1), midnight-theme (1), mojombo-blog (1), slate-theme (1), time-machine-theme (1)

  **Sample diff** (`DataTalksClub-datatalksclub.github.io/blog/do-you-know-golden-rules-while-working-with-data.html`):
  ```
    body > div > div > div > div > div > div > pre > code > span: attribute_differs - expected: "class='k'", actual: "class='n'"
  ```

- [ ] **Content text/ordering differences (collection sort, markdown)** -- 358 pages

  **Description:** Page body text content differs between Jekyll and rustkyll. This includes: collection items appearing in different order, markdown content being split into different text nodes, and text content shifts due to upstream element structure changes.

  **Root cause:** Multiple causes: (1) Collections sorted differently (different default sort key or sort order), (2) Markdown rendering produces different paragraph/text splits, (3) Front matter values processed differently.

  **Affected sites:** DataTalksClub-datatalksclub.github.io (24), alexeygrigorev-alexeygrigorev.github.io (1), alexeygrigorev-kids-horror-stories-ru (2), alexeygrigorev-mlbookcamp-page (1), alexeygrigorev-mlwiki.org (325), large-blog-3000 (1), mojombo-blog (4)

  **Sample diff** (`DataTalksClub-datatalksclub.github.io/blog/data-narrative.html`):
  ```
    body > div > div > div > div > p: text_differs - expected: 'My sister, however, is a fantastic storyteller. She is able to make the most mundane events engaging. Her stories seem to connect with \u200b', actual: 'My sister, however, is a fantastic storyteller. She is able to make the most mundane events engaging. Her stories seem to connect with \u200b_everyone_. \u200b People laugh. Listeners pay attention. Others retell her stories. She combines characters, conflict, and a conclusion seamlessly. A good business story must do the same.'
  ```

- [ ] **Markdown block structure differences** -- 335 pages

  **Description:** The HTML element structure produced by markdown rendering differs. Elements appear in different order, text content that should be in `<p>` tags appears as raw text, or block-level elements are wrapped differently.

  **Root cause:** Differences between rustkyll's markdown parser output and Jekyll's kramdown parser for edge cases: image references with markdown links, nested lists, definition lists, and complex block-level structures.

  **Affected sites:** DataTalksClub-datatalksclub.github.io (26), alexeygrigorev-aihero (2), alexeygrigorev-mlbookcamp-page (4), alexeygrigorev-mlwiki.org (235), architect-theme (1), cayman-theme (1), dinky-theme (1), government-github (2), hacker-theme (1), leap-day-theme (1), merlot-theme (1), midnight-theme (1), mojombo-blog (2), muan-blog (53), opensource-guide (2), slate-theme (1), time-machine-theme (1)

  **Sample diff** (`DataTalksClub-datatalksclub.github.io/blog/essentials-of-public-speaking-for-career-in-data-science.html`):
  ```
    body > div > div > div > div: expected_element_got_text - expected: '<p>', actual: 'Photo by [Kane Reinholdtsen](https://unsplash.com/@kanereinholdtsen?utm_source=medium&utm_medium=ref'
  ```

- [ ] **JSON-LD other value differences** -- 212 pages

  **Description:** Various JSON-LD fields differ: `description` truncation/content, `headline`, page title format. Includes cases where special characters like `$` get different handling.

  **Root cause:** Multiple minor differences in how rustkyll generates JSON-LD structured data compared to Jekyll's jekyll-seo-tag plugin, including text truncation logic, special character handling, and field content generation.

  **Affected sites:** DataTalksClub-datatalksclub.github.io (202), architect-theme (1), cayman-theme (2), dinky-theme (1), hacker-theme (1), leap-day-theme (1), merlot-theme (1), midnight-theme (1), slate-theme (1), time-machine-theme (1)

  **Sample diff** (`DataTalksClub-datatalksclub.github.io/people/danbecker.html`):
  ```
    body > script > jsonld.description: jsonld_value_differs - expected: 'Dan started his data science career by finishing 2nd (out of 1353 teams) in a kaggle competition with a $500,000 grand prize. Since then, he’s worked as a data scientist at Google and was Product D...', actual: 'Dan started his data science career by finishing 2nd (out of 1353 teams) in a kaggle competition with a$500,000 grand prize. Since then, he’s worked as a data scientist at Google and was Product Di...'
  ```

- [ ] **Text node splitting differences** -- 138 pages

  **Description:** Text content is split differently across child text nodes and elements. For example, text after a `<br>` tag may appear as a separate text node in one output but as part of the parent element's text in the other.

  **Root cause:** Different handling of inline elements and text node boundaries in the HTML output. This can result from different markdown rendering or template output for elements like `<br>` tags within paragraphs.

  **Affected sites:** DataTalksClub-datatalksclub.github.io (22), alexeygrigorev-mlbookcamp-page (1), alexeygrigorev-mlwiki.org (114), mojombo-blog (1)

  **Sample diff** (`DataTalksClub-datatalksclub.github.io/blog/ai-tools-for-personal-productivity.html`):
  ```
    body > div > div > div > div > div > p > br: extra_text - expected: '(none)', actual: "We'll keep you informed about our events, articles, courses, and everything else happening in the Cl"
  ```

- [ ] **Missing HTML elements in rustkyll output** -- 126 pages

  **Description:** rustkyll output is missing HTML elements (like `<p>`, `<a>`, `<span>`) that Jekyll includes. Content may be present but not wrapped in the expected element.

  **Root cause:** Markdown rendering differences where rustkyll doesn't generate certain wrapper elements, or content that should produce specific elements is handled differently.

  **Affected sites:** DataTalksClub-datatalksclub.github.io (17), alexeygrigorev-mlwiki.org (103), government-github (2), jekyll-docs-docs (2), opensource-guide (2)

  **Sample diff** (`DataTalksClub-datatalksclub.github.io/blog/free-machine-learning-courses.html`):
  ```
    body > div > div > div > div > p: missing_element - expected: '<p>', actual: '(none)'
  ```

- [ ] **Markdown table rendering failure** -- 109 pages

  **Description:** Markdown tables are not rendered as `<table>` HTML elements. Instead, the table content appears as raw text or within other elements.

  **Root cause:** rustkyll's markdown parser does not handle certain table formats, particularly pipe tables that appear inside list items or have non-standard formatting (wiki-style tables, tables with leading pipes).

  **Affected sites:** DataTalksClub-datatalksclub.github.io (1), alexeygrigorev-mlwiki.org (108)

  **Sample diff** (`DataTalksClub-datatalksclub.github.io/books/20220425-natural-language-processing-with-transformers.html`):
  ```
    body > div > div > div > div > div > div > ul > li: expected_element_got_text - expected: '<table>', actual: 'engineering: training such models requires a large distributed infrastructure with'
  ```

- [ ] **Extra HTML elements in rustkyll output** -- 90 pages

  **Description:** rustkyll generates extra HTML elements (like `<p>`, `<ul>`, `<div>`) that are not present in Jekyll's output. These appear within the page content area.

  **Root cause:** Markdown rendering differences where rustkyll wraps content in additional block elements, or Liquid template processing produces extra wrapper elements.

  **Affected sites:** DataTalksClub-datatalksclub.github.io (17), alexeygrigorev-mlbookcamp-page (1), alexeygrigorev-mlwiki.org (56), muan-blog (16)

  **Sample diff** (`DataTalksClub-datatalksclub.github.io/blog/guidelines-to-get-data-engineer-job-against-odds.html`):
  ```
    body > div > div > div > div > ul > li > p: extra_element - expected: '(none)', actual: '<p>'
  ```

- [ ] **Other attribute differences** -- 85 pages

  **Description:** Miscellaneous attribute differences not covered by other categories, such as `alt` attribute whitespace differences on images.

  **Root cause:** Various minor differences in how attributes are generated, including whitespace normalization in alt text and other attribute values.

  **Affected sites:** alexeygrigorev-little-book-of-metals-ru (33), alexeygrigorev-mlbookcamp-page (3), alexeygrigorev-mlwiki.org (48), mojombo-blog (1)

  **Sample diff** (`alexeygrigorev-little-book-of-metals-ru/часть_1_история/глава_01_введение.html`):
  ```
    body > main > div > article > div > h1: attribute_differs - expected: "id='глава-1-введение---мир-металлов-вокруг-нас'", actual: "id='-1-------'"
  ```

- [ ] **Redirect pages use relative URLs instead of absolute** -- 41 pages

  **Description:** Pages generated by jekyll-redirect-from use relative URLs (`/community/`) instead of absolute URLs (`https://choosealicense.com/community/`). Affects `<link>`, `<meta>`, `<a>`, and `<script>` elements.

  **Root cause:** rustkyll's redirect page generation does not prepend `site.url` to the redirect target URL, producing relative paths instead of the absolute URLs that Jekyll generates.

  **Affected sites:** choosealicense.com (16), government-github (11), jekyll-docs-docs (14)

  **Sample diff** (`choosealicense.com/existing/index.html`):
  ```
    link: attribute_differs - expected: "href='https://choosealicense.com/community/'", actual: "href='/community/'"
  ```

- [ ] **JSON-LD datePublished timezone offset** -- 34 pages

  **Description:** The `datePublished` field in JSON-LD structured data uses UTC (+00:00) instead of the local timezone offset. Jekyll outputs `2023-12-11T00:00:00+01:00` but rustkyll outputs `2023-12-11T00:00:00+00:00`.

  **Root cause:** rustkyll does not apply the site's timezone configuration (from `_config.yml` `timezone` field) when formatting dates in JSON-LD output.

  **Affected sites:** DataTalksClub-datatalksclub.github.io (34)

  **Sample diff** (`DataTalksClub-datatalksclub.github.io/blog/8-newsletters-for-data-science-ai-and-ml-enthusiasts.html`):
  ```
    body > script > jsonld.@graph[0].datePublished: jsonld_value_differs - expected: '2023-12-11T00:00:00+01:00', actual: '2023-12-11T00:00:00+00:00'
  ```

- [ ] **Content link href differences** -- 33 pages

  **Description:** Links within page content have different href values. Includes URL encoding differences for non-ASCII characters (Cyrillic, special chars) and zero-width space handling.

  **Root cause:** rustkyll URL-encodes non-ASCII characters in href attributes (producing `%D0%...` for Cyrillic) while Jekyll preserves them as-is. Also, zero-width spaces in URLs get encoded differently.

  **Affected sites:** alexeygrigorev-alexeygrigorev.github.io (1), alexeygrigorev-mlwiki.org (2), choosealicense.com (2), dinky-theme (2), government-github (10), hacker-theme (2), large-blog-3000 (1), leap-day-theme (2), merlot-theme (2), midnight-theme (2), mojombo-blog (3), opensource-guide (2), time-machine-theme (2)

  **Sample diff** (`alexeygrigorev-alexeygrigorev.github.io/services.html`):
  ```
    body > main > div > div > div > div > div > a: attribute_differs - expected: "href='/services/consulting.html'", actual: "href='/services/devrel.html'"
  ```

- [ ] **JSON-LD author description trailing whitespace/markdown** -- 21 pages

  **Description:** Author descriptions in JSON-LD have trailing newlines or unprocessed markdown links. Jekyll outputs clean text but rustkyll appends `\n` or leaves `[link text](url)` as raw markdown.

  **Root cause:** rustkyll does not strip trailing whitespace from front matter values or does not process markdown within author description fields before embedding them in JSON-LD.

  **Affected sites:** DataTalksClub-datatalksclub.github.io (21)

  **Sample diff** (`DataTalksClub-datatalksclub.github.io/blog/benefits-of-learning-in-public.html`):
  ```
    body > script > jsonld.@graph[0].author[0].description: jsonld_value_differs - expected: 'Alexey Grigorev is the founder of DataTalks.Club', actual: 'Alexey Grigorev is the founder of DataTalks.Club\n'
  ```

- [ ] **Markdown inline formatting not applied** -- 21 pages

  **Description:** Inline markdown formatting like `_emphasis_`, `**bold**`, and `[links](url)` is not converted to HTML. Missing `<em>`, `<strong>`, or `<a>` elements.

  **Root cause:** Some markdown content is not being processed through the markdown parser, likely because it appears in a context where rustkyll doesn't apply markdown conversion (e.g., within certain Liquid output or front matter values).

  **Affected sites:** DataTalksClub-datatalksclub.github.io (9), alexeygrigorev-mlwiki.org (6), government-github (3), jekyll-docs-docs (2), mojombo-blog (1)

  **Sample diff** (`DataTalksClub-datatalksclub.github.io/blog/data-narrative.html`):
  ```
    body > div > div > div > div > p > em: missing_element - expected: '<em>', actual: '(none)'
  ```

- [ ] **Title tag missing site description suffix** -- 19 pages

  **Description:** The `<title>` element shows only the page title without the `| site.description` suffix that Jekyll appends. Jekyll outputs `Theme Name | Description` but rustkyll outputs just `Theme Name`.

  **Root cause:** rustkyll's SEO tag generation or title template does not append `site.description` or `site.tagline` to the page title in the format expected by jekyll-seo-tag.

  **Affected sites:** DataTalksClub-datatalksclub.github.io (1), architect-theme (2), cayman-theme (1), dinky-theme (2), hacker-theme (2), leap-day-theme (2), merlot-theme (2), midnight-theme (2), opensource-guide (1), slate-theme (2), time-machine-theme (2)

  **Sample diff** (`DataTalksClub-datatalksclub.github.io/blog/how-do-data-professionals-use-data-engineering-tools-and-practices.html`):
  ```
    head > title: text_differs - expected: 'How Do Data Professionals Use Data Engineering Tools and Practices? – DataTalks.Club', actual: 'How Do Professionals Use Data Engineering Tools and Practices? – DataTalks.Club'
  ```

- [ ] **URL encoding differences for special characters** -- 19 pages

  **Description:** Non-ASCII characters in URLs (Cyrillic text, special symbols, square brackets) are percent-encoded differently. Jekyll preserves certain characters while rustkyll encodes them.

  **Root cause:** rustkyll applies stricter URL encoding than Jekyll, encoding characters that Jekyll leaves as-is (particularly non-ASCII characters in fragment identifiers and path components).

  **Affected sites:** DataTalksClub-datatalksclub.github.io (2), alexeygrigorev-mlwiki.org (17)

  **Sample diff** (`DataTalksClub-datatalksclub.github.io/books/20210426-tiny-python-projects.html`):
  ```
    body > div > div > div > div > div > div > p > a: attribute_differs - expected: "href='https://learning.oreilly.com/library/view/mastering-python-for/9781098100872/>'", actual: "href='https://learning.oreilly.com/library/view/mastering-python-for/9781098100872/%3E'"
  ```

- [ ] **JSON-LD missing fields (url)** -- 18 pages

  **Description:** JSON-LD structured data is missing the `url` field that Jekyll includes.

  **Root cause:** rustkyll's JSON-LD generation does not include the `url` field from the page's permalink/URL. The jekyll-seo-tag plugin includes this field when `site.url` is configured.

  **Affected sites:** architect-theme (2), cayman-theme (2), dinky-theme (2), hacker-theme (2), leap-day-theme (2), merlot-theme (2), midnight-theme (2), slate-theme (2), time-machine-theme (2)

  **Sample diff** (`architect-theme/another-page.html`):
  ```
    head > meta > meta > meta > script > jsonld.url: jsonld_missing_field - expected: '"/another-page.html"', actual: '(none)'
  ```

- [ ] **JSON-LD extra fields (name)** -- 9 pages

  **Description:** JSON-LD structured data includes a `name` field that Jekyll does not include.

  **Root cause:** rustkyll's JSON-LD generation adds a `name` field (from `site.title`) that the jekyll-seo-tag plugin does not include for this page type.

  **Affected sites:** architect-theme (1), cayman-theme (1), dinky-theme (1), hacker-theme (1), leap-day-theme (1), merlot-theme (1), midnight-theme (1), slate-theme (1), time-machine-theme (1)

  **Sample diff** (`architect-theme/another-page.html`):
  ```
    head > meta > meta > meta > script > jsonld.name: jsonld_extra_field - expected: '(none)', actual: '"Architect theme"'
  ```

- [ ] **JSON-LD FAQ answer text differences** -- 8 pages

  **Description:** FAQ page structured data (`mainEntity[].acceptedAnswer.text`) has minor HTML differences in the answer text, such as trailing whitespace or slight HTML structure variations.

  **Root cause:** rustkyll's markdown-to-HTML conversion for FAQ answers produces slightly different whitespace or HTML structure compared to Jekyll, particularly around trailing spaces and paragraph boundaries.

  **Affected sites:** DataTalksClub-datatalksclub.github.io (8)

  **Sample diff** (`DataTalksClub-datatalksclub.github.io/blog/ai-dev-tools-zoomcamp-2025-free-course-to-master-coding-assistants-agents-and-automation.html`):
  ```
    body > div > div > div > div > script > jsonld.mainEntity[0].acceptedAnswer.text: jsonld_value_differs - expected: '<p>The AI Dev Tools Zoomcamp is a free, community-driven program by <a href="/">DataTalks.Club</a> that teaches practical applications of AI tools in software development through hands-on project work', actual: '<p>The AI Dev Tools Zoomcamp is a free, community-driven program by <a href="/">DataTalks.Club</a> that teaches practical applications of AI tools in software development through hands-on project work'
  ```

- [ ] **Ampersand handling in heading IDs** -- 7 pages

  **Description:** Heading IDs generated from text containing `&` differ. Jekyll produces `free--free-to-audit-courses` (stripping the ampersand) while rustkyll produces `free-amp-free-to-audit-courses` (converting `&` to `amp`).

  **Root cause:** rustkyll's heading ID generation converts `&` to `amp` instead of stripping it like Jekyll/kramdown does.

  **Affected sites:** DataTalksClub-datatalksclub.github.io (3), alexeygrigorev-mlwiki.org (4)

  **Sample diff** (`DataTalksClub-datatalksclub.github.io/blog/free-data-engineering-courses.html`):
  ```
    body > div > div > div > div > h3: attribute_differs - expected: "id='free--free-to-audit-courses'", actual: "id='free-amp-free-to-audit-courses'"
  ```

- [ ] **Inline code gets extra CSS class** -- 1 pages

  **Description:** Inline `<code>` elements inside links get an extra `class='highlighter-rouge language-plaintext'` attribute that Jekyll does not add.

  **Root cause:** rustkyll's markdown renderer adds syntax highlighting classes to inline code elements when they appear in certain contexts (like inside links), while Jekyll/kramdown does not.

  **Affected sites:** muan-blog (1)

  **Sample diff** (`muan-blog/colophon.html`):
  ```
    body > main > ul > li > ul > li > a > code: extra_attribute - expected: '(none)', actual: "class='highlighter-rouge language-plaintext'"
  ```

---

## Summary

| Category | Pages affected |
|----------|---------------|
| SEO meta tag differences (og:, twitter:, description) | 2236 |
| Layout/template not applied | 773 |
| Body class attribute differs | 764 |
| Permalink adds .html extension incorrectly | 756 |
| Date formatting missing leading zeros | 673 |
| Syntax highlighting differences | 574 |
| Content text/ordering differences (collection sort, markdown) | 358 |
| Markdown block structure differences | 335 |
| JSON-LD other value differences | 212 |
| Text node splitting differences | 138 |
| Missing HTML elements in rustkyll output | 126 |
| Markdown table rendering failure | 109 |
| Extra HTML elements in rustkyll output | 90 |
| Other attribute differences | 85 |
| Redirect pages use relative URLs instead of absolute | 41 |
| JSON-LD datePublished timezone offset | 34 |
| Content link href differences | 33 |
| JSON-LD author description trailing whitespace/markdown | 21 |
| Markdown inline formatting not applied | 21 |
| Title tag missing site description suffix | 19 |
| URL encoding differences for special characters | 19 |
| JSON-LD missing fields (url) | 18 |
| JSON-LD extra fields (name) | 9 |
| JSON-LD FAQ answer text differences | 8 |
| Ampersand handling in heading IDs | 7 |
| Inline code gets extra CSS class | 1 |

