# Issue 158: Fix blog/open-source-free-ai-agent-evaluation-tools.html (545 diffs)

Top DOM diff blog post. Investigate and fix rendering differences. TDD per pattern.

## Log

### [SWE] 2026-03-16

**Analysis:** Starting diff was 506 lines (Jekyll vs rustkyll). Categorized into 7 patterns:
1. figcaption `<p>` stripping (16 diffs) -- rustkyll was incorrectly stripping `<p>` from `<figcaption>`
2. Code block `</pre></div></div>` splitting (48 diffs) -- `add_block_spacing` was breaking code wrapper closing tags across lines
3. Blank line after `</figure>` (16 diffs) -- `</figure>` not in block spacing tags list
4. List indentation (25 diffs) -- Jekyll indents loose list items, rustkyll didn't
5. Python syntax: dict colon `pi` vs `p` (14 diffs) -- YAML rule matching Python dict colons
6. Python syntax: module dot splitting (9 diffs) -- syntect splits dotted module names
7. Python syntax: `print`/`input` classification (7 diffs)

**Fixes applied:**
1. Removed `figcaption` from `STRIP_P_PARENT_TAGS` -- source HTML has `<figcaption><p>...</p></figcaption>`, Jekyll preserves it
2. Added guard in `add_block_spacing` to not add newlines between `</pre></div></div>` in code block wrappers
3. Added `</figure>` to block spacing tags list
4. Implemented `indent_list_items()` in kramdown postprocess pipeline -- indents loose list items with 2/4 spaces
5. Added `source.python punctuation.separator.key-value -> p` scope override (before YAML `pi` rule)
6. Added `merge_python_dotted_modules()` post-processor to merge `<span class="nn">X</span><span class="p">.</span><span class="nn">Y</span>` into `<span class="nn">X.Y</span>`
7. Added Python post-processing: `print` as `k` (matching Rouge), `input` as `nb` (matching Rouge)
8. Protected `<p>` inside `<figcaption>` from being stripped when inside `<figure>` (updated `maybe_strip_p_tags`)

**Result:** Diffs reduced from 506 to 325 lines (36% reduction). Of remaining 325 lines:
- ~160 lines are FAQ section indentation (template engine difference, not kramdown)
- ~60 lines are YAML/Bash syntax highlighting differences (syntect vs Rouge tokenization)
- ~40 lines are whitespace/blank line differences in template-generated HTML
- Remaining are minor syntax highlighting variants

**Tests:** 11 new tests for issue 158 + 2 updated existing tests. All 1304 lib tests pass, 0 failures. Clippy clean, fmt clean.

**Files modified:**
- `src/kramdown.rs` -- figcaption fix, code block div fix, list indentation, figure block spacing
- `src/syntax.rs` -- Python dict colon, module dot merging, print/input classification
