# Issue #592: Kramdown bold `**word**` rendered as `<em>*word*</em>` in certain contexts

## Problem

In kramdown mode, `**word**` (bold) is sometimes rendered as `<em>*word*</em>`
(italic with literal asterisks) instead of `<strong>word</strong>`. This happens
when bold markers appear near other emphasis markers or in complex inline formatting.

**Source markdown (chirpy getting-started):**
```markdown
In the **Source** section (under _Build and deployment_), select [**GitHub Actions**]...
```

**Jekyll output (correct):**
```html
In the <strong>Source</strong> section (under <em>Build and deployment</em>), select <a href="..."><strong>GitHub Actions</strong></a>...
```

**Rustkyll output (broken):**
```html
In the <em>*Source*</em> section (under <em>Build and deployment</em>), select <a href="..."><strong>GitHub Actions</strong></a>...
```

Note: `[**GitHub Actions**]` renders correctly as `<strong>`, but standalone
`**Source**` in the same paragraph renders as `<em>*Source*</em>`. This suggests
the double-asterisk parser is consuming only one `*` as emphasis opener, leaving
the other `*` as literal text.

## Affected Sites

- **chirpy** (14/17): getting-started page has this as 1 of 12 diffs
  (`<strong>` vs `<em>` mismatch on `**Source**`)
- Potentially any site with `**bold**` near `_italic_` in kramdown mode

## Root Cause

The kramdown emphasis parser likely has a precedence/nesting issue. When it
encounters `**Source**` in a paragraph that also contains `_italic_` markers,
it may be:
1. Treating the first `*` as an emphasis opener (for `_..._` style matching)
2. Then treating `*Source*` as italic content
3. Leaving `*` as literal around the word

The fix should ensure that `**` is always parsed as strong emphasis when it
appears as a matched pair, regardless of surrounding `_` emphasis markers.

## Acceptance Criteria

- [ ] `**word**` renders as `<strong>word</strong>` in all contexts
- [ ] `**word**` adjacent to `_italic_` text renders correctly
- [ ] `_italic **bold** italic_` renders as `<em>italic <strong>bold</strong> italic</em>`
- [ ] Existing emphasis handling not regressed (single `*`, `_`, `***` combos)
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes
- [ ] DTC DOM match count must not drop below 788/790
- [ ] Chirpy getting-started page `**Source**` renders as `<strong>Source</strong>`

## Test Scenarios

### Unit: Bold near italic
- `In the **Source** section` -> `<strong>Source</strong>`
- `_Settings_ tab, **Source** section` -> `<em>Settings</em> tab, <strong>Source</strong> section`
- `_italic **bold** italic_` -> `<em>italic <strong>bold</strong> italic</em>`

### Unit: Bold in complex inline formatting
- `In the **Source** section (under _Build and deployment_)` -> both strong and em correct
- `[**GitHub Actions**](url)` -> link with `<strong>` (verify no regression)

### Unit: Edge cases
- `**bold** _italic_ **bold**` -> two `<strong>` and one `<em>`
- `*italic* **bold** *italic*` -> correct nesting
- `***bold italic***` -> `<strong><em>bold italic</em></strong>` or equivalent

### Integration: Chirpy getting-started
- Build chirpy site
- Verify posts/getting-started/index.html has `<strong>Source</strong>` (not `<em>*Source*</em>`)

## Dependencies

None.

## DOM Baseline

- DTC: 788/790 matched
- Chirpy: 14/17 matched, 101 total diffs

## Log

### [SWE] 2026-04-02

**Root Cause Analysis:**
The `escape_mixed_delimiter_emphasis` function in `src/kramdown.rs` is a pre-processor
that escapes inner delimiters in mixed-delimiter emphasis (e.g., `_*text*_` -> `_\*text\*_`).
The bug had two parts:

1. `find_mixed_emphasis_span` would skip over matching closing delimiters when no inner
   delimiters had been found yet, then keep scanning. This caused `_Settings_` to be
   treated as starting a span that extended all the way to `_Build and deployment_`,
   consuming `**Source**` as inner content and escaping its `*` to `\*`.

2. IAL blocks like `{:target="_blank"}` contain `_` characters that were being treated
   as emphasis delimiters, causing false matches across IAL boundaries.

**Fix 1: Stop scanning past matched closing delimiters (NoMix)**
- Wrote tests: test_issue592_bold_near_underscore_italic, test_issue592_escape_chirpy_exact_line,
  test_issue592_chirpy_full_render, and 5 more (8 total)
- Ran tests: 3 FAIL -- `**Source**` rendered as `<em>*Source*</em>`, escape function
  produced `\*\*Source\*\*`
- Implemented fix: Changed `find_mixed_emphasis_span` to return `MixedEmphasisResult::NoMix`
  when it finds a matching closing delimiter with no inner delimiters, instead of continuing
  to scan past it. Updated caller to skip past the entire non-mixed span.
- Ran tests: 7 of 8 PASS (chirpy tests pass), but DTC DOM regression: 787/790 with
  Daniel Egbo `**bold**` inside link broken by IAL `_blank` underscore.

**Fix 2: Skip IAL blocks in emphasis scanning**
- Identified that `{:target="_blank"}` underscores were being treated as emphasis delimiters
- Added IAL block skipping (`{:...}`) in both `escape_mixed_delimiter_emphasis` main loop
  and `find_mixed_emphasis_span` scanner
- Ran tests: 8/8 PASS
- DTC DOM: 788/790 with 8 total diffs (matches baseline exactly)
- Chirpy DOM: 14/17 with 99 total diffs (improved from 101, -2 diffs from Source fix)

**Summary:**
- Files modified: src/kramdown.rs
- Tests added: 8 unit tests for issue 592
- Build results: 4049 tests pass, 0 fail, clippy clean, fmt clean
- DTC DOM: 788/790 (0 regression, baseline maintained)
- DTC build time: 0.955s (under 1.0s)
- Chirpy DOM: 14/17, 99 diffs (improved from 101)
- Chirpy getting-started: `<strong>Source</strong>` confirmed correct
