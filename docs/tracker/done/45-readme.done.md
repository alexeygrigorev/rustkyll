# Issue 45: Create README

## Problem

The project has no README. Users need to understand what rustkyll is, how to install and use it, how it was built, which sites were tested, and what the known limitations are.

## Requirements

- Write a README.md at the project root
- Do not use bold or italic formatting (no ** or * markup)
- Sections to include:
  1. What rustkyll is (Rust static site generator replacing Jekyll, goal: speed up DTC website builds)
  2. How to install / set up
  3. How to use (build, serve commands)
  4. How it was written (agent-driven development process, issue tracker, PM/SWE/QA pipeline)
  5. Which sites were tested (DTC site, alexeygrigorev repos, complex external sites like bitcoin.org, opensource.guide, etc.)
  6. Known limitations (features not yet supported, sites that don't build)
- Keep it practical and concise

## Dependencies

None.

## Acceptance Criteria

- [ ] A file `README.md` exists at the project root (`/home/alexey/git/rustkyl/README.md`)
- [ ] The file contains no `**` or `*` markup (no bold or italic formatting)
- [ ] The README contains a section explaining what rustkyll is -- must mention "Rust", "Jekyll", and "static site generator"
- [ ] The README contains installation/setup instructions that mention `cargo build` or equivalent
- [ ] The README contains usage instructions that document both `build` and `serve` subcommands with their key flags (`--source`, `--destination`, `--incremental`, `--port`, `--livereload`)
- [ ] The README contains a section about the agent-driven development process, mentioning the PM/SWE/QA pipeline and the file-based issue tracker in `docs/tracker/`
- [ ] The README contains a section listing tested sites, covering at minimum:
  - The DataTalks.Club site (`datatalksclub.github.io`)
  - At least 3 alexeygrigorev repos (e.g. `kids-horror-stories-ru`, `alexeygrigorev.github.io`, `snippets`)
  - At least 4 external complex sites (e.g. `bitcoin.org`, `opensource.guide`, `hyde`, `wtf-html-css`)
- [ ] The README contains a known-limitations section listing at least 2 concrete limitations (e.g. no Sass/SCSS compilation, no plugin system beyond seo-tag)
- [ ] The README is well-structured with clear section headings (using `#`, `##`, or `###`)
- [ ] `cargo build` still compiles without errors (README does not break the build)
- [ ] `cargo test` still passes (README does not break existing tests)

## Test Scenarios

This issue is documentation-only. There is no Rust code to write and no new unit or integration tests to add. Verification is manual/structural:

### Structural: README exists and has correct formatting
- Verify `README.md` exists at the project root
- Verify the file contains no `**` or `*` markup
- Verify the file is valid Markdown (section headings, code blocks properly closed)

### Content: All required sections present
- Grep for section headings to confirm all six required topics are covered
- Verify the "what is it" section mentions Rust, Jekyll, static site generator
- Verify the usage section documents `build` and `serve` subcommands
- Verify the tested sites section includes DTC, alexeygrigorev repos, and external sites
- Verify the limitations section lists concrete items

### Regression: Existing build and tests unaffected
- Run `cargo build` and confirm it succeeds
- Run `cargo test` and confirm all existing tests pass
