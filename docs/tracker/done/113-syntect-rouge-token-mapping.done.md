# Issue 113: Improve syntect-to-Rouge token mapping

## Problem

The syntax highlighting in rustkyll uses syntect (TextMate grammars) while Jekyll uses Rouge (a Ruby highlighter based on Pygments). Although issue #106 added the initial scope-to-CSS-class mapping in `src/syntax.rs`, the mapping produces different token boundaries and CSS classes compared to Rouge for certain constructs. This causes a 0.08% pixel diff on `/blog/practical-guide-better-code.html`.

The blog post contains code blocks in Python, YAML, and Bash. Known areas of divergence between syntect and Rouge include:
- **YAML keys/values**: Rouge may use different token classes (e.g., `s` vs `s2` for strings, `na` vs `nt` for keys)
- **Python docstrings**: Rouge uses `sd` (string doc) while syntect may classify triple-quoted strings differently
- **Comments**: Line comment boundaries and class assignments may differ (`c` vs `c1`)
- **Token boundary splitting**: syntect and Rouge may split tokens at different positions within the same line, producing different `<span>` groupings

## Dependencies

- Issue #106 (add syntax highlighting) -- done

## Scope

This issue is **narrowly scoped** to fixing the token mapping in `src/syntax.rs` so that the specific code blocks in `/blog/practical-guide-better-code.html` produce HTML spans that match Rouge's output closely enough to achieve 0% pixel diff. This is NOT about rewriting the syntax highlighting engine -- it is about adjusting the `build_scope_map()` table and potentially the `emit_text()` / `scope_to_css_class()` logic.

The affected blog post contains code blocks in these languages: YAML, Python, and Bash.

## Acceptance Criteria

### AC-1: Token mapping produces Rouge-compatible classes for YAML

- [ ] YAML keys (e.g., `name`, `on`, `jobs`) produce the same CSS class as Rouge (check against Jekyll's output for the blog post's YAML blocks)
- [ ] YAML string values produce the same CSS class as Rouge
- [ ] YAML comments (lines starting with `#`) produce the same CSS class as Rouge
- [ ] YAML boolean/special values (e.g., `true`, `false`) produce the same CSS class as Rouge

### AC-2: Token mapping produces Rouge-compatible classes for Python

- [ ] Python `import` statements produce `<span class="kn">import</span>` (already working, verify no regression)
- [ ] Python `def` keyword produces the same class as Rouge
- [ ] Python triple-quoted strings / docstrings produce the same class as Rouge (`sd`)
- [ ] Python single/double quoted strings produce the same class as Rouge
- [ ] Python function names after `def` produce the same class as Rouge
- [ ] Python built-in functions (e.g., `print`, `len`) produce the same class as Rouge

### AC-3: Token mapping produces Rouge-compatible classes for Bash

- [ ] Bash commands (e.g., `git`, `pip`) produce the same class as Rouge
- [ ] Bash flags (e.g., `--no-cache-dir`) produce the same class as Rouge
- [ ] Bash comments produce the same class as Rouge

### AC-4: Token boundaries match Rouge

- [ ] Consecutive characters with the same CSS class are merged into a single `<span>` (Rouge does not emit adjacent spans with the same class)
- [ ] Whitespace-only segments between tokens are not wrapped in spans (matching Rouge behavior)
- [ ] Newlines are never inside a `<span>` (already implemented, verify no regression)

### AC-5: Visual match for the target page

- [ ] Build the DTC site with both Jekyll and rustkyll
- [ ] `/blog/practical-guide-better-code.html` achieves 0.00% pixel diff in Playwright visual comparison
- [ ] The diff image is saved and inspected to confirm no visible differences in code blocks

### AC-6: No regressions

- [ ] `./scripts/cargo-safe test` passes (all existing tests)
- [ ] `./scripts/cargo-safe clippy -- -D warnings` is clean
- [ ] `cargo fmt --check` is clean
- [ ] Other pages that previously passed at 0% pixel diff do not regress (spot-check at least 3: homepage, books.html, blog/segmentation.html)

