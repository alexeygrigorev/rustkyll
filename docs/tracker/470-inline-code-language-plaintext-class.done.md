# Issue 470: Inline code language-plaintext class (mlbookcamp, muan-blog)

## Problem

Rustkyll adds `class="language-plaintext highlighter-rouge"` to inline `<code>` elements
in kramdown mode. However, not all kramdown versions add `language-plaintext`:

- **DTC** (kramdown 2.4.0, pinned): Jekyll outputs `<code class="language-plaintext highlighter-rouge">` -- rustkyll matches correctly.
- **mlbookcamp** (kramdown 2.5.2): Jekyll outputs `<code class="highlighter-rouge">` (no `language-plaintext`) -- rustkyll wrongly adds `language-plaintext`.
- **muan-blog** (CommonMarkGhPages): Jekyll outputs bare `<code>` (no class at all) -- rustkyll wrongly adds `class="highlighter-rouge language-plaintext"`.

The `language-plaintext` class is also added to code block wrapper `<div>` elements
on mlbookcamp (same version-dependent behavior).

## Root Cause

The inline code class addition in `src/frontmatter.rs` (`add_inline_code_class_to_events_impl`)
always adds `language-plaintext highlighter-rouge` when `add_code_classes` is true.
The flag `use_kramdown_code_classes` is set based on whether `markdown:` is kramdown,
but does not account for kramdown version differences.

For muan-blog (CommonMarkGhPages), the `use_kramdown_code_classes` flag should be `false`,
but DOM comparison shows the classes are still being added -- investigate whether the flag
is not being propagated to all code paths, or if the DOM comparison is from a stale build.

Key code locations:
- `src/frontmatter.rs:206-222` -- `add_inline_code_class_to_events_impl` adds the class
- `src/template/layout.rs:52-54` -- `use_kramdown_code_classes` flag
- `src/main.rs:470-475` -- flag is set based on `markdown:` config
- `src/kramdown.rs:4558` -- code block wrapper div also uses `language-plaintext`

## Affected Sites

- **mlbookcamp** (9 files with diffs): Both inline `<code>` and code block wrapper `<div>` get extra `language-plaintext` class. Jekyll (kramdown 2.5.2) does not add it.
- **muan-blog** (1 file, `pages/issues.html`): Inline `<code>` gets `class='highlighter-rouge language-plaintext'` but Jekyll (CommonMarkGhPages) produces bare `<code>` with no class.

## Scope

Two distinct fixes:

1. **mlbookcamp (kramdown version-dependent):** When a site uses kramdown >= 2.5, do NOT add `language-plaintext` to inline `<code>` or code block wrapper `<div>`. Detection options: (a) check Gemfile.lock for kramdown version, (b) add a config option, or (c) detect based on the resolved kramdown gem version. The simplest approach may be to read Gemfile.lock if it exists and parse the kramdown version.

2. **muan-blog (CommonMark):** Ensure `use_kramdown_code_classes=false` actually prevents ALL class additions to inline code. Debug why muan-blog still gets classes despite using CommonMarkGhPages.

## Dependencies

None.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` passes
- [ ] DTC DOM baseline: 790/790 (must not regress)
- [ ] mlbookcamp DOM match: >= 6/15 (baseline is 6/15; inline code class fix should improve this)
- [ ] muan-blog DOM match: >= 2197/2218 (baseline from DOM report: 2197 matching out of 2218)
- [ ] For mlbookcamp: inline `<code>` elements have `class="highlighter-rouge"` (no `language-plaintext`)
- [ ] For mlbookcamp: code block wrapper `<div>` elements have `class="highlighter-rouge"` (no `language-plaintext`)
- [ ] For muan-blog: inline `<code>` elements have NO class attribute (bare `<code>`)
- [ ] Tests pass: `./scripts/cargo-safe test`
- [ ] At least 3 new or updated unit tests covering:
  - kramdown mode with language-plaintext (DTC behavior preserved)
  - kramdown mode without language-plaintext (mlbookcamp behavior)
  - CommonMark mode with no class on inline code (muan-blog behavior)

## Test Scenarios

### Unit: Inline code class behavior
- Parse markdown with backtick code in "kramdown 2.4" mode, verify `<code class="language-plaintext highlighter-rouge">` is produced
- Parse markdown with backtick code in "kramdown 2.5+" mode (or equivalent flag), verify `<code class="highlighter-rouge">` is produced (no `language-plaintext`)
- Parse markdown with backtick code in CommonMark mode, verify bare `<code>` with no class

### Unit: Code block wrapper div
- Render a fenced code block with no language in kramdown 2.4 mode, verify wrapper div has `class="language-plaintext highlighter-rouge"`
- Render a fenced code block with no language in kramdown 2.5+ mode, verify wrapper div has `class="highlighter-rouge"` (no `language-plaintext`)

### Integration: Site-level verification
- Build mlbookcamp site, check that inline `<code>` in `index.html` has `class="highlighter-rouge"` (not `language-plaintext highlighter-rouge`)
- Build muan-blog site, check that inline `<code>` in `pages/issues.html` has no class attribute
- Build DTC site, verify 790/790 DOM match is preserved

## Baselines (recorded at grooming time)

- DTC: 790/790
- mlbookcamp: 6/15 matching files (9 files with differences)
- muan-blog: 2211/2218 matching files (from DOM report: 2218 common files, 7 with diffs, but only 5 of the 7 diffs involve language-plaintext in 1 file)
