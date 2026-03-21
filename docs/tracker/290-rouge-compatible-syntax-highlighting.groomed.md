# Issue 290: Rouge-compatible syntax highlighting token mapping

## Problem

Syntax highlighting token class differences are the largest remaining DOM difference category across multiple sites. Rustkyll uses syntect (Sublime Text grammars) while Jekyll uses Rouge (Pygments-based). They produce different CSS class names for the same code tokens.

`src/syntax.rs` (2221 lines) already has extensive language-specific mappings for Ruby, JSON, YAML, JavaScript, SQL, Python, Bash, and XML. However, several token mismatches remain.

## Impact

**10 theme sites all share the same Ruby code block example.** Fixing the Ruby token mapping would bring all 10 from 50% to 100%:
- architect-theme (1/2 -> 2/2)
- cayman-theme (1/2 -> 2/2)
- dinky-theme (0/2 -> 2/2)
- hacker-theme (0/2 -> 2/2)
- leap-day-theme (0/2 -> 2/2)
- merlot-theme (0/2 -> 2/2)
- midnight-theme (0/2 -> 2/2)
- primer-theme (0/2 -> 2/2)
- slate-theme (1/2 -> 2/2)
- time-machine-theme (0/2 -> 2/2)

Additional impact:
- lanyon: 4/6 -> 6/6 (partially syntax highlighting)
- mlwiki.org: 236/639 -> improved (116 syntax class diffs)
- mlbookcamp-page: 6/15 -> improved
- DTC: minor improvement (~29 class diffs)

Estimated total: +10 to +15 newly matching pages minimum (theme sites alone), plus partial improvements across other sites.

## Specific token mismatches to fix

### Ruby (highest priority -- fixes 10 theme sites)

All 10 theme sites use the same Ruby code block. The mismatches are:

| Expected (Rouge) | Actual (rustkyll) | Count per site | What it is |
|---|---|---|---|
| `o` (Operator) | `p` (Punctuation) | 3 | Likely `(`, `)`, or `.` classified differently |
| `nf` (Name.Function) | `n` (Name) | 3 | Method/function names not recognized as functions |
| `n` (Name) | `nf` (Name.Function) | 1 | Something classified as function that shouldn't be |
| `s2` (String.Double) | `dl` (String.Delimiter) | 1 | Quote delimiter classified as delimiter instead of string |
| `si` (String.Interpol) | `s2` (String.Double) | 0-1 | Interpolation inside string not detected |

### JavaScript/TypeScript (mlwiki.org)

| Expected (Rouge) | Actual (rustkyll) | Count | What it is |
|---|---|---|---|
| `kd` (Keyword.Declaration) | `k` (Keyword) | 14 | `var`/`let`/`const`/`function` should be `kd` not `k` |
| `kd` (Keyword.Declaration) | `kt` (Keyword.Type) | 6 | Storage types misclassified |

### Python (mlwiki.org, mlbookcamp-page)

| Expected (Rouge) | Actual (rustkyll) | Count | What it is |
|---|---|---|---|
| `ow` (Operator.Word) | `k` (Keyword) | 4 | `in`, `not`, `is`, `and`, `or` should be `ow` |
| `nc` (Name.Class) | `nb` (Name.Builtin) | 4 | Class names misclassified as builtins |
| `sh` (String.Heredoc) | `s` (String) | 4 | Triple-quoted strings should be `sh` |

### General cross-language

| Expected (Rouge) | Actual (rustkyll) | Count | What it is |
|---|---|---|---|
| `mi` (Number.Integer) | `m` (Number) | 2 | Integer literals should be `mi` not generic `m` |
| `n` (Name) | `nn` (Name.Namespace) | 7 | Namespace names misclassified |

## Approach

