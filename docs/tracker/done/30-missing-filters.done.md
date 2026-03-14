# Issue 30: Missing Liquid Filters

## Problem

Several Jekyll/Liquid filters are not implemented in rustkyll. The compatibility research (Issue 22) identified `number_of_words`, `group_by`, `xml_escape`, `sort_natural`, `concat`, `compact`, and `truncatewords` as missing.

After investigation:
- `compact`, `concat`, and `sort_natural` are already provided by liquid's stdlib (included via `ParserBuilder::with_stdlib()`). No custom implementation needed.
- `number_of_words`, `group_by`, `xml_escape`, and `truncatewords` are NOT provided by liquid or liquid-lib and require custom implementation.

Note: None of these four filters are currently used in the DataTalks.Club reference site's layouts or includes. However, they are used by external Jekyll sites (minimal-mistakes, beautiful-jekyll) and should be implemented for generic Jekyll compatibility.

## Requirements

- Implement `number_of_words` filter -- counts words in a string (splits on whitespace)
- Implement `group_by` filter -- groups an array of objects by a property, returns array of `{name, items, size}` objects
- Implement `xml_escape` filter -- escapes `&`, `<`, `>`, `"`, `'` for use in XML/HTML attributes and XML feeds
- Implement `truncatewords` filter -- truncates a string to N words, appending `...` by default (or a custom suffix)
- Register all new filters in `TemplateEngine::builder()` in `src/template/engine.rs`
- Follow the existing filter implementation pattern (see `src/template/filters/where_filter.rs`, `src/template/filters/jsonify.rs` for examples)
- All existing tests must continue to pass

## Implementation Notes

- Each filter should be in its own file under `src/template/filters/` (e.g., `number_of_words.rs`, `group_by.rs`, `xml_escape.rs`, `truncatewords.rs`)
- Each filter file must declare the filter struct with `#[derive(Clone, ParseFilter, FilterReflection)]` and implement `Filter`
- Re-export from `src/template/filters/mod.rs`
- Register in `TemplateEngine::builder()` with `.filter(filters::FilterName)`

### Filter specifications (matching Jekyll behavior)

**`number_of_words`:**
- Input: string
- Output: integer (as scalar)
- Behavior: split input on whitespace, count non-empty segments
- Example: `"Hello world foo" | number_of_words` => `3`
- Empty string returns `0`

**`group_by`:**
- Input: array of objects
- Parameter: property name (string)
- Output: array of objects, each with `name` (the property value), `items` (array of matching objects), `size` (count)
- Example: `[{type: "a", v: 1}, {type: "b", v: 2}, {type: "a", v: 3}] | group_by: "type"` => `[{name: "a", items: [...], size: 2}, {name: "b", items: [...], size: 1}]`
- Items missing the property are grouped under an empty string name
- Non-array input returns empty array

**`xml_escape`:**
- Input: string
- Output: string with XML entities escaped
- Must escape: `&` -> `&amp;`, `<` -> `&lt;`, `>` -> `&gt;`, `"` -> `&quot;`, `'` -> `&apos;`
- Important: `&` must be escaped first to avoid double-escaping

**`truncatewords`:**
- Input: string
- Parameter 1: number of words (integer)
- Parameter 2 (optional): ellipsis string (default `"..."`)
- Output: string truncated to N words with ellipsis appended
- Example: `"one two three four" | truncatewords: 2` => `"one two..."`
- Example: `"one two three" | truncatewords: 2, "--"` => `"one two--"`
- If input has fewer words than N, return the input unchanged (no ellipsis)

## Dependencies

- Issue 07 (template filters) -- must be `.done.md` (already done)
- Issue 06 (template engine core) -- must be `.done.md` (already done)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes with all new and existing tests
- [ ] `number_of_words` filter is registered and works: `"hello world" | number_of_words` renders `2`
- [ ] `group_by` filter is registered and works: groups array by property into `{name, items, size}` structure
- [ ] `xml_escape` filter is registered and works: `"<p>Tom & Jerry</p>" | xml_escape` renders `&lt;p&gt;Tom &amp; Jerry&lt;/p&gt;`
- [ ] `truncatewords` filter is registered and works: `"one two three four" | truncatewords: 2` renders `one two...`
- [ ] Each filter has its own source file under `src/template/filters/`
- [ ] Each filter is re-exported from `src/template/filters/mod.rs`
- [ ] Each filter is registered in `TemplateEngine::builder()` in `src/template/engine.rs`
- [ ] The implementation is generic -- no site-specific hardcoding

## Test Scenarios

### Unit: number_of_words

- `"Hello world"` => `2`
- `""` (empty string) => `0`
- `"  spaces  between  words  "` => `3` (whitespace-only segments ignored)
- `"single"` => `1`
- `"one\ntwo\tthree"` => `3` (tabs and newlines count as whitespace)

### Unit: group_by

- Array with 3 items, 2 sharing a property value => 2 groups, sizes 2 and 1
- Array where all items share the same property value => 1 group
- Array where some items are missing the grouping property => those items grouped under empty string name
- Empty array => empty array result
- Non-array input => empty array result
- Each group object has `name`, `items`, and `size` keys
- Verify `size` equals `items.length` for each group

### Unit: xml_escape

- `"<p>Hello</p>"` => `"&lt;p&gt;Hello&lt;/p&gt;"`
- `"Tom & Jerry"` => `"Tom &amp; Jerry"`
- `'She said "hi"'` => `"She said &quot;hi&quot;"`
- `"it's"` => `"it&apos;s"`
- `"no special chars"` => `"no special chars"` (unchanged)
- `""` (empty string) => `""` (unchanged)
- `"&amp;"` => `"&amp;amp;"` (does not skip already-escaped entities -- matches Jekyll behavior)
- String with all five special characters combined

### Unit: truncatewords

- `"one two three four five"` with N=3 => `"one two three..."`
- `"one two three four five"` with N=3 and custom ellipsis `"--"` => `"one two three--"`
- `"one two"` with N=5 => `"one two"` (fewer words than N, no ellipsis)
- `""` with N=3 => `""` (empty input)
- `"one"` with N=1 => `"one..."` -- wait, Jekyll returns "one..." only if there are more words. Since there is exactly 1 word and N=1, the input is not truncated. Correct: `"one"` (no ellipsis, since word count equals N)
- `"  one  two  "` with N=1 => `"one..."` (extra whitespace handled, there are more words)

### Integration: template rendering

- Parse and render a template `{{ content | number_of_words }}` with content "hello beautiful world" => `3`
- Parse and render a template `{{ text | xml_escape }}` with text `<b>bold & "quoted"</b>` => correct escaped output
- Parse and render a template `{{ text | truncatewords: 2 }}` with text "a b c d" => `a b...`
- Parse and render a template using `group_by` on an array variable => verify grouped output structure
