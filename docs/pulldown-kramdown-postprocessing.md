# Pulldown-cmark to Kramdown Postprocessing Reference

This document catalogs every preprocessing, during-processing, and postprocessing
transformation that rustkyll applies to bridge the gap between pulldown-cmark
(CommonMark) and Jekyll's kramdown Markdown engine.

Line numbers are approximate and will drift as the code evolves. Function names
are exact as of the time of writing.

---

## Pipeline Overview

### `markdown_to_html()` (kramdown mode, the default)

```
 Input: raw markdown (after Liquid rendering)
   |
   v
 1. process_markdown_attribute          [kramdown.rs]
 2. protect_preexisting_curly_quotes    [frontmatter.rs]
 3. escape_paren_list_markers           [frontmatter.rs]
 4. protect_math_content                [frontmatter.rs]
 5. escape_headings_in_list_context     [kramdown.rs]
 6. collapse_blank_lines_between_list_items [kramdown.rs]
 7. convert_kramdown_pipe_tables        [kramdown.rs]
 8. split_text_after_html_block_close   [kramdown.rs]
 9. normalize_zwsp_for_emphasis         [frontmatter.rs]
10. fix_kramdown_emphasis_patterns      [frontmatter.rs]
11. protect_consecutive_single_quotes   [frontmatter.rs]
12. protect_liquid_quotes               [frontmatter.rs]
   |
   v  pulldown-cmark parsing (with SMART_PUNCTUATION, TABLES, STRIKETHROUGH)
   |
13. add_inline_code_class_to_events     [frontmatter.rs] (event stream)
   |
   v  pulldown-cmark HTML rendering
   |
14. restore_liquid_quotes               [frontmatter.rs]
15. restore_consecutive_single_quotes   [frontmatter.rs]
16. restore_math_content                [frontmatter.rs]
17. decode_pulldown_url_encoding        [frontmatter.rs]
18. fix_smart_quote_directions          [kramdown.rs]
19. restore_preexisting_curly_quotes    [frontmatter.rs]
   |
   v  kramdown::postprocess()
   |
20. strip_paragraphs_in_html_blocks     [kramdown.rs]
21. encode_bare_ampersands              [kramdown.rs]
22. add_heading_ids                     [kramdown.rs]
23. apply_block_ial                     [kramdown.rs]
24. apply_inline_attributes             [kramdown.rs]
25. wrap_fenced_code_blocks             [kramdown.rs]
26. wrap_bare_text_in_paragraphs        [kramdown.rs]
27. add_block_spacing                   [kramdown.rs]
28. remove_ol_start_attribute           [kramdown.rs]
29. indent_list_items                   [kramdown.rs]
30. indent_blockquote_content           [kramdown.rs]
31. normalize_figcaption_whitespace     [kramdown.rs]
32. normalize_bare_void_elements        [kramdown.rs]
33. normalize_boolean_attributes        [kramdown.rs]
   |
   v
 Output: kramdown-compatible HTML
```

### `markdown_to_html_with_options()` (configurable mode)

Identical to `markdown_to_html()` except:
- `ENABLE_SMART_PUNCTUATION` is only enabled when `enable_smart_punctuation` is true
- Uses `add_inline_code_class_to_events_impl` with `add_code_classes` and `hardbreaks` parameters
- When `hardbreaks` is true, `SoftBreak` events are converted to `HardBreak`

### `markdown_to_html_for_filter()` (markdownify filter)

Same pre-markdown and during-markdown steps as `markdown_to_html()`. Same post-markdown restore steps. Calls `kramdown::postprocess_for_filter()` instead of `postprocess()`.

### `postprocess_for_filter()`

```
 Input: HTML from pulldown-cmark (after restore steps)
   |
24. apply_inline_attributes
28. remove_ol_start_attribute
27. add_block_spacing
29. indent_list_items
32. normalize_bare_void_elements
33. normalize_boolean_attributes
   |
   v
 Output: filter-compatible HTML
```

### Layout rendering pipeline (`render_markdown_page_with_cached_site` / `render_markdown_page_with_site_overrides`)

```
 Input: raw markdown with Liquid tags
   |
   v  Liquid template rendering
   |
37. dedent_html_lines                   [frontmatter.rs]
34. mark_existing_html_headings         [kramdown.rs]
35. collapse_blank_lines_in_html_blocks [kramdown.rs]
   |
   v  markdown_to_html_with_options()   (full pipeline above)
   |
36. remove_heading_markers              [kramdown.rs]
   |
   v  Layout wrapping (Liquid template)
   |
   v  normalize_html_output()
   |
38. normalize_br_only                   [kramdown.rs]
39. normalize_boolean_attributes        [kramdown.rs]  (only if `=""` found)
   |
   v  (if hardbreaks enabled)
40. normalize_br_to_html5               [frontmatter.rs]
   |
   v
 Output: final HTML written to disk
```

---

## A. Pre-markdown Transformations

Applied to the markdown source before pulldown-cmark parsing.

---

### 1. `kramdown::process_markdown_attribute`

- **What:** Finds HTML elements with `markdown="1"` attribute, renders their inner content as markdown via a secondary pulldown-cmark pass, strips the `markdown="1"` attribute, and replaces the element content with the rendered HTML. For inline containers (`<p>`, `<span>`), strips the outer `<p>` wrapper from the rendered markdown.
- **Example:**
  - Input: `<aside markdown="1">**bold** text</aside>`
  - Output: `<aside>\n<p><strong>bold</strong> text</p>\n</aside>`
- **Why:** kramdown supports `markdown="1"` on HTML elements, processing their content as markdown. pulldown-cmark treats HTML blocks as opaque. (Issue 228)
- **Where:** `kramdown.rs`, ~line 172
- **When:** Pre-markdown (first step in `markdown_to_html()` and `markdown_to_html_with_options()`)
- **Which sites:** Both kramdown and CommonMarkGhPages
- **Risk:** Medium -- nested `markdown="1"` elements or complex HTML structures may not be handled perfectly. The secondary parse does not apply the full pre-processing pipeline (no smart quote protection, etc.).

