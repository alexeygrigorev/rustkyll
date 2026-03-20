# Issue 252: Document all pulldown-cmark to kramdown postprocessing steps

## Problem

We apply numerous preprocessing and postprocessing transformations to pulldown-cmark's markdown output to match Jekyll's kramdown behavior. These are scattered across `src/kramdown.rs` and `src/frontmatter.rs` with no central documentation of what each does, why it exists, and which sites it affects.

## Goal

Create a comprehensive reference document listing every preprocessing and postprocessing step applied to bridge the gap between pulldown-cmark and kramdown. For each transformation, document:

1. **What it does** (input -> output, with a concrete example)
2. **Why** (what kramdown behavior it matches, which issue introduced it)
3. **Where** (function name, file, approximate line number)
4. **When it runs** (pre-markdown, during-markdown, post-markdown, post-layout)
5. **Which sites it affects** (kramdown only, CommonMarkGhPages only, both, or filter-only)
6. **Risk level** (Low/Medium/High -- does it ever over-apply, cause regressions, or have known edge cases?)

## Scope

The document must cover ALL of the following transformation categories. The SWE must read the actual source code and catalog every step -- do not rely on this list alone, as it may be incomplete.

### A. Pre-markdown transformations (applied to markdown source before pulldown-cmark parsing)

These run in `frontmatter::markdown_to_html()` and `frontmatter::markdown_to_html_with_options()`:

1. `kramdown::process_markdown_attribute` -- processes `markdown="1"` on HTML elements (Issue 228)
2. `protect_preexisting_curly_quotes` / `restore_preexisting_curly_quotes` -- protects pre-existing Unicode curly quotes from re-processing
3. `escape_paren_list_markers` -- escapes `1) text` patterns since kramdown only uses `.` delimiter
4. `protect_math_content` / `restore_math_content` -- protects `$...$` and `$$...$$` from backslash-escape stripping (Issue 227)
5. `kramdown::escape_headings_in_list_context` -- escapes `#` headings inside list items to match kramdown behavior (Issue 204)
6. `kramdown::collapse_blank_lines_between_list_items` -- collapses partially-loose lists to tight (Issue 204)
7. `kramdown::convert_kramdown_pipe_tables` -- converts kramdown pipe-table syntax to HTML (Issue 200)
8. `kramdown::split_text_after_html_block_close` -- splits text after `</figure>` etc. onto new lines (Issue 203)
9. `normalize_zwsp_for_emphasis` -- normalizes ZWSP before `_`/`*` for emphasis (Issue 198)
10. `fix_kramdown_emphasis_patterns` -- fixes `word*X*` patterns kramdown handles but CommonMark does not (Issue 206)
11. `protect_consecutive_single_quotes` / `restore_consecutive_single_quotes` -- protects `''` and `'''` from smart punctuation (Issue 198)
12. `protect_liquid_quotes` / `restore_liquid_quotes` -- protects quotes inside `{% %}`, `{{ }}`, and `{:}` from smart punctuation

### B. During-markdown transformations (applied to pulldown-cmark event stream)

13. `add_inline_code_class_to_events` / `add_inline_code_class_to_events_impl` -- adds `class="language-plaintext highlighter-rouge"` to backtick `Code` events; optionally converts `SoftBreak` to `<br>` for hardbreaks mode

### C. Post-markdown restore steps (applied to HTML output before kramdown postprocessing)

14. `restore_liquid_quotes` -- restores quote placeholders
15. `restore_consecutive_single_quotes` -- restores `''`/`'''` placeholders
16. `restore_math_content` -- restores math block placeholders
17. `decode_pulldown_url_encoding` -- decodes `%5D` back to `]` in href/src attributes (Issue 207/212)
18. `kramdown::fix_smart_quote_directions` -- fixes curly quote open/close directions to match kramdown rules (Issue 211)
19. `restore_preexisting_curly_quotes` -- restores pre-existing curly quote placeholders

### D. Kramdown postprocessing pipeline (`kramdown::postprocess`)

Applied in order:

