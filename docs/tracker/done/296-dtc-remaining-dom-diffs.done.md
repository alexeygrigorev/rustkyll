# Issue 296: DTC remaining 133 DOM diff pages

## Problem

DTC matches 657/790 (83%). 133 pages have DOM diffs per the comparison at
`docs/comparison/dom-details/DataTalksClub-datatalksclub.github.io.txt`
(built against `_site_rustkyll_recount`). Analysis below categorizes the
diffs and identifies which are fixable vs known limitations.

NOTE: The dom-details file references `_site_rustkyll_recount` which may be
stale. Some diffs (e.g., author description trailing newlines) may already
be fixed in the current `_site_rustkyll` build. The engineer must rebuild
and recount before and after to measure actual impact.

## Diff Analysis

### Total: 156 pages with diffs (dom-details reports 156, not 133)

---

### Category A: JSON-LD value diffs only (104 pages)

These pages have NO content/structural diffs -- only JSON-LD differences.

#### A1: Transcript timestamp format -- 60 pages -- KNOWN ACCEPTABLE

Podcast transcripts use `[0:30]` in rustkyll vs `[30.0]` in Jekyll. This is
a YAML 1.1 sexagesimal interpretation difference. Rustkyll intentionally
keeps the human-readable `0:30` format rather than converting to float
`30.0`. This is documented as an acceptable difference in `src/yaml.rs`
(see `test_sexagesimal_short_timestamp_stays_as_string`).

**Action: No fix needed. These 60 pages should be excluded from the diff
count or the comparison tool should be updated to ignore this known
difference.**

#### A2: About/guest description trailing newline -- ~52 pages -- FIXABLE

Podcast `about[1].description` and `about[2].description` values in JSON-LD
have a trailing `\n` in Jekyll but not in rustkyll (or vice versa). Example:

- Jekyll: `"Born in Argentina...mentoring.\n"` (465 chars)
- Rustkyll: `"Born in Argentina...mentoring."` (464 chars)

The guest bio (`content` field of people collection items) is rendered to
HTML, then `strip_html | strip_newlines | jsonify` is applied. The trailing
newline comes from the HTML rendering adding a `\n` after the closing `</p>`
tag. Jekyll's `strip_html` preserves this trailing newline; rustkyll's
`strip_html` or `strip_newlines` trims it.

**Root cause:** Mismatch in how `strip_newlines` filter handles trailing
newlines after `strip_html`. Jekyll's `strip_newlines` only strips internal
newlines (replaces `\n` with empty string), so a trailing `\n` from the HTML
becomes part of the string. Need to verify exact behavior.

**Files to modify:** `src/template/filters/strip_html.rs`,
`src/template/seo_tag.rs` (the `strip_html_tags` function)

#### A3: Author description -- markdown links not stripped -- ~2 pages -- FIXABLE

Author descriptions that contain markdown links like
`[Accents Welcome](https://accentswelcome.com)` are rendered differently:

- Jekyll keeps raw markdown: `"the founder of [Accents Welcome](https://accentswelcome.com)"`
- Rustkyll strips to plain text: `"the founder of Accents Welcome,"`

**Root cause:** Jekyll's `content` field for collection items is rendered
HTML (so the link becomes `<a href="...">Accents Welcome</a>`). After
`strip_html`, it becomes plain text with the link text preserved but the URL
dropped. The difference is that rustkyll may be stripping markdown before
rendering to HTML, or the content field does not go through the full
markdown rendering pipeline.

**Files to modify:** `src/generator.rs` (collection item content rendering)

#### A4: Description truncation at 200 chars -- ~20 pages -- LIKELY STALE

Many `about[1].description` diffs appear identical in the first 190 chars
with differences only in the truncated portion of the comparison output.
These may be the same trailing-newline issue (A2) or may already be fixed.

**Action: Rebuild and recount to verify.**

---

### Category B: Content/structural diffs (52 pages)

These pages have actual HTML content differences.

#### B1: YouTube embed / HTML comment wrapping -- 6 pages -- LIKELY ALREADY FIXED

Pages: 6 zoomcamp blog posts (ai-dev-tools, data-engineering, free-ml-courses,
llm, machine-learning, mlops). Pattern: `p -> div` tag shift caused by HTML
comment wrapping difference.

**Status:** Issue 274 (`dtc-script-embed-passthrough.done.md`) addressed this
exact pattern. These 6 pages are likely fixed in the current build.

#### B2: Syntax highlighting class diffs -- 6 pages -- PARTIALLY FIXABLE

