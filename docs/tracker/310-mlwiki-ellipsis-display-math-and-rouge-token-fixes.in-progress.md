# Issue 310: mlwiki ellipsis in display math regression + cross-site rouge token mapping

## Problem

mlwiki.org has 313 diff pages. After filtering the 176 Jekyll math bugs
(issue 309), approximately 137 pages remain with real diffs. Two fixable
categories account for 41+ pages:

### Bug A: Ellipsis conversion applied to display math -- 10 pages (ONLY this diff) + 17 pages (this + other diffs)

Issue 302 added ellipsis conversion (`...` to U+2026) in
`restore_math_content_impl()` in `src/frontmatter.rs`. This correctly
matches Jekyll's kramdown behavior for inline math (`$...$`) and regular
text. However, it also applies to display math (`$$...$$`), which Jekyll
does NOT convert.

Example (Bayes_Theorem.html):
- Source: `$$P(H_1) + ... + P(H_n)$$`
- Jekyll output: `\[P(H_1) + ... + P(H_n)\]` (three ASCII dots preserved)
- Rustkyll output: `\[P(H_1) + ... + P(H_n)\]` (Unicode ellipsis U+2026)

Root cause: `restore_math_content_impl` applies `.replace("...", "\u{2026}")`
to ALL math content without distinguishing inline from display math. The
`math_saved` vector stores both inline and display math content, but the
restoration does not differentiate.

Fix: Track whether each saved math entry is inline (`$...$`) or display
(`$$...$$`). Apply ellipsis conversion only to inline math entries.

### Bug B: Rouge syntax highlighting token class mismatches -- 31 pages

Syntect's scope-to-CSS-class mapping differs from Ruby Rouge for several
languages. Issue 293 fixed PHP-specific mappings, but many other languages
are affected:

**Python (12 pages):** Builtin functions (`print`, `len`, `range`) map to
`nb` (Name.Builtin) in Rouge but `n` (Name) in syntect. Keywords like
`class`, `def` may also differ.

**Java (7 pages):** `new` keyword maps to `o` (Operator) instead of `k`
(Keyword). Class names after `new` map to `nb` instead of `nc` (Name.Class).
Issue 300 fixed this for JavaScript but not Java.

**SQL (6 pages):** SQL keywords like `SELECT`, `FROM`, `WHERE`, `JOIN` map
to `n` (Name) instead of `k` (Keyword). This also affects DTC's 6 syntax
highlighting diff pages.

**XML (3 pages):** Tag names map to different classes (`nt` vs `p` for
punctuation, `na` vs `s` for attributes).

**Bash (2 pages):** Shell keywords and variable references differ.

**Plaintext (12 pages):** Pages with `language-plaintext` somehow have
highlighting class differences. These may be false positives from the DOM
comparison tool detecting other differences.

Root cause: The `build_scope_map()` function in `src/syntax.rs` maps
TextMate/Sublime scopes to Rouge CSS classes. The mapping is incomplete
for many languages. Language-specific scope overrides are needed (similar
to the PHP fix in issue 293 and the JS fix in issue 300).

## Scope

### In scope

1. **Fix ellipsis in display math** -- modify `protect_math_content()` to
   track inline vs display math entries, and modify `restore_math_content_impl()`
   to only apply ellipsis conversion for inline math.

2. **Fix Python rouge token mapping** -- add Python-specific scope overrides
   for builtin functions (`nb`), keywords (`k`), and decorators.

3. **Fix SQL rouge token mapping** -- add SQL-specific scope overrides for
   keywords (`k` for `SELECT`, `FROM`, `WHERE`, etc.). This also fixes 6
   DTC pages.

4. **Fix Java rouge token mapping** -- add Java-specific scope overrides
   for `new` keyword (`k`), class names (`nc`), and integer literals (`mi`).

### Out of scope

- XML token mapping (3 pages) -- lower priority, can be follow-up
- Bash token mapping (2 pages) -- lower priority
- Plaintext highlighting diffs (12 pages) -- need investigation, may be
  false positives
- Markdown parsing structural diffs (47 pages) -- fundamental pulldown-cmark
  vs kramdown differences, tracked separately
- Jekyll math bugs (176 pages) -- tracked by issue 309

## Dependencies

