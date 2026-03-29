# Issue 508: Implement al-folio custom Liquid tags (tabs, details, quote, cite, etc.)

## Problem

24 al-folio pages contain raw Liquid syntax in the generated HTML because rustkyll does not implement several custom Liquid tags that al-folio defines via its Jekyll plugins. These tags appear as literal `{% tabs %}`, `{% details %}`, etc. in the output.

Affected custom tags (observed in al-folio output):

### Block tags (paired open/close)
- `{% tabs %}` / `{% endtabs %}` -- Tabbed content panels
- `{% details %}` / `{% enddetails %}` -- Collapsible details/summary blocks
- `{% quote %}` / `{% endquote %}` -- Styled blockquotes with attribution

### Inline tags (Jekyll Scholar plugin: `jekyll/scholar`)
- `{% cite %}` -- Bibliography citation references
- `{% reference %}` -- Full bibliography reference entries
- `{% bibliography %}` -- Render full bibliography from BibTeX

### Other inline tags
- `{% twitter %}` -- Embedded tweets
- `{% jupyter_notebook %}` -- Embedded Jupyter notebook HTML
- `{% post_url %}` -- Link to a post by filename (may already be partially supported)

### Filters
- `file_exists` -- Check if a file exists in the site source
- `bust_file_cache` -- Append cache-busting query parameter to file URLs

## Relationship to Issue 344

Issue #344 identified the same tags but has minimal acceptance criteria and no implementation detail. This issue supersedes #344 with concrete scope and testable criteria. Issue #344 should be closed in favor of this issue.

**Important note:** Many of the Liquid leaks observed in the current output (24 pages) are caused by issue #505 (missing `.liquid` layout support). When layouts are not found, rustkyll falls back to writing raw markdown-to-HTML content without Liquid processing. Once #505 is fixed, the leak count will drop significantly. The remaining leaks will be from genuine custom tags that need implementation.

## Scope

1. Implement the block tags (`tabs`, `details`, `quote`) as custom Liquid tags that produce the correct HTML structure.
2. Implement `cite` and `reference` tags as no-op or minimal stubs (full BibTeX support is out of scope; the goal is to stop Liquid leaks).
3. Implement `twitter` and `jupyter_notebook` as stubs that produce a placeholder or pass-through.
4. Implement `file_exists` and `bust_file_cache` filters.
5. Eliminate all raw Liquid syntax from al-folio output.

## Baseline

- al-folio Liquid leaks: 24 pages
- DTC DOM baseline: 790/790

## Acceptance Criteria

- [ ] No al-folio page contains raw `{% tabs %}`, `{% details %}`, `{% quote %}`, `{% cite %}`, `{% reference %}`, `{% twitter %}`, or `{% jupyter_notebook %}` in the generated HTML.
- [ ] `{% tabs %}` produces a `<ul>` with tab navigation and `<div>` panels (matching al-folio's expected output).
- [ ] `{% details %}` produces `<details><summary>` HTML elements.
- [ ] `{% quote %}` produces a styled `<blockquote>` with optional attribution.
- [ ] `{% cite %}` and `{% reference %}` produce reasonable output (even if simplified) without leaking Liquid.
- [ ] `file_exists` filter returns `true`/`false` based on whether the file exists in the source directory.
- [ ] `bust_file_cache` filter appends a cache-busting query parameter.
- [ ] al-folio Liquid leak count drops from 24 to 0.
- [ ] DTC DOM match count does not drop below 790/790.
- [ ] `cargo build` compiles without errors; `cargo clippy` clean; `cargo fmt` clean.

## Test Scenarios

### Unit: tabs tag
- Parse `{% tabs group %} {% tab group label %} content {% endtab %} {% endtabs %}`, verify output contains tab navigation and panels.

### Unit: details tag
- Parse `{% details Summary text %} body content {% enddetails %}`, verify output is `<details><summary>Summary text</summary>body content</details>`.

### Unit: quote tag
- Parse `{% quote Author %} quote text {% endquote %}`, verify styled blockquote with attribution.

### Unit: cite/reference tags
- Parse `{% cite einstein2015 %}`, verify no Liquid leak (output can be a stub like `[einstein2015]`).

### Unit: filters
- Test `file_exists` returns true for existing files and false otherwise.
- Test `bust_file_cache` appends query parameter to URLs.

### Integration: al-folio build
- Build al-folio and grep output for raw `{%` -- verify zero matches in HTML content.
- Verify blog/2024/tabs page has actual tab HTML structure.

## Dependencies

- Issue #235 (al-folio site is set up)
- Issue #505 (layouts must be applied for full page context)
