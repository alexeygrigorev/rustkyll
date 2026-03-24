# Issue 325: Push DTC DOM coverage to 100% (745/790 -> 790/790)

## Problem

DTC is the flagship site and currently matches 745/790 pages (94%). Issue 316 pushed it from 743 to 746, but ~44 pages still have DOM diffs. Most remaining diffs fall into well-understood categories from issue 316's analysis. Closing this gap is the single most visible quality milestone for the project.

## Remaining diff categories (~44 pages)

### Category A: Book comment `newline_to_br | markdownify` pipeline (~21 pages)

The `{{ thread.text | newline_to_br | markdownify }}` pipeline in book review templates produces incorrect HTML when comments contain lists with `<br />` tags inserted by `newline_to_br`.

**A1: Nested list continuation after `<br />` (5 pages)**
- `<br />\n` inside a numbered list item that has an indented sub-list causes rustkyll to close the `<ol>` and create a separate `<ul>`, instead of keeping the `<ul>` nested inside the `<li>`
- Affected: `20210222-ml-algotrading-2ed`, `20210405-the-practitioners-guide`, `20210927-effective-data-science`, `20240715-ai-data-privacy`, `20241104-llm-engineer`

**A2: Smart quote style differences (6+ pages)**
- Kramdown and rustkyll make different curly/straight quote decisions in comment text
- Affected: `20210412-ai-and-machine-learning-for-coders` and others

**A3: Numbered list restart after `<br />` (10+ pages)**
- `<br />\n3. text` inside an existing `<ol>` starts a new `<ol>` instead of continuing
- `<br />\n` before a blockquote `>` causes structural differences
- Affected: `20220912-skills-of-successful-software-engineer`, `20221121-reliable-machine-learning`, `20230807-driving-data-quality-with-data-contracts`, `20231106-analytics-engineering-with-sql-and-dbt`

### Category B: Remaining include rendering offset (~2 pages)

Issue 316 fixed 4 of the 6 include-offset pages. Two remain:
- `blog/machine-learning-zoomcamp.html` (1 diff: alt attribute)
- `blog/mlops-zoomcamp.html` (4 diffs: bold text parsing)

### Category C: Syntax highlighting class diffs (~6 pages)

Rouge token class mismatches for SQL, Shell/Bash, and Python:
- SQL: `k` vs `n` for `SELECT`, `WHERE`
- Shell: various class diffs for command arguments
- Python: `k` vs `n` for `print`, decorators
- Affected: `do-you-know-golden-rules`, `how-to-run-postgresql`, `important-sql-fact`, `naming-variables`, `open-source-free-ai-agent-evaluation`, `practical-guide-better-code`

### Category D: JSONLD and miscellaneous (~8 pages)

- 4 pages with JSONLD description diffs (remaining from issue 305)
- 2 pages with markdown link parsing issues (`{:target="_blank"}` IAL attributes)
- 2 podcast pages with `<br>` / text split diffs

### Category E: Structural edge cases (~2 pages)

- Missing `<script>` elements (lambda page)
- Duplicate slug resolution (data-professionals page)

## Scope

This issue covers ALL remaining 44 pages. The engineer should triage and fix as many as possible. If some categories prove architecturally difficult (e.g., deep pulldown-cmark list continuation changes), those specific items may be descoped to follow-up issues -- but only with explicit PM approval and new issue creation.

Priority order:
1. Category A (21 pages) -- highest page count, shared root cause
2. Category C (6 pages) -- rouge token fixes are well-understood from issues 293, 310
3. Category D (8 pages) -- individual fixes, each small
4. Category B (2 pages) -- continuation of issue 316 work
5. Category E (2 pages) -- may require deeper investigation

## Dependencies