Pages: do-you-know-golden-rules, how-to-run-postgresql, important-sql-fact,
naming-variables, open-source-ai-agent, practical-guide-better-code.

Token class differences between syntect and Rouge (e.g., `class='k'` vs
`class='n'` for SQL keywords, Python builtins). Issue 290 addressed the
major token mappings. Remaining diffs are edge cases in specific languages.

**Files to modify:** `src/syntax.rs` (language-specific token mappings)

- `how-to-run-postgresql` (140 diffs) and `practical-guide-better-code` (154
  diffs) have cascading diffs from syntax highlighting differences. Fixing
  the token mapping for bash/shell and Python would resolve most of these.

#### B3: Math/LaTeX block rendering -- 2 pages -- LIKELY ALREADY FIXED

Pages: `ner-reformers.html` (199 diffs), `regularization-in-regression.html`
(48 diffs). Display math `$$...$$` wrapped in `<p>` vs bare text node.

**Status:** Issue 276 (`dtc-latex-math-block-rendering.done.md`) fixed this.
These 247 total diffs are likely resolved in the current build.

#### B4: Nested lists in book review comments -- 7 pages -- FIXABLE

Pages: 7 book review pages. Pattern: `ol > li > ul: missing_element` -- a
`<ul>` nested inside an `<ol><li>` is not being produced by the markdown
parser, and instead appears as a sibling.

**Root cause:** Kramdown handles list continuation (a list inside a list
item) differently from pulldown-cmark. When a comment contains numbered
items with sub-bullets, kramdown nests them; pulldown-cmark may break them
into separate lists.

**Files to modify:** `src/kramdown_parser/` (list parsing), or this may
require post-processing in `src/kramdown.rs`

#### B5: Missing `<br>` in book comments -- 8 pages -- FIXABLE

Pattern: Text in book review comments contains line breaks that Jekyll
renders as `<br>` elements. Rustkyll is not producing these `<br>` tags,
causing the text to be merged into a single paragraph with extra text and
missing elements.

**Root cause:** Book review comments use a data file format where line
breaks within a comment should be converted to `<br>`. This is likely a
`newline_to_br` filter or raw HTML `<br>` handling issue.

**Files to modify:** `src/template/filters/newline_to_br.rs`, or the
book review template rendering in `src/template/layout.rs`

#### B6: Comment ordering diffs -- 3 pages -- DATA-DEPENDENT

Pages: 3 book review pages where comment text from different authors appears
in different order. This suggests the comment data is being iterated in a
different order (e.g., hash map iteration order vs insertion order).

**Root cause:** Collection data or YAML array ordering difference.

**Files to modify:** `src/yaml.rs` or `src/generator.rs` (data ordering)

#### B7: Markdown parsing edge cases -- ~10 pages -- MIXED

Various inline parsing differences:
- `data-engineers-arent-plumbers.html`: double-nested `<strong>` (issue 275)
- `interview-with-valerii-chetvertakov.html`: italic/link interaction
- `guidelines-to-get-data-engineer-job-against-odds.html`: zero-width space,
  list item `<p>` wrapping
- `how-to-setup-lightweight-local-version-for-airflow.html` (453 diffs):
  massive markdown parsing differences (bold/link ordering)
- `mlops-zoomcamp.html`: inline `**...**` spanning across content

**Files to modify:** `src/kramdown_parser/span_parser.rs`,
`src/kramdown.rs`

#### B8: Meta/head attribute issues -- 2 pages -- JEKYLL BUGS

- `ml-deployment-lambda.html` (276 diffs): Jekyll parses a malformed YAML
  description as HTML attributes (`aws=''`, `deployment=''`). Rustkyll
  correctly renders the description as a string. This is a Jekyll bug.
- `how-do-data-professionals.html` (170 diffs): Front matter date in the
  slug path differs (`2025-04-29` vs `2025-04-15`), plus description/title
  content differs. Likely a front matter parsing difference.

**Action: B8 diffs should be verified as Jekyll bugs and excluded from the
match count, or the comparison tool should be configured to ignore them.**

#### B9: Unicode/smart quote diffs -- ~5 pages -- MINOR

Single-character differences: curly quotes vs straight quotes, em-dash vs
triple-dash, ellipsis vs three dots. Examples:
- `20210201-data-teams.html`: curly quotes `\u201c` vs straight `"`
- `20220627-designing-machine-learning-systems.html`: `...` vs ellipsis
- `20230123-snowflake-definitive-guide.html`: dash differences

**Files to modify:** `src/template/engine.rs` (smartypants/typographic
processing), `src/kramdown.rs`

