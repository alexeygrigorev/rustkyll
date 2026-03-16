# Issue 160: Fix blog/important-sql-fact-that-everyone-should-know.html (411 diffs)

Second highest DOM diff blog post. Investigate and fix rendering differences. TDD per pattern.

## Log

### [SWE] 2026-03-16

#### Analysis
Built both Jekyll and rustkyll, ran dom_compare on the blog post. Categorized all 411 diffs:
- 190 diffs: Missing spans (bare text in SQL code blocks where Jekyll/Rouge wraps every token)
- 114 diffs: Text content misalignment caused by missing spans cascading
- 48 diffs: `no` vs `n` CSS class on SQL identifiers (syntect constant.other vs Rouge name)
- 7 diffs: `nb` vs `k` CSS class on SQL functions like COUNT (syntect builtin vs Rouge keyword)
- 7 diffs: figcaption missing `<p>` wrapping (already fixed by uncommitted issue 158 changes)
- 4 diffs: `m` vs `mi` CSS class on SQL numbers
- 1 diff: JSON-LD timezone (+00:00 vs +02:00)

#### Root causes
1. Syntect's SQL grammar leaves many tokens unscoped (punctuation, identifiers, some keywords like IS)
2. Syntect maps SQL aggregate functions to `support.function` (-> `nb`) but Rouge maps them to keywords (`k`)
3. Syntect maps SQL identifiers to `constant.other` (-> `no`) but Rouge uses `n`
4. Syntect emits multi-word keywords like `LEFT JOIN` as one token; Rouge splits them
5. Syntect uses generic `constant.numeric` for SQL numbers (-> `m`); Rouge uses `mi`

#### Fixes applied (TDD: failing test -> fix -> pass)

**Fix 1: SQL scope mapping overrides** (src/syntax.rs)
- `source.sql support.function` -> `k` (functions like COUNT, SUM treated as keywords)
- `source.sql constant.other.database-name` -> `n` (identifiers)
- `source.sql constant.other.table-name` -> `n` (identifiers)
- `source.sql constant.other` -> `n` (generic SQL identifiers)
- `source.sql constant.numeric` -> `mi` (integer numbers)

**Fix 2: SQL post-processing** (src/syntax.rs)
- Added `postprocess_sql_highlighting()` function that wraps bare text tokens in SQL output
- Known SQL keywords (IS, IN, NOT, BETWEEN, etc.) -> `<span class="k">`
- Punctuation `(`, `)`, `.`, `,`, `;` -> `<span class="p">`
- Star `*` -> `<span class="o">`
- Other word tokens -> `<span class="n">`
- Splits multi-word keyword spans (e.g. `LEFT JOIN` -> `LEFT` + `JOIN`)

**Fix 3: Clippy fixes** (src/syntax.rs, src/kramdown.rs)
- Fixed manual strip_prefix pattern
- Removed unused html_escape_char function
- Fixed unused enumerate index

**Fix 4: Updated test for Python print** (pre-existing issue from uncommitted changes)
- test_python_builtin_is_nb updated to expect `k` (matching Rouge post-processing)

#### Results
- Diffs reduced: 411 -> 13 (96.8% reduction)
- Remaining 13 diffs:
  - 12x class `k` vs `n` for single-char alias `c` (Rouge quirk, cosmetic only)
  - 1x JSON-LD timezone difference (+02:00 vs +00:00, pre-existing)
- Tests: 1530 passed, 0 failed
- Clippy: clean (0 warnings with -D warnings)
- Fmt: clean

#### Files modified
- src/syntax.rs: SQL scope overrides, SQL post-processing, keyword splitting, tests updated
- src/kramdown.rs: clippy fix (unused enumerate index)
- docs/tracker/160-fix-blog-sql-fact.in-progress.md: this log