---

### 2. `protect_preexisting_curly_quotes` / `restore_preexisting_curly_quotes`

- **What:** Replaces pre-existing Unicode curly quotes (U+2018, U+2019, U+201C, U+201D) in the markdown source with null-byte-delimited placeholders before processing. After `fix_smart_quote_directions` runs, the placeholders are restored to the original curly quotes.
- **Example:**
  - Input: `He said \u{201C}hello\u{201D}`
  - Protected: `He said \x00CLD\x00hello\x00CRD\x00`
  - After restore: `He said \u{201C}hello\u{201D}` (unchanged)
- **Why:** kramdown only converts straight quotes to curly quotes; pre-existing curly quotes pass through unchanged. Without protection, `fix_smart_quote_directions` would re-process them and potentially change their direction.
- **Where:** `frontmatter.rs`, ~line 591 (protect), ~line 607 (restore)
- **When:** Pre-markdown (protect, step 2) / Post-markdown (restore, step 19)
- **Which sites:** Both (called in all three `markdown_to_html*` functions)
- **Risk:** Low -- placeholder uses null bytes which should not appear in normal markdown content.

---

### 3. `escape_paren_list_markers`

- **What:** Escapes `N)` ordered list markers at the start of lines by inserting a backslash before the closing parenthesis, converting `1) text` to `1\) text`. Only applies outside code blocks and HTML blocks.
- **Example:**
  - Input: `1) First item`
  - Output: `1\) First item`
- **Why:** kramdown only recognizes `.` as an ordered list delimiter; `1) text` is treated as a regular paragraph. pulldown-cmark (CommonMark) recognizes both `.` and `)`.
- **Where:** `frontmatter.rs`, ~line 831
- **When:** Pre-markdown (step 3)
- **Which sites:** Both
- **Risk:** Low -- only modifies lines matching the specific `N) ` pattern at line start.

---

### 4. `protect_math_content` / `restore_math_content`

- **What:** Replaces content inside `$...$` (inline math) and `$$...$$` (display math) delimiters with indexed placeholders before markdown processing. After HTML generation, placeholders are restored to the original math content.
- **Example:**
  - Input: `$\alpha + \beta$`
  - Protected: `$\x00MATH0MATH\x00$`
  - After restore: `$\alpha + \beta$`
- **Why:** pulldown-cmark treats `\,` as an escaped comma and strips the backslash. kramdown passes `\,` through literally inside math blocks. (Issue 227)
- **Where:** `frontmatter.rs`, ~line 621 (protect), ~line 707 (restore)
- **When:** Pre-markdown (protect, step 4) / Post-markdown (restore, step 16)
- **Which sites:** Both
- **Risk:** Low -- inline math does not cross line boundaries; display math (`$$`) can span lines.

---

### 5. `kramdown::escape_headings_in_list_context`

- **What:** Backslash-escapes `#` heading markers that appear immediately after a list item without a blank line separator. Escapes `# heading` to `\# heading` in that context.
- **Example:**
  - Input:
    ```
    - list item
    #### heading text
    ```
  - Output:
    ```
    - list item
    \#### heading text
    ```
- **Why:** In kramdown, a heading marker after a list item without a blank line is treated as text within the list item, not as a heading. pulldown-cmark (CommonMark) treats it as a heading, breaking the list. (Issue 204)
- **Where:** `kramdown.rs`, ~line 587
- **When:** Pre-markdown (step 5)
- **Which sites:** Both
- **Risk:** Low -- only affects `#` lines immediately following list items without blank line separation. Tracks code blocks to avoid false matches.

---

### 6. `kramdown::collapse_blank_lines_between_list_items`

- **What:** Collapses blank lines between consecutive list items in "partially loose" lists (some blank lines but not all between items). Fully loose lists (blank lines between ALL items) are left unchanged.
- **Example:**
  - Input (partially loose):
    ```
    - item 1

    - item 2
    - item 3
    ```
  - Output:
    ```
    - item 1
    - item 2
    - item 3
    ```
- **Why:** CommonMark makes an entire list loose if ANY blank line appears between items. kramdown only wraps items in `<p>` when ALL inter-item gaps have blank lines. (Issue 204)
- **Where:** `kramdown.rs`, ~line 653
- **When:** Pre-markdown (step 6)
- **Which sites:** Both
- **Risk:** Medium -- the heuristic for detecting fully loose vs. partially loose may not match kramdown exactly in all edge cases.

---

### 7. `kramdown::convert_kramdown_pipe_tables`

- **What:** Converts kramdown-style pipe table lines (lines ending with `|` that are NOT part of a standard GFM pipe table) into raw HTML `<table><tbody><tr><td>` elements. Standard pipe tables (with separator rows like `|---|---|`) are left for pulldown-cmark to handle.
- **Example:**
  - Input: `cell1 | cell2 |`
  - Output: `<table><tbody><tr><td>cell1</td><td>cell2</td></tr></tbody></table>`
- **Why:** kramdown treats any line ending with `|` as a table row, but pulldown-cmark only recognizes GFM-style tables with header/separator rows. (Issue 200)
- **Where:** `kramdown.rs`, ~line 745
- **When:** Pre-markdown (step 7)
- **Which sites:** Both
- **Risk:** Medium -- the heuristic to distinguish kramdown tables from standard GFM tables (checking for separator lines) may misclassify edge cases.

---

### 8. `kramdown::split_text_after_html_block_close`