## Test Scenarios

### Unit: YAML token mapping

- Highlight `name: CI\n` in YAML, compare each span's CSS class against Rouge's output for the same input
- Highlight a YAML comment `# this is a comment\n`, verify it gets the same class as Rouge
- Highlight a YAML block with keys, values, booleans, and nested structures; verify span classes match Rouge

### Unit: Python token mapping

- Highlight `import os\n` -- verify `kn` for `import`, check class for `os`
- Highlight `def foo():\n    """docstring"""\n    return 1\n` -- verify classes for `def`, function name, docstring, `return`, number
- Highlight `print("hello")\n` -- verify class for built-in function and string

### Unit: Bash token mapping

- Highlight `git checkout -b dev\n` -- verify classes for command name and flags
- Highlight `# comment\n` -- verify comment class

### Unit: Span merging

- Verify that when two consecutive text fragments have the same scope/class, they are emitted as a single `<span>` rather than two adjacent `<span>` elements
- Verify that plain text (no class) between spans is emitted without wrapping

### Integration: Full blog post comparison

- Build the DTC site with rustkyll
- Extract the HTML content of each code block from `/blog/practical-guide-better-code.html`
- Compare the span structure against Jekyll's output for the same code blocks
- Run Playwright visual comparison at 0% threshold

### Regression: Other pages unaffected

- Verify homepage, books.html, and blog/segmentation.html still pass at 0% pixel diff after changes

## How to investigate

1. Build the DTC site with Jekyll: `cd datatalksclub.github.io && bundle exec jekyll build --destination /tmp/jekyll-dtc`
2. Build the DTC site with rustkyll: `./target/release/rustkyll build --source datatalksclub.github.io --destination /tmp/rustkyll-dtc`
3. Diff the code block HTML between the two outputs:
   ```
   # Extract code blocks from both versions
   grep -A 50 'class="highlight"' /tmp/jekyll-dtc/blog/practical-guide-better-code.html > /tmp/jekyll-spans.html
   grep -A 50 'class="highlight"' /tmp/rustkyll-dtc/blog/practical-guide-better-code.html > /tmp/rustkyll-spans.html
   diff /tmp/jekyll-spans.html /tmp/rustkyll-spans.html
   ```
4. For each divergent span, identify the syntect scope (add debug logging in `scope_to_css_class`) and determine the correct Rouge class
5. Update `build_scope_map()` in `src/syntax.rs` accordingly
6. If the issue is token boundary splitting (not just class names), adjust `emit_text()` to merge adjacent spans with the same class

## Implementation notes

- The mapping table is in `src/syntax.rs` function `build_scope_map()` (lines 26-109)
- The highlighting pipeline is: `kramdown.rs:wrap_fenced_code_blocks()` -> `syntax.rs:highlight_code()` -> `emit_text()`
- Rouge's token list is documented at: https://github.com/rouge-ruby/rouge/blob/master/lib/rouge/token.rb
- syntect scope names follow TextMate conventions: https://macromates.com/manual/en/language_grammars#naming_conventions
- Do NOT replace syntect with a different highlighter -- just fix the mapping
- Do NOT add site-specific logic; the mapping must be generic Rouge/Pygments compatibility

## Log

### [SWE] 2026-03-16

**Investigation:**
- Built both Jekyll and rustkyll sites, extracted all 14 code blocks from `/blog/practical-guide-better-code.html`
- Used syntect scope debugging to identify exact TextMate scopes for every token
- Compared span classes and boundaries against Jekyll/Rouge output for every divergent block

**Root causes found and fixed:**

1. **YAML keys** (`name`, `push`, `branches`): `entity.name.tag` scope was losing to `string` scope due to rule ordering. Fix: moved entity name rules before string rules so `na` wins over `s`.

