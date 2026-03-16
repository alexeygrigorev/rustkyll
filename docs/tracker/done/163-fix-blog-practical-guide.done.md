# Issue 163: Fix blog/practical-guide-better-code.html (315 diffs)

Fourth highest DOM diff blog post. Investigate and fix rendering differences. TDD per pattern.

## Log

### [SWE] 2026-03-16

- Built both Jekyll and rustkyll sites from scratch and compared output
- Initial diff: 186 line-level diff lines, 259 DOM differences
- Identified 11 diff patterns across syntax highlighting, list indentation, blockquote formatting, and JSON-LD

**Fixes implemented:**

1. **Syntax highlighting: trailing newlines in spans** (kramdown.rs)
   - Rouge/Jekyll emits `<span class="c1"># comment</span>\n` (newline outside)
   - Syntect was emitting `<span class="c1"># comment\n</span>` (newline inside)
   - Fixed: added `.replace("\n</span>", "</span>\n")` post-processing in `wrap_fenced_code_blocks`
   - Affects YAML comments, YAML strings, Python docstrings

2. **YAML `on` keyword classification** (syntax.rs)
   - Syntect classifies `on` as `kc` (constant.language/boolean)
   - Rouge treats it as `na` (name attribute) when used as mapping key
   - Fixed: post-processing replacement in YAML-specific block

3. **Bash `install` builtin** (syntax.rs)
   - Rouge wraps `install` in `<span class="nb">install </span>`
   - Syntect leaves it as plain text
   - Fixed: `postprocess_bash_install` function

4. **Tight list `<li>` indentation** (kramdown.rs)
   - kramdown indents `<li>` by 2 spaces in tight lists
   - pulldown-cmark produces no indent
   - Fixed: `indent_list_items` now adds 2-space indent for tight lists

5. **Blockquote `<p>` indentation** (kramdown.rs)
   - kramdown indents `<p>` by 2 spaces inside `<blockquote>`
   - Fixed: `indent_blockquote_content` function adds 2-space indent and removes blank lines

6. **Collection item content trailing whitespace** (generator.rs)
   - `author.content` had trailing newline causing `"Data Scientist\n"` in JSON-LD
   - Fixed: `trim_end()` on html_content before storing in template context

**Results:**
- Line-level diff: 186 -> 78 (58% reduction)
- DOM differences: 259 -> 254 (note: many remaining diffs are from per-line span splits in Python docstrings)
- Tests: 1319 unit tests pass, 0 failures
- Clippy: clean (0 warnings)
- Fmt: clean

**Remaining diffs (not fixed in this issue):**
- Python multi-line docstring span structure (Jekyll keeps one span, rustkyll splits per line)
- JSON-LD timezone (+01:00 vs +00:00) -- environment-specific, Jekyll uses local TZ
- Liquid template whitespace in JSON-LD (empty conditional branches produce extra whitespace)
- Blank line after `<!--more-->` separator

**Files modified:**
- `src/syntax.rs` - YAML `on` keyword fix, bash `install` builtin, trailing newline post-processing function
- `src/kramdown.rs` - tight list indentation, blockquote indentation, trailing newline fix in code blocks
- `src/generator.rs` - trim trailing whitespace from collection item content