- **What:** Inserts a blank line between a closing HTML block tag (e.g., `</figure>`, `</div>`) and text that immediately follows it on the same line. This causes pulldown-cmark to parse the trailing text as a new markdown paragraph.
- **Example:**
  - Input: `</figure>Photo by [Name](url)`
  - Output:
    ```
    </figure>

    Photo by [Name](url)
    ```
- **Why:** In kramdown, text after a closing block tag is treated as new content. In CommonMark, the entire line is part of the HTML block, so markdown links in the trailing text are not parsed. (Issue 203)
- **Where:** `kramdown.rs`, ~line 407
- **When:** Pre-markdown (step 8)
- **Which sites:** Both
- **Risk:** Low -- only triggers when non-whitespace text immediately follows a block close tag on the same line, and only for known block-level closing tags.

---

### 9. `normalize_zwsp_for_emphasis`

- **What:** Inserts a regular space after zero-width space (U+200B) when followed by `_` or `*` emphasis markers.
- **Example:**
  - Input: `\u{200b}_emphasized_`
  - Output: `\u{200b} _emphasized_`
- **Why:** CommonMark does not classify ZWSP as whitespace (it is Unicode category Cf, "format"), so `\u{200b}_word_` is treated as mid-word and emphasis is not applied. kramdown recognizes ZWSP as a word boundary. (Issue 198)
- **Where:** `frontmatter.rs`, ~line 522
- **When:** Pre-markdown (step 9)
- **Which sites:** Both
- **Risk:** Low -- only applies when ZWSP is immediately followed by emphasis markers.

---

### 10. `fix_kramdown_emphasis_patterns`

- **What:** Inserts ZWSP+space before `*X*` patterns that are immediately preceded by an alphanumeric character (e.g., `word*X*`), enabling CommonMark to recognize the emphasis delimiter.
- **Example:**
  - Input: `word*.doc*`
  - Output: `word\u{200b} *.doc*`
- **Why:** In CommonMark, `word*X*` is not a left-flanking delimiter run (preceded by alphanumeric). kramdown is more permissive and recognizes emphasis in this context. (Issue 206)
- **Where:** `frontmatter.rs`, ~line 490
- **When:** Pre-markdown (step 10)
- **Which sites:** Both
- **Risk:** Low -- only fires on specific `alphanumeric*non-space*` patterns with short content (up to 5 chars between `*`).

---

### 11. `protect_consecutive_single_quotes` / `restore_consecutive_single_quotes`

- **What:** Replaces `'''` and `''` with null-byte-delimited placeholders before markdown processing. After HTML generation, placeholders are restored to original quote sequences.
- **Example:**
  - Input: `''bold text''`
  - Protected: `\x00SQ2\x00bold text\x00SQ2\x00`
  - After restore: `''bold text''`
- **Why:** kramdown does NOT convert `''text''` or `'''text'''` to curly quotes -- it keeps them as literal straight single quotes (used in MediaWiki-style markup). pulldown-cmark's smart punctuation converts them to curly quotes. (Issue 198)
- **Where:** `frontmatter.rs`, ~line 563 (protect), ~line 573 (restore)
- **When:** Pre-markdown (protect, step 11) / Post-markdown (restore, step 15)
- **Which sites:** Both
- **Risk:** Low -- uses null-byte placeholders; `'''` is replaced before `''` to avoid partial matching.

---

### 12. `protect_liquid_quotes` / `restore_liquid_quotes`

- **What:** Replaces double-quote characters inside Liquid tags (`{% %}`, `{{ }}`), and kramdown inline attribute lists (`{: }`) with null-byte-delimited placeholders. After HTML generation, placeholders are restored.
- **Example:**
  - Input: `{:target="_blank"}`
  - Protected: `{:target=\x00QUOT\x00_blank\x00QUOT\x00}`
  - After restore: `{:target="_blank"}`
- **Why:** pulldown-cmark's smart punctuation converts straight double quotes to curly quotes, which would break Liquid tag syntax and kramdown IAL attribute parsing.
- **Where:** `frontmatter.rs`, ~line 899 (protect), ~line 948 (restore)
- **When:** Pre-markdown (protect, step 12) / Post-markdown (restore, step 14)
- **Which sites:** Both
- **Risk:** Low -- only targets content between `{%`/`%}`, `{{`/`}}`, and `{:`/`}` delimiters.

---

## B. During-markdown Transformations

Applied to the pulldown-cmark event stream during parsing.

---

### 13. `add_inline_code_class_to_events` / `add_inline_code_class_to_events_impl`

- **What:** Transforms pulldown-cmark `Code` events (from backtick `` `code` ``) into raw HTML events with `class="language-plaintext highlighter-rouge"`. Also restores trailing whitespace before `SoftBreak` events (which pulldown-cmark strips but kramdown preserves). When `hardbreaks` is true, converts `SoftBreak` to `HardBreak` (producing `<br />`).
- **Example:**
  - Input event: `Code("example")`
  - Output event: `InlineHtml("<code class=\"language-plaintext highlighter-rouge\">example</code>")`
- **Why:** Jekyll/kramdown adds `class="language-plaintext highlighter-rouge"` to all backtick-generated inline code. pulldown-cmark outputs bare `<code>` tags. Raw HTML `<code>` tags in the source are left untouched (they come through as `Html` events, not `Code` events). (Issue 223 for hardbreaks)
- **Where:** `frontmatter.rs`, ~line 170 (wrapper), ~line 185 (impl)
- **When:** During markdown (event stream transformation, step 13)
- **Which sites:** `add_code_classes` is true for kramdown, false for CommonMarkGhPages. `hardbreaks` is true only for CommonMarkGhPages with HARDBREAKS option.
- **Risk:** Low -- operates on strongly-typed events, not string manipulation.

---

## C. Post-markdown Restore Steps

Applied to HTML output after pulldown-cmark rendering, before kramdown postprocessing.

---