- Issue 316 (DTC remaining 47 pages) -- DONE or near-done. This issue continues that work.
- Issue 308 (book comments newline_to_br) -- DONE
- Issue 293 (rouge token class mapping) -- DONE
- Issue 305 (JSONLD description) -- DONE

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `./scripts/cargo-safe test` passes with all existing tests plus new tests
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes (no new warnings)
- [ ] `cargo fmt --check` passes
- [ ] DTC DOM match reaches 780+/790 (must fix at least 35 of the 44 remaining pages)
- [ ] If 790/790 is not achieved, follow-up issues are created for every remaining page with a clear description of the diff
- [ ] No regressions on muan-blog (must remain 2172+/2218)
- [ ] No regressions on mlwiki (must remain 560+/644)
- [ ] No regressions on sites currently at 100% (lanyon, minima, choosealicense, etc.)
- [ ] Tests include non-ASCII/Unicode content (accented names in book comments, CJK text)
- [ ] At least 15 new test functions covering the categories fixed

## Test Scenarios

### Unit: Nested list continuation with `<br />`
- Input through `newline_to_br | markdownify`: `1. Question<br />\n   - Point A<br />\n   - Point B<br />\n2. Follow-up`
- Verify: `<ul>` is a CHILD of the first `<li>`, not a sibling of `<ol>`
- Verify: single `<ol>` wraps both numbered items

### Unit: Numbered list continuation after `<br />`
- Input through `newline_to_br | markdownify`: `1. First<br />\n2. Second<br />\n3. Third`
- Verify: single `<ol>` with 3 `<li>` items (not three separate `<ol>` elements)

### Unit: Smart quote alignment
- Input: `He said "hello" and she said 'goodbye'`
- Verify: quote characters match kramdown's smart quote output exactly
- Include edge cases: apostrophes in contractions (`don't`), quotes after punctuation

### Unit: SQL syntax highlighting
- Input: fenced code block with `SELECT * FROM users WHERE id = 1`
- Verify: `SELECT`, `FROM`, `WHERE` get class `k` (Keyword), not `n` (Name)

### Unit: Shell syntax highlighting
- Input: fenced code block with `bash` language, containing common commands
- Verify: token classes match Jekyll's rouge output

### Unit: JSONLD description edge cases
- Input: page with author bio containing special characters (newlines, quotes, HTML entities)
- Verify: JSON-LD description field matches Jekyll's encoding and truncation

### Unit: IAL `{:target="_blank"}` on links
- Input: `[link text](url){:target="_blank"}`
- Verify: `<a href="url" target="_blank">link text</a>` output

### Unit: Unicode in book comments (required per project memory)
- Input: `1. Empfehlung von Munstermann<br />\n   - Recce bei Neuberger Berman<br />\n2. Danke schon!`
- Verify: accented characters preserved, nested list structure correct

### Integration: DTC full site build and DOM comparison
- Build DTC with rustkyll in release mode
- Run DOM comparison against Jekyll cached output
- Verify match count is 780+ out of 790
- Spot-check at least 5 previously-failing pages:
  - `books/20210222-ml-algotrading-2ed.html` -- nested list in comments
  - `books/20220912-skills-of-successful-software-engineer.html` -- numbered list continuation
  - `blog/machine-learning-zoomcamp.html` -- include rendering
  - `blog/do-you-know-golden-rules.html` -- SQL syntax highlighting
  - A podcast page with `<br>` diffs

### Regression: Other sites
- Run DOM comparison on muan-blog, mlwiki, choosealicense
- Verify no regression on any site
- Run `./scripts/cargo-safe test` full suite

## Output Verification

```bash
./scripts/cargo-safe build --release
./target/release/rustkyll build \
  --source websites/DataTalksClub/datatalksclub.github.io/ \
  --destination /tmp/dtc_325

uv run scripts/dom_compare.py \
  --jekyll-dir websites/DataTalksClub/datatalksclub.github.io/_site_jekyll_cached \
  --rustkyll-dir /tmp/dtc_325
```

Expected: "780+ files matched" (or ideally 790/790).

