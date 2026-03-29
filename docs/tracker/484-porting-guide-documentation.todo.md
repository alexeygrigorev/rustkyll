# Issue 484: "Porting your website to Rustkyll" documentation

## Problem

Users need clear guidance on migrating from Jekyll to rustkyll. Most
sites are drop-in replacements, but some need adaptation. There's no
documentation covering this.

## Scope

Create `docs/porting-guide.md` covering:

### 1. Quick Start (drop-in replacement)
- Install rustkyll
- Run `rustkyll build` in your Jekyll site directory
- Compare output with `scripts/dom_compare.py`
- Most sites work immediately

### 2. Theme Support
- **Bundled themes** (minima, etc.): how rustkyll resolves gem themes
- **Remote themes** (jekyll-remote-theme): current support status
- **GitHub Pages themes** (cayman, architect, etc.): fully supported
- **Custom themes**: _layouts/ and _includes/ work as-is

### 3. Plugin Compatibility
- **Fully supported plugins**: jekyll-seo-tag, jekyll-feed, jekyll-sitemap,
  jekyll-paginate, jekyll-github-metadata
- **Partially supported**: jekyll-gist, jekyll-redirect-from (planned)
- **Not supported**: custom Ruby plugins (generators, tags, filters)

### 4. Porting Custom Plugins
- **Custom Liquid filters**: list common ones rustkyll supports, how to
  check if yours works
- **Custom Liquid tags**: same
- **Custom generators**: use the YAML page generator (#483) as alternative
- **bitcoin-org case study**: step-by-step porting of TranslatePageGenerator

### 5. Known Differences
- Smart quotes/typography: minor differences in some edge cases
- Syntax highlighting: syntect vs Rouge token class names
- Date formatting: timezone handling
- SASS compilation: grass vs sassc

### 6. Troubleshooting
- "My layout isn't applied" — check theme resolution
- "Pages are missing" — likely a generator plugin
- "Syntax highlighting looks different" — token class mapping
- "Build is slower than Jekyll" — check SASS, large page counts

## Deliverable

A single `docs/porting-guide.md` file, well-structured with examples.

## Baseline

DTC 790/790. Documentation only — no code changes.