### 14. `restore_liquid_quotes`

See item 12 above. Restores `\x00QUOT\x00` placeholders back to `"`.

- **Where:** `frontmatter.rs`, ~line 948
- **When:** Post-markdown (step 14, first restore)

---

### 15. `restore_consecutive_single_quotes`

See item 11 above. Restores `\x00SQ2\x00` and `\x00SQ3\x00` placeholders back to `''` and `'''`.

- **Where:** `frontmatter.rs`, ~line 573
- **When:** Post-markdown (step 15)

---

### 16. `restore_math_content`

See item 4 above. Restores `\x00MATHNMATH\x00` placeholders back to original math content.

- **Where:** `frontmatter.rs`, ~line 707
- **When:** Post-markdown (step 16)

---

### 17. `decode_pulldown_url_encoding`

- **What:** Decodes `%5D` back to `]` in `href="..."` and `src="..."` attribute values. Only decodes `]` (0x5D); other percent-encoded characters (non-ASCII, spaces, etc.) are left encoded.
- **Example:**
  - Input: `<a href="url%5D">`
  - Output: `<a href="url]">`
- **Why:** pulldown-cmark percent-encodes `]` in URLs, but Jekyll/kramdown preserves them as-is. (Issue 207, Issue 212)
- **Where:** `frontmatter.rs`, ~line 737
- **When:** Post-markdown (step 17)
- **Which sites:** Both
- **Risk:** Low -- only modifies content inside `href="..."` and `src="..."` attributes, and only decodes `]`.

---

### 18. `kramdown::fix_smart_quote_directions`

- **What:** Re-determines the direction (opening/closing) of every smart quote character (U+2018/U+2019/U+201C/U+201D) in the HTML output using kramdown's SmartyPants-based heuristics instead of pulldown-cmark's Unicode-standard logic. Also handles German-style double quotes (U+201E opener).
- **Example:**
  - Input: `\u{201C}word \u{2018}s\u{2019} end\u{201D}` (pulldown-cmark directions)
  - Output: May change directions based on preceding/following characters using kramdown rules (Issue 211)
- **Why:** pulldown-cmark uses Unicode left-flanking/right-flanking delimiter logic; kramdown uses simpler character-class rules from SmartyPants/RubyPants. They disagree in several cases (e.g., quote after punctuation, apostrophes in certain contexts). (Issue 211)
- **Where:** `kramdown.rs`, ~line 3308
- **When:** Post-markdown (step 18)
- **Which sites:** Both (but only has effect when smart punctuation is enabled)
- **Risk:** Medium -- the kramdown SQ_RULES are complex with many edge cases. Apostrophe detection uses a heuristic (alphabetic chars on both sides).

---

### 19. `restore_preexisting_curly_quotes`

See item 2 above. Restores curly quote placeholders back to original Unicode characters.

- **Where:** `frontmatter.rs`, ~line 607
- **When:** Post-markdown (step 19, after `fix_smart_quote_directions`)

---

## D. Kramdown Postprocessing Pipeline (`kramdown::postprocess`)

Applied in order to the HTML after all restore steps.

---

### 20. `strip_paragraphs_in_html_blocks`

- **What:** Removes auto-generated `<p>` wrappers that pulldown-cmark inserts inside HTML block elements (`<li>`, `<td>`, `<th>`, `<h1>`-`<h6>`, `<figure>`, `<summary>`, `<dd>`, `<dt>`). Preserves `<p>` tags in bare `<li>` elements (from markdown list syntax), in `<figcaption>` elements, and where `<p>` content contains block-level children.
- **Example:**
  - Input: `<li class="item"><p>text</p></li>`
  - Output: `<li class="item">text</li>`
  - Preserved: `<li><p>text</p></li>` (bare `<li>` from markdown, loose list)
- **Why:** pulldown-cmark wraps inline content in `<p>` tags inside many HTML block elements. kramdown does not do this for HTML that comes from includes/raw HTML. (D1-related)
- **Where:** `kramdown.rs`, ~line 1121
- **When:** Kramdown postprocess (step 20, first in pipeline)
- **Which sites:** Kramdown only (via `postprocess()`)
- **Risk:** Medium -- the heuristic for distinguishing auto-generated vs. intentional `<p>` tags (bare `<li>` detection, `<figcaption>` nesting) may not cover all edge cases.

---

### 21. `encode_bare_ampersands`

- **What:** Encodes `&` characters that are not part of valid HTML entity references (`&name;`, `&#digits;`, `&#xhex;`) as `&amp;`. Skips content inside `<script>` blocks entirely.
- **Example:**
  - Input: `Tom & Jerry`
  - Output: `Tom &amp; Jerry`
  - Preserved: `&amp;` (already encoded), `&#8217;` (numeric entity)
- **Why:** pulldown-cmark passes raw HTML blocks through verbatim, so bare `&` characters survive into the output. Jekyll/kramdown re-encodes these as `&amp;`. (D17)
- **Where:** `kramdown.rs`, ~line 3177
- **When:** Kramdown postprocess (step 21)
- **Which sites:** Kramdown only (via `postprocess()`)
- **Risk:** Low -- uses precise entity-start detection. `<script>` block content is correctly skipped (important for JSON-LD structured data with `&` in strings).

---

### 22. `add_heading_ids`

- **What:** Adds auto-generated `id` attributes to heading tags (`<h1>`-`<h6>`) using the kramdown-parser-gfm slugification algorithm: lowercase, strip non-word characters (keeping Unicode letters, digits, underscores, hyphens), replace spaces/tabs with hyphens. Handles duplicate IDs by appending `-1`, `-2`, etc. Supports explicit `{#custom-id}` syntax. Skips headings with existing attributes (e.g., from `mark_existing_html_headings`).
- **Example:**
  - Input: `<h2>Hello World!</h2>`
  - Output: `<h2 id="hello-world">Hello World!</h2>`
  - Explicit: `<h2>Title {#my-id}</h2>` -> `<h2 id="my-id">Title</h2>`
