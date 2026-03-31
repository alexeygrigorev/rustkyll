# Issue 460: muan-blog escaped dash \- becomes <hr>

## Problem

muan-blog (uses `CommonMarkGhPages` markdown engine) has pages with `\-` at
line start. Jekyll/kramdown treats `\-` as an escaped dash producing literal
`-`. Rustkyll must match this behavior in the CommonMarkGhPages preprocessing
path.

### Affected source files

1. `_posts/2020-06-06-thoughts-on-reparations.md` line 63: `\- Mu-An @ Brooklyn, NY`
   - Preceded by raw `<br>` tag on line 62
   - Jekyll: bare `<br>` + bare `\- Mu-An @ Brooklyn, NY` (HTML block context)
   - Rustkyll: `<p><br> \- Mu-An @ Brooklyn, NY</p>` (wrapped in `<p>`)

2. `_posts/2020-10-02-leaving-github.md` line 42: `\- Mu-An @ Brooklyn, already happier.`
   - Jekyll: `<p>- Mu-An @ Brooklyn, already happier.</p>` (correct: backslash consumed)
   - Rustkyll: `<p>- Mu-An @ Brooklyn, already happier.</p>` (correct: matches Jekyll)

3. `_notes/2023-10-04-uu.md` line 9: `> \- comrade tripp` (inside blockquote)
   - Jekyll: `- comrade tripp` (correct: backslash consumed)
   - Rustkyll: `- comrade tripp` (correct: matches Jekyll)

The structural difference on the reparations page (bare `<br>` + text vs
`<p>`-wrapped) is a separate concern. The core issue is ensuring `\-` is
reliably treated as an escaped dash in the CommonMarkGhPages preprocessing
path, preventing any scenario where it could trigger horizontal rule (`<hr>`)
rendering.

## Scope

Add a preprocessing step in the CommonMarkGhPages rendering path
(`markdown_to_html_with_options` in `src/frontmatter.rs`) that converts `\-`
at line start to literal `-` before pulldown-cmark processes the content. This
matches Jekyll's commonmarker gem behavior where backslash-escaped dashes are
consumed.

## Baseline

- DTC: 790/790
- muan-blog: 36/39

## Acceptance Criteria

- [ ] `./scripts/cargo-safe build` compiles without errors
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes
- [ ] `./scripts/cargo-safe test` passes with all existing + new tests
- [ ] New unit test: `\- text` at line start produces `<p>- text</p>` (backslash consumed, dash literal) in CommonMarkGhPages mode
- [ ] New unit test: `\-` inside blockquote produces literal `-` in CommonMarkGhPages mode
- [ ] New unit test: `---` (three dashes) still produces `<hr>` (escape does not interfere with real horizontal rules)
- [ ] New unit test: `\-` in middle of line (not at start) is still handled correctly
- [ ] Build muan-blog and verify leaving-github page renders `- Mu-An @ Brooklyn, already happier.` (backslash consumed)
- [ ] Build muan-blog and verify reparations page `\- Mu-An @ Brooklyn, NY` renders correctly
- [ ] DTC DOM match count must not drop below 790/790
- [ ] muan-blog DOM match count must not drop below 36/39

## Test Scenarios

### Unit: Escaped dash preprocessing (CommonMarkGhPages mode)

- Input `\- text` at line start → output contains `- text` (literal dash, no backslash)
- Input `---` at line start → output contains `<hr>` (real horizontal rule unaffected)
- Input `> \- quoted text` → output contains `- quoted text` (escaped dash in blockquote)
- Input `text with \- dash in middle` → output contains `- dash in middle`
- Input `\-\-\-` at line start → output does NOT contain `<hr>` (all three dashes escaped)

### Integration: muan-blog rendering

- Build muan-blog, check leaving-github page: `- Mu-An` appears, `\- Mu-An` does NOT appear
- Build muan-blog, check reparations page: `\- Mu-An` or `- Mu-An` renders correctly
- Build muan-blog, check uu note: `- comrade tripp` appears in blockquote

### Regression: DTC and other sites

- DTC DOM comparison stays at 790/790
- muan-blog DOM comparison stays at 36/39 or improves

## Dependencies

None.

## Log

### [PM] 2026-03-31
- Investigated muan-blog source: 3 files with `\-` pattern
- Verified leaving-github and uu note already render correctly (backslash consumed)
- Reparations page has structural difference (bare `<br>` + text vs `<p>`-wrapped)
- Confirmed muan-blog uses CommonMarkGhPages engine (pulldown-cmark path)
- DTC baseline: 790/790
- muan-blog baseline: 36/39