---

## Scope for This Issue

This issue is an umbrella analysis. Given the large number of categories,
the engineer should focus on the **top 3 fixable categories** with the
highest page impact:

### Priority 1: JSON-LD description trailing newlines (A2) -- ~52 pages

Fix `strip_newlines` behavior to match Jekyll: the filter should replace
all `\n` characters with empty string (including trailing), not trim the
string. Alternatively, ensure `content` field for collection items includes
the trailing newline that Jekyll produces.

### Priority 2: Verify already-fixed diffs and update comparison (A1, B1, B3)

Rebuild `_site_rustkyll`, rerun the DOM comparison, and confirm that issues
274 (HTML comment wrapping, 6 pages), 276 (math blocks, 2 pages), and
any trailing-newline fixes from issue 217 are reflected. Update the
dom-details file.

### Priority 3: Syntax highlighting remaining token diffs (B2) -- 6 pages

Review and fix the remaining token class mappings in `src/syntax.rs` for
SQL keywords (class `k` vs `n`), Python builtins (`nb` vs `n`), and
YAML/JSON tokens.

### Out of Scope (track separately)

- A1 (transcript timestamps): Known acceptable, 60 pages
- B4 (nested lists): 7 pages, complex kramdown compatibility issue
- B5 (comment `<br>`): 8 pages, template-level fix
- B6 (comment ordering): 3 pages, data ordering issue
- B7 (markdown parsing): 10 pages, tracked partly by issue 275
- B8 (Jekyll bugs): 2 pages, should be excluded from comparison
- B9 (unicode/quotes): 5 pages, minor cosmetic differences

## Dependencies

- Issue 274 (HTML comment wrapping) -- DONE
- Issue 275 (inline emphasis nesting) -- TODO, covers some B7 pages
- Issue 276 (math block rendering) -- DONE
- Issue 290 (Rouge-compatible syntax highlighting) -- DONE

## Key Files to Modify

- `src/template/filters/strip_html.rs` -- strip_html behavior for trailing newlines
- `src/template/seo_tag.rs` -- JSON-LD description generation, strip_html_tags
- `src/generator.rs` -- collection item content rendering (lines ~5090-5140)
- `src/syntax.rs` -- syntax highlighting token mappings (SQL, Python)
- `src/yaml.rs` -- only if comment ordering (B6) is in scope

## Acceptance Criteria

- [ ] Rebuild DTC site with rustkyll and rerun DOM comparison
- [ ] JSON-LD `about[N].description` trailing newline matches Jekyll output on all podcast pages
- [ ] JSON-LD `author[0].description` with markdown links matches Jekyll output (raw markdown preserved)
- [ ] DOM match count improves to 720+/790 (currently 657, with ~60 known-acceptable timestamp diffs and ~8 already-fixed pages)
- [ ] Syntax highlighting class diffs for SQL and Python reduced (target: fix `class='k'` vs `class='n'` for SQL keywords)
- [ ] No regressions on other sites (choosealicense, lanyon, mlwiki, etc.)
- [ ] `cargo test` passes
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes

## Test Scenarios

### Unit: JSON-LD description trailing newline

- Render a people collection item with bio ending in a paragraph, verify
  `content | strip_html | strip_newlines | jsonify` produces output with
  trailing newline matching Jekyll behavior
- Render a people collection item with bio containing markdown links, verify
  the JSON-LD description preserves the raw markdown link syntax (matching
  Jekyll's `{{ guest.content | strip_html | strip_newlines | jsonify }}`)
- Test `strip_newlines` filter: input `"Hello\nWorld\n"` should produce
  `"HelloWorld"` (Jekyll replaces all `\n` with empty string)

### Unit: Syntax highlighting token classes

- Highlight a SQL code block with `SELECT`, `FROM`, `WHERE` keywords, verify
  they get `class="k"` (not `class="n"`)
- Highlight a Python code block with `len()`, `print()` builtins, verify
  they get `class="nb"` (not `class="n"`)

### Integration: DOM comparison recount

- Build DTC site with rustkyll
- Run DOM comparison tool
- Verify match count is 720+ out of 790
- Verify no new diffs introduced (no regressions)
- Specifically check: `blog/data-narrative.html` (author description with
  markdown link), `podcast/analytics-engineer-skills-tools.html` (about
  description trailing newline), `blog/important-sql-fact.html` (SQL
  syntax highlighting)

### Regression: Other sites

- Run DOM comparison on choosealicense, lanyon, mlwiki to verify no regressions
- Run `cargo test` full suite