- **Why:** kramdown auto-generates heading IDs for anchor linking. pulldown-cmark does not generate heading IDs.
- **Where:** `kramdown.rs`, ~line 1914
- **When:** Kramdown postprocess (step 22)
- **Which sites:** Kramdown only (via `postprocess()`)
- **Risk:** Low -- uses the GFM algorithm matching `kramdown-parser-gfm`. Unicode letters are preserved (Cyrillic, CJK, etc.).

---

### 23. `apply_block_ial`

- **What:** Finds block-level kramdown Inline Attribute Lists (IALs) -- paragraphs like `<p>{: .class }</p>` -- and applies their attributes (classes, IDs, arbitrary key-value pairs) to the preceding block element's opening tag, then removes the IAL paragraph. Also handles IALs merged into paragraph text by pulldown-cmark (e.g., `<p>text {: .class }</p>`).
- **Example:**
  - Input:
    ```html
    <h2 id="title">Title</h2>
    <p>{: .fs-9 }</p>
    ```
  - Output: `<h2 id="title" class="fs-9">Title</h2>`
- **Why:** kramdown supports `{: .class #id key="value"}` attribute lists on their own line to modify the preceding element. pulldown-cmark treats these as regular paragraphs.
- **Where:** `kramdown.rs`, ~line 1428
- **When:** Kramdown postprocess (step 23)
- **Which sites:** Kramdown only (via `postprocess()`)
- **Risk:** Medium -- multi-pass approach (standalone IAL paragraphs first, then merged IALs). Relies on finding the "preceding block element" by searching backwards for closing tags.

---

### 24. `apply_inline_attributes`

- **What:** Finds inline kramdown IALs that follow a closing HTML tag (e.g., `</a>{:target="_blank"}`) and moves the attributes onto the corresponding opening tag. Supports `.class`, `#id`, and `key="value"` syntax. Skips IALs inside `<pre>` blocks. Does not apply `target` attributes to non-`<a>` elements.
- **Example:**
  - Input: `<a href="url">text</a>{:target="_blank"}`
  - Output: `<a href="url" target="_blank">text</a>`
- **Why:** kramdown supports `{:key="value"}` immediately after inline elements (most commonly links). pulldown-cmark does not recognize this syntax.
- **Where:** `kramdown.rs`, ~line 1644
- **When:** Kramdown postprocess (step 24); also in `postprocess_for_filter()`
- **Which sites:** Both (kramdown via `postprocess()`, filter via `postprocess_for_filter()`)
- **Risk:** Medium -- walks backwards through the output buffer to find the matching opening tag. The `target`-only-on-`<a>` guard prevents misattribution when links fail to parse.

---

### 25. `wrap_fenced_code_blocks`

- **What:** Wraps `<pre><code>...</code></pre>` blocks in kramdown-style div structure with syntax highlighting. For no-language blocks: `<div class="highlighter-rouge language-plaintext"><div class="highlight"><pre class="highlight"><code>...</code></pre></div></div>`. For language-tagged blocks: uses `language-{lang} highlighter-rouge`. Attempts syntax highlighting via `crate::syntax::highlight_code`; falls back to plain code if unsupported.
- **Example:**
  - Input: `<pre><code>hello</code></pre>`
  - Output: `<div class="highlighter-rouge language-plaintext"><div class="highlight"><pre class="highlight"><code>hello</code></pre></div></div>`
- **Why:** Jekyll/kramdown with Rouge wraps fenced code blocks in a specific div structure for syntax highlighting CSS.
- **Where:** `kramdown.rs`, ~line 2185
- **When:** Kramdown postprocess (step 25)
- **Which sites:** Kramdown only (via `postprocess()`)
- **Risk:** Low -- pattern matching on `<pre><code` is robust. HTML-unescapes code content before highlighting, then the highlighter re-escapes as needed.

---

### 26. `wrap_bare_text_in_paragraphs`

- **What:** Wraps bare inline text that sits between block-level elements at the top level (depth 0) in `<p>...</p>` tags. Tracks container element nesting depth to avoid wrapping text inside block elements.
- **Example:**
  - Input:
    ```html
    </h3>
    Some loose text
    <ul>
    ```
  - Output:
    ```html
    </h3>
    <p>Some loose text</p>
    <ul>
    ```
- **Why:** kramdown auto-wraps loose inline text between block elements in `<p>` tags. pulldown-cmark does not do this for text originating from raw HTML / Liquid template output.
- **Where:** `kramdown.rs`, ~line 2284
- **When:** Kramdown postprocess (step 26)
- **Which sites:** Kramdown only (via `postprocess()`)
- **Risk:** Medium -- uses line-by-line analysis with depth tracking. The heuristic for detecting "bare text context" (block element before and after) may miss edge cases with mixed inline/block content on the same line.

---

### 27. `add_block_spacing`

- **What:** Adds an extra newline after closing block-level tags (`</p>`, `</h1>`-`</h6>`, `</ul>`, `</ol>`, `</blockquote>`, `</div>`, `</pre>`, `</table>`, `</figure>`) to produce `\n\n` separation between consecutive block elements. Skips `</pre></div></div>` sequences (code block wrappers). Skips content inside `<script>` blocks entirely. Does not add double-newline at the very end of content.
- **Example:**
  - Input: `<p>first</p>\n<p>second</p>\n`
  - Output: `<p>first</p>\n\n<p>second</p>\n`