Spot-checks on specific pages:
```bash
# Nested list in book comments -- must show <ul> inside <li>
grep -A10 'nate8020\|Aleix' /tmp/dtc_325/books/20210222-ml-algotrading-2ed.html | head -20

# SQL highlighting -- SELECT must have class="k"
grep 'SELECT' /tmp/dtc_325/blog/do-you-know-golden-rules.html | head -5

# JSONLD -- description field must not have trailing newline
grep -o '"description":"[^"]*"' /tmp/dtc_325/blog/*.html | head -5
```

## Log

### [SWE] 2026-03-23

**Starting state:** 745/790 (94%)

**Fixes implemented (TDD for each):**

1. **URL `>` encoding fix (books/20210426)** -- `%3E` decoded to `&gt;` in `decode_url_for_jekyll_compat` to match kramdown's HTML escaping
   - Test: `test_325_greater_than_decoded_to_html_entity_in_url`, `test_325_greater_than_in_oreilly_url`

2. **Ellipsis in pipe table cells (books/20220627)** -- Added `apply_typographic_symbols` to convert `...` to `...` (U+2026) in pipe table cell content
   - Test: `test_325_pipe_table_typographic_symbols`, `test_325_apply_typographic_symbols`

3. **Dash preprocessing (books/20241118 + others)** -- Preprocess 2+ dash sequences to match kramdown's greedy algorithm (--- = em-dash, -- = en-dash) before pulldown-cmark. Handles code spans and markdown link URLs correctly.
   - Test: `test_325_preprocess_kramdown_dashes`, `test_325_markdownify_four_dashes`, `test_325_markdownify_triple_dashes_em_dash`, `test_325_markdownify_triple_dashes_after_br`, `test_325_markdownify_multiple_triple_dashes_long_text`

4. **HTML escaping in pipe table cells (books/20221010)** -- Escape `<` and `>` to `&lt;`/`&gt;` in pipe table cell content
   - Test: `test_325_pipe_table_html_escapes_angle_brackets`

5. **SQL table alias classification (blog/important-sql-fact)** -- Added `c` to SQL_NAME_TO_KEYWORD list to match Rouge's classification
   - Test: `test_325_sql_table_alias_c_is_k`

6. **Math protection: currency $ fix (blog/mlops-zoomcamp + 1 more)** -- Skip closing `$` followed by digit in `protect_math_content` to prevent `$2.19 billion...$16.6 billion` from being matched as inline math
   - Test: `test_325_bold_text_with_dollar_sign`

7. **Compilation fixes** -- Fixed missing `sort_by` field in test `CollectionConfig` initializers (pre-existing build errors in `src/collection.rs` and `src/config.rs`)

**Final state:** 751/790 (95%) -- 6 pages flipped (from 745 to 751)

**Pages fixed:**
- books/20210426-tiny-python-projects.html (URL encoding)
- books/20220627-designing-machine-learning-systems.html (ellipsis in pipe table)
- books/20241118-why-data-science-projects-fail.html (dash conversion)
- books/20221010-managing-machine-learning-projects.html (HTML escaping in pipe table)
- blog/mlops-zoomcamp.html (math protection)
- blog/how-do-data-professionals-use-data-engineering-tools-and-practices.html (math protection)

**Remaining 39 diffs (not fixed):**
- 7 JSONLD author description pages (requires render-order emulation; too risky)
- ~20 book comment list continuation pages (Category A: <br/> before lists; broad fix caused 65+ regressions)
- 1 alt attribute newline page (pulldown-cmark normalizes newlines in inline HTML attributes)
- 1 smart quote direction page
- 3 events/index/people pages (time-dependent data)
- 7 syntax highlighting pages (Python/Shell/YAML; complex token mapping)

**Build:** 2647 lib tests pass, 0 fail. Clippy clean. Fmt clean.
**Files modified:** src/frontmatter.rs, src/kramdown.rs, src/syntax.rs, src/collection.rs, src/config.rs, src/generator.rs

**Known limitations:**
- Category A (book comment list continuation after <br/>) requires deep changes to markdownify pipeline; too risky for this issue
- JSONLD author description requires emulating Jekyll's render ordering (raw vs rendered content); high regression risk
- Some remaining diffs need site-specific investigation