- Issue 302 (mlwiki ellipsis/braces) -- DONE. This issue fixes a regression
  from that work.
- Issue 293 (rouge token mapping) -- DONE. This issue extends that work to
  more languages.
- Issue 300 (lanyon rouge JS fixes) -- DONE. Provides the pattern for
  language-specific overrides.

## Key Files to Modify

- `src/frontmatter.rs` -- `protect_math_content()` to track inline vs
  display math, `restore_math_content_impl()` to conditionally apply
  ellipsis conversion
- `src/syntax.rs` -- `build_scope_map()` to add Python, SQL, and Java
  language-specific scope overrides

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new tests below
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes
- [ ] Bug A: `...` inside `$$...$$` display math is NOT converted to Unicode
      ellipsis (stays as three ASCII dots)
- [ ] Bug A: `...` inside `$...$` inline math IS still converted to Unicode
      ellipsis (issue 302 behavior preserved)
- [ ] Bug A: `...` in regular text IS still converted to Unicode ellipsis
      (smart punctuation preserved)
- [ ] Bug B: Python `print()`, `len()`, `range()` get `class="nb"` (not `"n"`)
- [ ] Bug B: SQL `SELECT`, `FROM`, `WHERE`, `JOIN` get `class="k"` (not `"n"`)
- [ ] Bug B: Java `new` keyword gets `class="k"` (not `"o"`)
- [ ] Bug B: Java class names after `new` get `class="nc"` (not `"nb"`)
- [ ] mlwiki DOM match improves by 10+ pages (from 331 to 341+, before
      issue 309 filtering; or from 507 to 520+ after issue 309 filtering)
- [ ] DTC DOM match improves by 2+ pages (SQL highlighting fixes on
      `important-sql-fact.html` and `do-you-know-golden-rules.html`)
- [ ] No regressions on any of the 13+ sites currently at 100%
- [ ] No regressions on muan-blog, lanyon, or choosealicense match counts
- [ ] Tests include non-ASCII/Unicode content (e.g., Python with CJK
      variable names, SQL with Unicode string literals)

## Test Scenarios

### Unit: Display math ellipsis preservation (Bug A)

- Parse `$$A + ... + Z$$` through `markdown_to_html`, verify output contains
  `\[A + ... + Z\]` (three ASCII dots, NOT Unicode ellipsis)
- Parse `$A, B, C, ...$` through `markdown_to_html`, verify output contains
  Unicode ellipsis (inline math still converts -- issue 302 preserved)
- Parse `Hello... world` through `markdown_to_html`, verify output contains
  Unicode ellipsis (regular text still converts)
- Parse `$$\sum_{i=1}^{...} x_i$$` through `markdown_to_html`, verify
  `...` preserved as three dots
- Parse mixed content: `$a, ..., z$ and $$A + ... + Z$$`, verify inline
  math has ellipsis but display math has three dots
- Parse with Unicode: `$$\alpha + ... + \omega$$`, verify three dots preserved

### Unit: Python rouge token mapping (Bug B)

- Highlight `print("hello")` as Python, verify `print` gets `class="nb"`
- Highlight `len(x)` as Python, verify `len` gets `class="nb"`
- Highlight `class MyClass:` as Python, verify `class` gets `class="k"`
- Highlight `def func():` as Python, verify `def` gets `class="k"`
- Highlight `x = range(10)` as Python, verify `range` gets `class="nb"`
- Highlight Python with Unicode: `print("\u4e16\u754c")`, verify token classes

### Unit: SQL rouge token mapping (Bug B)

- Highlight `SELECT name FROM users WHERE id = 1` as SQL, verify `SELECT`,
  `FROM`, `WHERE` all get `class="k"`
- Highlight `JOIN`, `GROUP BY`, `ORDER BY`, `HAVING` as SQL, verify all
  get `class="k"`
- Highlight `COUNT()`, `SUM()`, `AVG()` as SQL, verify they get appropriate
  builtin class
- Highlight SQL with Unicode string: `SELECT * FROM t WHERE name = 'cafe'`,
  verify token classes

### Unit: Java rouge token mapping (Bug B)

- Highlight `new ArrayList()` as Java, verify `new` gets `class="k"` and
  `ArrayList` gets `class="nc"`
- Highlight `int x = 42;` as Java, verify `42` gets `class="mi"`
- Highlight `public class Main` as Java, verify `public` gets `class="k"`
  and `Main` gets `class="nc"`