- **Why:** kramdown outputs double newlines between block elements. pulldown-cmark outputs single newlines. (Issue 185 for script block handling)
- **Where:** `kramdown.rs`, ~line 2549
- **When:** Kramdown postprocess (step 27); also in `postprocess_for_filter()`
- **Which sites:** Both (kramdown via `postprocess()`, filter via `postprocess_for_filter()`)
- **Risk:** Low -- the `<script>` block skip (Issue 185) prevents modification of JSON-LD content containing `</p>` patterns in strings.

---

### 28. `remove_ol_start_attribute`

- **What:** Strips `start="N"` attributes from `<ol>` tags, converting `<ol start="2">` to `<ol>`.
- **Example:**
  - Input: `<ol start="3">`
  - Output: `<ol>`
- **Why:** pulldown-cmark adds `start="N"` to ordered lists that don't start at 1. kramdown never adds this attribute. (D11)
- **Where:** `kramdown.rs`, ~line 2661
- **When:** Kramdown postprocess (step 28); also in `postprocess_for_filter()`
- **Which sites:** Both
- **Risk:** Low -- simple attribute removal on `<ol ` tags.

---

### 29. `indent_list_items`

- **What:** Indents `<li>` items inside `<ul>` and `<ol>` lists. For loose lists (containing `<li>\n<p>`), indents `<li>`/`</li>` by 2 spaces and inner content by 4 spaces, and removes blank lines inside `<li>`. For tight lists, indents `<li>`/`</li>` by 2 spaces.
- **Example:**
  - Input (tight):
    ```html
    <ul>
    <li>item</li>
    </ul>
    ```
  - Output:
    ```html
    <ul>
      <li>item</li>
    </ul>
    ```
- **Why:** kramdown indents list item content with spaces to produce more readable HTML output. pulldown-cmark outputs flat unindented HTML.
- **Where:** `kramdown.rs`, ~line 3000
- **When:** Kramdown postprocess (step 29); also in `postprocess_for_filter()`
- **Which sites:** Both
- **Risk:** Low -- whitespace-only changes. Detects loose lists by the presence of `<li>\n<p>`.

---

### 30. `indent_blockquote_content`

- **What:** Ensures blockquote content has a blank line before the closing `</blockquote>` tag, matching kramdown's output format. Does not indent inner content (kramdown uses no indentation).
- **Example:**
  - Input: `<blockquote>\n<p>text</p>\n</blockquote>`
  - Output: `<blockquote>\n<p>text</p>\n\n</blockquote>`
- **Why:** kramdown outputs a blank line between the last content and the closing `</blockquote>` tag. (Issue 163, fixed in Issue 164)
- **Where:** `kramdown.rs`, ~line 3115
- **When:** Kramdown postprocess (step 30)
- **Which sites:** Kramdown only (via `postprocess()`)
- **Risk:** Low -- only adds a trailing blank line before `</blockquote>`.

---

### 31. `normalize_figcaption_whitespace`

- **What:** Removes the newline before `</figcaption>` closing tags, putting the closing tag on the same line as the content.
- **Example:**
  - Input: `<figcaption>text\n</figcaption>`
  - Output: `<figcaption>text</figcaption>`
- **Why:** pulldown-cmark puts `</figcaption>` on a new line. kramdown puts it on the same line as the content. (D6)
- **Where:** `kramdown.rs`, ~line 3164
- **When:** Kramdown postprocess (step 31)
- **Which sites:** Kramdown only (via `postprocess()`)
- **Risk:** Low -- simple string replacement.

---

### 32. `normalize_bare_void_elements`

- **What:** Converts bare void element tags (`<br>`, `<hr>`, `<img>`, `<input>`, `<meta>`, `<link>`, `<col>`, `<area>`, `<base>`, `<embed>`, `<param>`, `<source>`, `<track>`, `<wbr>`) to XHTML-style self-closing tags (e.g., `<br />`, `<hr />`). Does not modify tags that are already self-closing.
- **Example:**
  - Input: `<br>`
  - Output: `<br />`
- **Why:** Jekyll/kramdown outputs XHTML-style self-closing tags for void elements. Raw HTML in markdown source may contain bare void tags. (Issue 201)
- **Where:** `kramdown.rs`, ~line 2801
- **When:** Kramdown postprocess (step 32); also in `postprocess_for_filter()`
- **Which sites:** Both
- **Risk:** Low -- byte-level tag scanning handles UTF-8 correctly by advancing by full character widths for non-tag content.

---

### 33. `normalize_boolean_attributes`

- **What:** Removes empty string values from boolean HTML attributes, converting `required=""` to `required`, `novalidate=""` to `novalidate`, etc. Recognizes 18 standard boolean attributes.
- **Example:**
  - Input: `<input type="text" required="">`
  - Output: `<input type="text" required>`
- **Why:** pulldown-cmark produces `attribute=""` for boolean attributes. kramdown produces bare `attribute`. (D2, D12)
- **Where:** `kramdown.rs`, ~line 2912
- **When:** Kramdown postprocess (step 33); also in `postprocess_for_filter()` and `normalize_html_output()`
- **Which sites:** Both
- **Risk:** Low -- only modifies `=""` following known boolean attribute names. Quick-exits when no `=""` is found.

---

## E. Filter Postprocessing (`kramdown::postprocess_for_filter`)

Used by the `markdownify` Liquid filter (via `markdown_to_html_for_filter`). Applies a subset of the full kramdown postprocessing pipeline:

| Step | Function |
|------|----------|
| 24 | `apply_inline_attributes` |
| 28 | `remove_ol_start_attribute` |
| 27 | `add_block_spacing` |
| 29 | `indent_list_items` |
| 32 | `normalize_bare_void_elements` |
| 33 | `normalize_boolean_attributes` |

Skips: `strip_paragraphs_in_html_blocks`, `encode_bare_ampersands`, `add_heading_ids`, `apply_block_ial`, `wrap_fenced_code_blocks`, `wrap_bare_text_in_paragraphs`, `indent_blockquote_content`, `normalize_figcaption_whitespace`.

