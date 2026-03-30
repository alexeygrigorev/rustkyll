# Issue 344: Support al-folio custom Liquid tags and filters

## Problem

The al-folio demo renders raw Liquid markup in rustkyll because the theme depends on custom tags and filters implemented as Jekyll plugins in `_plugins/`:

### Tags from `_plugins/`:
- `details` / `enddetails` -- block tag that renders `<details><summary>...</summary>...</details>` (from `_plugins/details.rb`)
- `file_exists` -- tag that checks if a file exists and returns `"true"` or `"false"` (from `_plugins/file-exists.rb`)

### Tags referenced in templates but not in `_plugins/`:
- `cite` -- BibTeX citation tag (from `jekyll-scholar` gem)
- `reference` -- BibTeX reference tag (from `jekyll-scholar` gem)
- `bibliography` -- BibTeX bibliography listing (from `jekyll-scholar` gem)
- `tabs` / `endtabs` -- tab container block tag
- `tab` / `endtab` -- individual tab block tag
- `quote` / `endquote` -- styled quote block tag
- `jupyter_notebook` -- Jupyter notebook embed tag
- `social_links` -- social media links tag
- `bust_file_cache` -- cache-busting tag for asset URLs
- `twitter` -- Twitter embed tag

These blockers affect blog posts, notebook embeds, figures, and layout pages. When unrecognized, the Liquid template engine either errors or outputs raw `{% ... %}` markup.

## Scope

1. Implement the `details` block tag to produce correct `<details><summary>...</summary>...</details>` HTML.
2. Implement the `file_exists` tag to check file existence at the site source path and return `"true"` or `"false"`.
3. For tags that require complex external data (e.g., `cite`, `reference`, `bibliography` from jekyll-scholar; `jupyter_notebook`), register them as no-op or passthrough tags that produce empty output or a placeholder comment, so that the template engine does not error.
4. Register `tabs`/`tab`, `quote`, `social_links`, `bust_file_cache`, and `twitter` as no-op block/inline tags as appropriate.
5. Verify the affected al-folio pages no longer emit raw Liquid markup in generated HTML.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with tests covering `details` block tag (caption + body rendering) and `file_exists` tag (true/false return)
- [ ] Building `websites/al-folio/` with rustkyll produces no unknown-tag errors for `details`, `file_exists`, `cite`, `reference`, `bibliography`, `tabs`, `tab`, `quote`, `jupyter_notebook`, `social_links`, `bust_file_cache`, or `twitter`
- [ ] The al-folio pages that use `{% details %}...{% enddetails %}` blocks produce `<details>` HTML elements in the output
- [ ] The al-folio pages that use `{% file_exists %}` do not show raw Liquid markup
- [ ] Pages using scholar tags (`cite`, `reference`, `bibliography`) do not show raw Liquid markup; they produce empty output or a comment
- [ ] The al-folio DOM comparison improves or holds steady compared to the #235 baseline
- [ ] Any remaining unsupported al-folio tags or filters discovered during implementation are tracked in follow-up issues referencing `#235`
- [ ] DTC DOM count remains at 788/790 or above

## Test Scenarios

### Unit: details block tag
- Parse `{% details Summary text %}Body content{% enddetails %}`, verify output is `<details><summary>Summary text</summary><p>Body content</p></details>`
- Parse `{% details %}{% enddetails %}` with empty summary, verify graceful handling
- Parse details with Markdown body content, verify Markdown is rendered inside the details body
- Parse details with Unicode summary text (e.g., `Zusammenfassung`)

### Unit: file_exists tag
- `{% file_exists /path/to/existing_file %}` returns `"true"` when the file exists relative to site source
- `{% file_exists /path/to/nonexistent %}` returns `"false"`
- `{% file_exists %}` with empty path returns `"false"` gracefully

### Unit: no-op tags
- `{% cite key %}` produces empty output without error
- `{% tabs %}{% tab Title %}content{% endtab %}{% endtabs %}` produces empty or passthrough output without error
- `{% quote %}text{% endquote %}` produces empty or passthrough output without error
- `{% jupyter_notebook path %}` produces empty output without error

### Integration: al-folio page rendering
- Build `websites/al-folio/` with rustkyll and inspect pages that use `details`, `file_exists`, and scholar tags
- Verify generated HTML contains `<details>` elements where expected
- Verify no raw `{% details %}`, `{% cite %}`, `{% file_exists %}` markup appears in any generated HTML page

## Dependencies

- Issue #235 (must be `.done.md` or `.in-progress.md`)