- Highlight Java with Unicode: `String s = "\u00e9t\u00e9";`, verify classes

### Integration: mlwiki page rendering

- Build mlwiki.org with rustkyll
- Run DOM comparison against Jekyll cached output
- Verify Bayes_Theorem.html now matches (ellipsis fixed)
- Verify ANTLR4_Maven.html diff count reduced (XML rouge, may not be fully
  fixed if XML is out of scope)
- Spot-check a Python-heavy page for improved rouge token matching

### Integration: DTC page rendering

- Build DTC site with rustkyll
- Run DOM comparison
- Verify `blog/important-sql-fact-that-everyone-should-know.html` has
  reduced diff count (SQL `k` vs `n` fixed)
- Verify `blog/do-you-know-golden-rules-while-working-with-data.html` has
  reduced diff count

### Regression: Other sites

- Run `cargo test` full suite
- Verify all 13+ sites at 100% remain at 100%
- Verify muan-blog, lanyon, choosealicense match counts unchanged or improved
- Specifically verify that the display math ellipsis fix does not break
  kramdown conformance tests (656+/658 must be maintained)

## Output Verification

```bash
./scripts/cargo-safe build --release

# Test mlwiki
./target/release/rustkyll build \
  --source websites/alexeygrigorev/mlwiki.org/ \
  --destination /tmp/mlwiki_310

python3 scripts/dom_compare.py \
  --jekyll-dir websites/alexeygrigorev/mlwiki.org/_site_jekyll_cached \
  --rustkyll-dir /tmp/mlwiki_310

# Ellipsis check: Bayes_Theorem should have ... (three dots) in display math
grep '\.\.\.' /tmp/mlwiki_310/index.php/Bayes_Theorem.html | head -3
# Must show: ... + P(H_n)  (three ASCII dots, NOT Unicode ellipsis)

# Test DTC
./target/release/rustkyll build \
  --source websites/DataTalksClub/datatalksclub.github.io/ \
  --destination /tmp/dtc_310

python3 scripts/dom_compare.py \
  --jekyll-dir websites/DataTalksClub/datatalksclub.github.io/_site_jekyll_cached \
  --rustkyll-dir /tmp/dtc_310

# SQL check: important-sql-fact should have class="k" for SQL keywords
grep 'class="k"' /tmp/dtc_310/blog/important-sql-fact-that-everyone-should-know.html | head -3
# Must show SELECT, FROM, WHERE with class="k"
```

## Log

### [SWE] 2026-03-23

**TDD cycle:**

**Bug A (display math ellipsis):**
- Wrote 6 failing tests in kramdown.rs: display math ellipsis preserved, inline still converts, regular text still converts, sum pattern, mixed inline+display, Unicode content
- Ran tests: 4 FAILED as expected (display math tests show Unicode ellipsis where three dots expected)
- Implemented fix: changed `protect_math_content()` to return `Vec<MathEntry>` with `is_inline` flag; changed `restore_math_content_impl()` to only apply `...` -> ellipsis for inline math entries
- Ran tests: all 6 PASS

**Bug B (Java class name after new):**
- Wrote 1 failing test: `test_issue310_java_class_name_after_new_is_nc`
- Ran test: FAILED as expected (got `nb` for ArrayList, expected `nc`)
- Implemented fix: added `postprocess_java_new_class_names()` that replaces `nb` with `nc` after `<span class="k">new</span>`
- Ran test: PASSES

**Bug B (Python, SQL, Java other):**
- Wrote 14 additional tests for Python builtins/keywords, SQL keywords, Java keywords/integers/public
- Ran tests: all 14 PASSED already (existing scope rules handle these correctly)

**Results:**
- 20 new tests, all passing
- Full suite: 2602 tests pass, 0 fail, 2 ignored
- Clippy clean, fmt clean

**Files modified:**
- `src/frontmatter.rs` -- Added `MathEntry` struct, changed `protect_math_content` to track inline vs display, changed `restore_math_content_impl` to only apply ellipsis for inline math
- `src/syntax.rs` -- Added `postprocess_java_new_class_names()`, added Java post-processing step, added 14 test functions
- `src/kramdown.rs` -- Added 6 test functions for display math ellipsis