- **Where:** `kramdown.rs`, ~line 117
- **When:** Called from `frontmatter::markdown_to_html_for_filter()`
- **Which sites:** Kramdown (filter context)

---

## F. Pre-markdown Steps in the Layout/Rendering Pipeline

Called from `template/layout.rs` (in `render_markdown_page_with_cached_site` and `render_markdown_page_with_site_overrides`) and `template/filters/markdownify.rs`, before the markdown-to-HTML conversion.

---

### 34. `kramdown::mark_existing_html_headings`

- **What:** Adds a `data-raw-html` attribute to bare `<hN>` heading tags (e.g., `<h1>` becomes `<h1 data-raw-html>`). This causes `add_heading_ids` to see the tag as non-simple (it has attributes) and skip it.
- **Example:**
  - Input: `<h3>Include Heading</h3>`
  - Output: `<h3 data-raw-html>Include Heading</h3>`
- **Why:** Headings from `{% include %}` output should NOT get auto-generated `id` attributes. Only headings generated from markdown content by pulldown-cmark should get IDs. (D1)
- **Where:** `kramdown.rs`, ~line 18
- **When:** Layout pipeline (after Liquid rendering, before markdown conversion)
- **Which sites:** Both (called from layout.rs for all markdown pages)
- **Risk:** Low -- only modifies bare `<hN>` tags (no existing attributes). Cleaned up by `remove_heading_markers` after postprocessing.

---

### 35. `kramdown::collapse_blank_lines_in_html_blocks`

- **What:** Removes blank lines (lines containing only whitespace) inside HTML block elements (`<li>`, `<div>`, `<p>`, `<td>`, `<th>`, `<h1>`-`<h6>`, `<section>`, `<article>`, `<header>`, `<footer>`, `<nav>`, `<aside>`, `<figure>`, `<figcaption>`, `<details>`, `<summary>`, `<form>`, `<fieldset>`, `<dd>`, `<dt>`). Content outside HTML block elements is left unchanged.
- **Example:**
  - Input:
    ```html
    <li>

    text

    </li>
    ```
  - Output:
    ```html
    <li>
    text
    </li>
    ```
- **Why:** Liquid `{% include %}` output often contains blank lines (from `{% assign %}`, `{% for %}` loops, etc.) inside HTML block elements. pulldown-cmark interprets blank lines as paragraph separators and wraps content in `<p>` tags. kramdown does not do this.
- **Where:** `kramdown.rs`, ~line 360
- **When:** Layout pipeline (after `mark_existing_html_headings`, before markdown conversion)
- **Which sites:** Both
- **Risk:** Low -- only removes blank lines inside matched HTML block element pairs. Handles nested tags correctly.

---

### 36. `kramdown::remove_heading_markers`

- **What:** Removes all `data-raw-html` marker attributes from headings by simple string replacement.
- **Example:**
  - Input: `<h3 data-raw-html>Heading</h3>`
  - Output: `<h3>Heading</h3>`
- **Why:** Cleanup step after `add_heading_ids` has run, removing the temporary markers added by `mark_existing_html_headings`.
- **Where:** `kramdown.rs`, ~line 57
- **When:** Layout pipeline (after `markdown_to_html_with_options`, before layout wrapping)
- **Which sites:** Both
- **Risk:** Low -- simple string replacement.

---

### 37. `frontmatter::dedent_html_lines`

- **What:** Reduces 4+ spaces of leading indentation on HTML-like lines to at most 3 spaces. A line is considered "HTML-like" if it starts with `<`, ends with `>`, or is otherwise detected by `looks_like_html()`.
- **Example:**
  - Input: `    <a href="url">link</a>` (4 spaces)
  - Output: `   <a href="url">link</a>` (3 spaces)
- **Why:** In CommonMark, 4+ spaces of indentation creates an indented code block. Liquid includes (like related-posts.html) often produce HTML with 4+ spaces of indentation from `{% for %}` loops, which pulldown-cmark interprets as code blocks. kramdown is more lenient about indentation inside HTML.
- **Where:** `frontmatter.rs`, ~line 968
- **When:** Layout pipeline (after Liquid rendering, first pre-markdown step)
- **Which sites:** Both
- **Risk:** Low -- only modifies lines with 4+ leading spaces that look like HTML. Plain text lines (potential intentional code blocks) are preserved.

---

## G. Final Output Normalization (`kramdown::normalize_html_output`)

Applied to the FINAL rendered HTML (after layout wrapping) before writing to disk.

---

### 38. `normalize_br_only`

- **What:** Converts bare `<br>` tags to `<br />` (XHTML-style). Does NOT convert `<hr>`, `<meta>`, `<link>`, `<input>`, `<img>`, or other void elements.
- **Example:**
  - Input: `text<br>more`
  - Output: `text<br />more`
- **Why:** Raw HTML `<br>` tags in markdown content (e.g., table cells) need XHTML-style self-closing to match Jekyll/kramdown output. Other void elements are not converted at this stage because: (1) pulldown-cmark already outputs `<hr />` for markdown rules, (2) `postprocess()` already handles void elements in markdown-rendered content, (3) converting `<hr>` here would incorrectly affect include/layout HTML.
- **Where:** `kramdown.rs`, ~line 2893
- **When:** Final output normalization (step 38)
- **Which sites:** Both
- **Risk:** Low -- simple global string replacement.

---

### 39. `normalize_boolean_attributes` (final pass)

- **What:** Same as step 33, but applied to the full page HTML after layout wrapping. Only runs if `=""` is found in the output (quick-exit optimization).
- **Where:** `kramdown.rs`, ~line 2912 (same function)
- **When:** Final output normalization (step 39, only if `=""` found)
- **Which sites:** Both
- **Risk:** Low -- identical to step 33 but runs on the full page. The early application in `postprocess()` (step 33) means this usually finds nothing to change, avoiding a full scan of the 100-300KB page HTML.