20. `strip_paragraphs_in_html_blocks` -- strips `<p>` inside HTML block elements (e.g., `<li>`, `<div>`)
21. `encode_bare_ampersands` -- encodes `&` not part of entities to `&amp;` (D17), skips `<script>` blocks
22. `add_heading_ids` -- auto-generates `id` attributes on headings (slugified, deduplicated)
23. `apply_block_ial` -- applies kramdown block-level inline attribute lists (e.g., `{:.class}` on its own line)
24. `apply_inline_attributes` -- applies kramdown inline IALs (e.g., `{:target="_blank"}` after links)
25. `wrap_fenced_code_blocks` -- wraps fenced code blocks in `<div class="...">` wrappers (no-language variant)
26. `wrap_bare_text_in_paragraphs` -- wraps bare text between block elements in `<p>` tags
27. `add_block_spacing` -- adds extra `\n\n` between consecutive block elements
28. `remove_ol_start_attribute` -- strips `start` attribute from `<ol>` tags (D11)
29. `indent_list_items` -- indents loose list item content with 2 spaces
30. `indent_blockquote_content` -- indents blockquote inner content
31. `normalize_figcaption_whitespace` -- normalizes `</figcaption>` closing tag whitespace (D6)
32. `normalize_bare_void_elements` -- converts bare `<br>`, `<hr>` to XHTML `<br />`, `<hr />` (Issue 201)
33. `normalize_boolean_attributes` -- normalizes `required=""` to `required` (D2, D12)

### E. Lighter filter postprocessing (`kramdown::postprocess_for_filter`)

Used by `markdownify` filter. Applies a subset: `apply_inline_attributes`, `remove_ol_start_attribute`, `add_block_spacing`, `indent_list_items`, `normalize_bare_void_elements`, `normalize_boolean_attributes`.

### F. Pre-markdown steps in the layout/rendering pipeline (outside `frontmatter.rs`)

Called from `template/layout.rs` and `template/filters/markdownify.rs`:

34. `kramdown::mark_existing_html_headings` -- marks raw HTML headings with `data-raw-html` so `add_heading_ids` skips them (D1)
35. `kramdown::collapse_blank_lines_in_html_blocks` -- collapses blank lines inside HTML block elements to prevent spurious `<p>` wrapping
36. `kramdown::remove_heading_markers` -- removes `data-raw-html` markers after postprocessing
37. `frontmatter::dedent_html_lines` -- reduces 4+ space indentation on HTML lines to prevent code-block interpretation

### G. Final output normalization (`kramdown::normalize_html_output`)

Applied to the FINAL rendered HTML before writing to disk:

38. `normalize_br_only` -- converts bare `<br>` to `<br />` (NOT `<hr>`)
39. `normalize_boolean_attributes` -- normalizes boolean attributes (only if `=""` found)

### H. CommonMarkGhPages-specific

40. `frontmatter::normalize_br_to_html5` -- converts `<br />` back to `<br>` for non-kramdown sites

## Acceptance Criteria

- [ ] Document created at `docs/pulldown-kramdown-postprocessing.md`
- [ ] Every public and private transformation function listed above (items 1-40) is documented with all 6 fields (what/why/where/when/which-sites/risk)
- [ ] Each entry includes a concrete input->output example (can be brief, 1-2 lines)
- [ ] The document is organized by pipeline phase (pre-markdown, during-markdown, post-markdown, kramdown postprocess, filter postprocess, layout pipeline, final output)
- [ ] The document includes a pipeline diagram showing the order of all steps in `markdown_to_html()`, `markdown_to_html_with_options()`, `markdown_to_html_for_filter()`, `postprocess()`, `postprocess_for_filter()`, and `normalize_html_output()`
- [ ] Any transformation functions found in the source code that are NOT listed above are also documented (the SWE must grep for all relevant functions, not just use this list)
- [ ] The document references the originating issue number for each transformation where known (e.g., "Issue 204", "D11")
- [ ] The document is accurate against the current source code -- line numbers are approximate but function names must be exact
- [ ] `cargo build` still compiles without errors (no code changes expected, but verify)
- [ ] `cargo test` still passes (no code changes expected, but verify)

## Test Scenarios

This is a documentation-only issue. There are no new Rust tests to write. Verification is:

### Verification: Completeness
- Grep `src/kramdown.rs` and `src/frontmatter.rs` for all `pub fn` and `fn` definitions related to transformations
- Grep `src/template/layout.rs` and `src/template/filters/markdownify.rs` for all calls to `kramdown::` and `frontmatter::` transformation functions
- Verify every discovered function appears in the document

### Verification: Accuracy
- For each documented function, confirm the function name exists in the stated file
- Confirm the "when it runs" phase is correct by tracing the call chain
- Confirm the "which sites" field is correct by checking whether the function is called from `markdown_to_html` (kramdown), `markdown_to_html_with_options` (configurable), or `markdown_to_html_for_filter` (filter)

### Verification: Build
- `cargo build` compiles without errors
- `cargo test` passes (no regressions from documentation-only change)

## Dependencies

- None (documentation only)

## Notes

- The SWE should read `src/kramdown.rs`, `src/frontmatter.rs`, `src/template/layout.rs`, and `src/template/filters/markdownify.rs` thoroughly
- Function line numbers will drift as code changes; use approximate numbers and note they are approximate
- The "D1", "D2", etc. labels in code comments refer to diff categories from the original comparison work
- The protect/restore pairs should be documented together as a single logical transformation