1. Check the Ruby code block used in all 10 theme sites (it's the same code). Run Rouge on it to get expected output. Fix the 5 specific token mapping differences in `src/syntax.rs`.
2. Fix the JavaScript `kd`/`kt` mappings (already partially done for `source.js storage.type` but not catching all cases).
3. Fix Python `ow`/`nc`/`sh` mappings.
4. Fix general `mi` vs `m` and `nn` vs `n` mappings.
5. Validate by running DOM comparison on affected sites.

## Key Files

- `src/syntax.rs` -- main file to modify (2221 lines, contains scope mapping table and language-specific post-processing)
- Rouge token definitions: `/home/alexey/.rvm/gems/ruby-3.3.7/gems/rouge-4.7.0/lib/rouge/token.rb`
- Rouge Ruby lexer: `/home/alexey/.rvm/gems/ruby-3.3.7/gems/rouge-4.7.0/lib/rouge/lexers/ruby.rb`
- Rouge JavaScript lexer: `/home/alexey/.rvm/gems/ruby-3.3.7/gems/rouge-4.7.0/lib/rouge/lexers/javascript.rb`
- Rouge Python lexer: `/home/alexey/.rvm/gems/ruby-3.3.7/gems/rouge-4.7.0/lib/rouge/lexers/python.rb`

## Dependencies

None. This issue is independent of kramdown or template work.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes all existing tests (no regressions)
- [ ] All 10 theme sites reach 100% DOM match (2/2 pages each):
  - [ ] architect-theme: 2/2
  - [ ] cayman-theme: 2/2
  - [ ] dinky-theme: 2/2
  - [ ] hacker-theme: 2/2
  - [ ] leap-day-theme: 2/2
  - [ ] merlot-theme: 2/2
  - [ ] midnight-theme: 2/2
  - [ ] primer-theme: 2/2
  - [ ] slate-theme: 2/2
  - [ ] time-machine-theme: 2/2
- [ ] Zero syntax highlighting class diffs on the Ruby code blocks used in theme sites
- [ ] Reduced syntax highlighting diffs on mlwiki.org (currently 116 class diffs)
- [ ] No regressions on large-blog-3000 (3001/3001), large-docs-site (801/801), kids-horror-stories-ru (1344/1344), or DTC (657/790)
- [ ] New or updated unit tests for each token mapping fix (Ruby `o`/`p`, `nf`/`n`, `s2`/`dl`, `si`/`s2`; JavaScript `kd`/`k`; Python `ow`/`k`, `nc`/`nb`, `sh`/`s`)
- [ ] Include non-ASCII content in at least one test case
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` is clean
- [ ] Rouge MIT copyright notice preserved in source file (already present, verify not removed)

## Test Scenarios

### Unit: Ruby token mapping fixes

- Highlight a Ruby code block with method definitions and parentheses. Verify `(` and `)` are classified as `p` or `o` matching Rouge.
- Highlight Ruby code with method calls like `foo.bar(arg)`. Verify `bar` is `nf` (function name).
- Highlight Ruby string `"hello #{name}"`. Verify `"` is `s2`, `#{` is `si`, content is `s2`.
- Highlight the exact Ruby code block from the theme site examples (extract from architect-theme's index page). Verify all token classes match Rouge output.

### Unit: JavaScript token mapping fixes

- Highlight `var x = function() {}`. Verify `var` and `function` are `kd` (not `k` or `kt`).
- Highlight `let y = 5; const z = "hello"`. Verify `let` and `const` are `kd`.

### Unit: Python token mapping fixes

- Highlight `x in [1, 2, 3]`. Verify `in` is `ow` (operator.word, not `k`).
- Highlight `class MyClass:`. Verify `MyClass` is `nc` (name.class, not `nb`).
- Highlight `"""docstring"""`. Verify triple-quoted string is `sh` (string.heredoc).

### Unit: General fixes

- Highlight code with integer literal `42`. Verify class is `mi` (not `m`).

### Integration: DOM comparison

- Build all 10 theme sites with rustkyll
- Run DOM comparison against Jekyll cached output for each
- Verify 2/2 pages match on each theme site (0 total differences)
- Run DOM comparison on mlwiki.org and verify syntax highlighting diffs reduced
- Run DOM comparison on large-blog-3000 and large-docs-site to verify no regressions

## Output Verification

The engineer must:
1. Build each of the 10 theme sites: `./target/release/rustkyll build --source websites/<site> --destination websites/<site>/_site_rustkyll`
2. Run DOM comparison for each: `uv run scripts/dom_compare.py --jekyll-dir websites/<site>/_site_jekyll_cached --rustkyll-dir websites/<site>/_site_rustkyll`
3. Verify each shows: `Summary: 2 files matched, 0 files with differences, 0 total differences`
4. Run regression check on 3+ non-theme sites

## Notes

- The existing `src/syntax.rs` already has 2221 lines of mapping logic. This issue adds targeted fixes, not a rewrite.
- The 10 theme sites all use the exact same Ruby code example, so fixing one fixes all 10.
- Some diffs on non-theme sites (muan-blog, DTC) are NOT syntax highlighting -- they are quote escaping, meta tag, or other issues. Do not conflate them.