---

## H. CommonMarkGhPages-specific

---

### 40. `frontmatter::normalize_br_to_html5`

- **What:** Converts `<br />` to `<br>` for non-kramdown sites.
- **Example:**
  - Input: `text<br />more`
  - Output: `text<br>more`
- **Why:** Jekyll's CommonMarkGhPages renderer outputs `<br>` (HTML5 style), not `<br />` (XHTML style). This is called at the very end of the rendering pipeline to match Jekyll's output format.
- **Where:** `frontmatter.rs`, ~line 418
- **When:** Final step in layout rendering, only when `enable_hardbreaks` is true
- **Which sites:** CommonMarkGhPages only
- **Risk:** Low -- simple global string replacement. Only runs when the site is configured for CommonMarkGhPages with HARDBREAKS.

---

## Summary Table

| # | Function | File | Phase | Sites | Risk | Issue |
|---|----------|------|-------|-------|------|-------|
| 1 | `process_markdown_attribute` | kramdown.rs | Pre-markdown | Both | Medium | 228 |
| 2 | `protect/restore_preexisting_curly_quotes` | frontmatter.rs | Pre/Post-markdown | Both | Low | -- |
| 3 | `escape_paren_list_markers` | frontmatter.rs | Pre-markdown | Both | Low | -- |
| 4 | `protect/restore_math_content` | frontmatter.rs | Pre/Post-markdown | Both | Low | 227 |
| 5 | `escape_headings_in_list_context` | kramdown.rs | Pre-markdown | Both | Low | 204 |
| 6 | `collapse_blank_lines_between_list_items` | kramdown.rs | Pre-markdown | Both | Medium | 204 |
| 7 | `convert_kramdown_pipe_tables` | kramdown.rs | Pre-markdown | Both | Medium | 200 |
| 8 | `split_text_after_html_block_close` | kramdown.rs | Pre-markdown | Both | Low | 203 |
| 9 | `normalize_zwsp_for_emphasis` | frontmatter.rs | Pre-markdown | Both | Low | 198 |
| 10 | `fix_kramdown_emphasis_patterns` | frontmatter.rs | Pre-markdown | Both | Low | 206 |
| 11 | `protect/restore_consecutive_single_quotes` | frontmatter.rs | Pre/Post-markdown | Both | Low | 198 |
| 12 | `protect/restore_liquid_quotes` | frontmatter.rs | Pre/Post-markdown | Both | Low | -- |
| 13 | `add_inline_code_class_to_events[_impl]` | frontmatter.rs | During-markdown | Configurable | Low | 223 |
| 14 | `restore_liquid_quotes` | frontmatter.rs | Post-markdown | Both | Low | -- |
| 15 | `restore_consecutive_single_quotes` | frontmatter.rs | Post-markdown | Both | Low | 198 |
| 16 | `restore_math_content` | frontmatter.rs | Post-markdown | Both | Low | 227 |
| 17 | `decode_pulldown_url_encoding` | frontmatter.rs | Post-markdown | Both | Low | 207/212 |
| 18 | `fix_smart_quote_directions` | kramdown.rs | Post-markdown | Both | Medium | 211 |
| 19 | `restore_preexisting_curly_quotes` | frontmatter.rs | Post-markdown | Both | Low | -- |
| 20 | `strip_paragraphs_in_html_blocks` | kramdown.rs | Postprocess | Kramdown | Medium | -- |
| 21 | `encode_bare_ampersands` | kramdown.rs | Postprocess | Kramdown | Low | D17 |
| 22 | `add_heading_ids` | kramdown.rs | Postprocess | Kramdown | Low | -- |
| 23 | `apply_block_ial` | kramdown.rs | Postprocess | Kramdown | Medium | -- |
| 24 | `apply_inline_attributes` | kramdown.rs | Postprocess + Filter | Both | Medium | -- |
| 25 | `wrap_fenced_code_blocks` | kramdown.rs | Postprocess | Kramdown | Low | -- |
| 26 | `wrap_bare_text_in_paragraphs` | kramdown.rs | Postprocess | Kramdown | Medium | -- |
| 27 | `add_block_spacing` | kramdown.rs | Postprocess + Filter | Both | Low | 185 |
| 28 | `remove_ol_start_attribute` | kramdown.rs | Postprocess + Filter | Both | Low | D11 |
| 29 | `indent_list_items` | kramdown.rs | Postprocess + Filter | Both | Low | -- |
| 30 | `indent_blockquote_content` | kramdown.rs | Postprocess | Kramdown | Low | 163/164 |
| 31 | `normalize_figcaption_whitespace` | kramdown.rs | Postprocess | Kramdown | Low | D6 |
| 32 | `normalize_bare_void_elements` | kramdown.rs | Postprocess + Filter | Both | Low | 201 |
| 33 | `normalize_boolean_attributes` | kramdown.rs | Postprocess + Filter + Final | Both | Low | D2/D12 |
| 34 | `mark_existing_html_headings` | kramdown.rs | Layout pipeline | Both | Low | D1 |
| 35 | `collapse_blank_lines_in_html_blocks` | kramdown.rs | Layout pipeline | Both | Low | -- |
| 36 | `remove_heading_markers` | kramdown.rs | Layout pipeline | Both | Low | D1 |
| 37 | `dedent_html_lines` | frontmatter.rs | Layout pipeline | Both | Low | -- |
| 38 | `normalize_br_only` | kramdown.rs | Final output | Both | Low | -- |
| 39 | `normalize_boolean_attributes` (final) | kramdown.rs | Final output | Both | Low | D2/D12 |
| 40 | `normalize_br_to_html5` | frontmatter.rs | Final output | CommonMarkGhPages | Low | -- |