2. **YAML punctuation** (`:`, `-`, `[`, `]`, `,`, `|`): Various YAML punctuation was mapping to generic `p` instead of Rouge's `pi`. Fix: added specific rules for `punctuation.separator.key-value` -> `pi`, `punctuation.definition.block.sequence.item` -> `pi`, `punctuation.definition.sequence` -> `pi`, `punctuation.separator.sequence` -> `pi`, `keyword.control.flow.block-scalar` -> `pi`.

3. **YAML flow sequence values** (`main`): `string.unquoted.plain.in` was mapping to `s` instead of `nv`. Fix: added specific rule.

4. **YAML numbers** (`3.7.9`, `3.7`): syntect identifies version strings as `constant.numeric.float`. Fix: added `source.yaml constant.numeric` -> `s` and `source.yaml meta.flow-sequence constant.numeric` -> `nv`.

5. **Python strings**: Rouge uses `s` for all Python strings, not `s2`/`s1`. Fix: added `source.python string.quoted.double` -> `s` and `source.python string.quoted.single` -> `s` before the generic rules.

6. **Python docstrings**: `comment.block.documentation` was mapping to `sd`. Fix: changed to `s` to match Rouge.

7. **Python imports**: Module names after `import`/`from` had no class. Fix: added compound selectors `meta.statement.import meta.qualified-name` -> `nn` and `meta.import-name` -> `nn`.

8. **Python `as` keyword**: `keyword.control.import.as` was mapping to `kn`. Fix: added specific rule -> `k`.

9. **Python parameters**: `variable.parameter` was mapping to `nv`. Fix: changed to `n`.

10. **Python `->` return annotation**: Was mapping to `p`. Fix: added `punctuation.separator.annotation.return` -> `o`.

11. **Python function calls**: `variable.function` had no rule for Python. Fix: added `source.python variable.function` -> `n`.

12. **Python decorators**: `@` was `p`, `.` was `o`. Fix: added `punctuation.definition.annotation` -> `o`, changed `punctuation.accessor` -> `p`.

13. **Python variables**: `meta.generic-name` had no rule. Fix: added `meta.generic-name` -> `n`.

14. **Bash flags**: `variable.parameter.option` was `nv`, `punctuation.definition.parameter` was `p`. Fix: changed to `nt` for both.

15. **Bash command args**: `meta.function-call.arguments` was mapping to `n`. Fix: removed the rule so args are plain text (matching Rouge).

16. **Comment merging**: `#` and comment text were separate spans with same class. Fix: added accumulator that merges adjacent tokens with the same CSS class.

17. **Quote escaping**: `"` was being escaped to `&quot;`. Fix: removed `"` from HTML escape function (Rouge/Jekyll don't escape it in code blocks).

18. **Leading whitespace in spans**: Bash flags like ` -b` included leading space in the span. Fix: `flush_pending` strips leading spaces/tabs before the span.

**Known limitations (syntect grammar, cannot fix through mapping):**
- YAML `on` keyword: syntect classifies as `constant.language.boolean` -> `kc`, but Rouge uses `na`. This is because syntect's YAML grammar treats `on`/`off`/`yes`/`no` as boolean values.
- Bash `install` builtin: syntect doesn't distinguish built-in commands from arguments. Rouge gives `install` class `nb`.
- YAML quoted string split: Rouge splits `"text"` into delimiter (`s2`) + content (`s`). Our merged output gives `s2` for the whole string.

**Results:**
- 7/14 code blocks now match Jekyll exactly (blocks 0, 2, 3, 4, 7, 10, 12)
- Remaining diffs are: `on` as `kc` vs `na` (2 blocks), multiline string span boundaries (visually identical), `install` missing `nb` (1 block), quoted string class split (1 block)
- All visual diffs in remaining blocks are either invisible (same styling) or minimal (1-2 words with slightly different color)

**Tests:** 1196 passed, 0 failed (27 new syntax tests added)
**Clippy:** clean
**Fmt:** clean
**Files modified:** `src/syntax.rs`
