# Issue 22: Jekyll Compatibility Research

## Description

Research Jekyll features that are NOT yet covered by issues 01-21 to ensure rustkyll can be a drop-in replacement for Jekyll. Find real Jekyll projects on the internet (GitHub Pages sites, open-source Jekyll sites) and test rustkyll against them to discover missing features.

## Dependencies

- Issue 19 (CLI and full build -- need a working build to test against)

## Scope

### Research Phase

Investigate Jekyll features not covered in existing issues:

- **Pagination** (`jekyll-paginate`, `jekyll-paginate-v2`) -- paginated listing pages
- **Sass/SCSS processing** -- Jekyll compiles `.scss` files to CSS
- **CoffeeScript support** -- `.coffee` to JS compilation
- **Drafts** (`_drafts/` directory) -- unpublished posts
- **Data file formats** -- CSV and JSON data files (not just YAML)
- **Custom plugins** -- common community plugins used by GitHub Pages
- **Excerpts** -- automatic excerpt generation (first paragraph)
- **Categories and tags** -- category/tag archive pages
- **Front matter defaults** -- path/type-based front matter defaults (beyond layout)
- **Hooks and generators** -- custom Ruby generators
- **Internationalization** -- multi-language support
- **SEO plugin** (`jekyll-seo-tag`) -- automatic meta tags
- **Redirect plugin** (`jekyll-redirect-from`) -- redirect pages
- **Optional front matter** (`jekyll-optional-front-matter`) -- render .md without front matter
- **Relative links plugin** (`jekyll-relative-links`) -- .md links to HTML
- **GitHub metadata plugin** (`jekyll-github-metadata`) -- repo info
- **Liquid advanced features** -- `tablerow`, `cycle`, `increment/decrement`, `raw`, `comment`, `render`
- **Static files API** -- `site.static_files` with properties (path, modified_time, extname)
- **Permalink variables** -- `:year`, `:month`, `:day`, `:categories`, `:slug`, etc.
- **Timezone handling** -- `timezone` config option
- **Future posts** -- `future: false` config to hide future-dated posts

### Testing Phase

- Find 3-5 open-source Jekyll sites on GitHub (varying complexity)
- Clone them into `websites/` directory (gitignored)
- Attempt to build each with rustkyll
- Document which features are missing or broken
- Categorize findings by importance (critical / nice-to-have / rare)

### Test Website Setup

The `websites/` directory is gitignored. To set up test sites:

```bash
mkdir -p websites
cd websites
git clone https://github.com/<org>/<repo>.git  # repeat for each test site
```

Document the exact repos and commit hashes used in the report so results are reproducible.

### Deliverable

A report documenting:
1. Missing features found, ranked by how commonly they appear in real Jekyll sites
2. For each feature: what it does, which test sites use it, complexity estimate
3. Recommendations for which features to implement as new issues
4. List of test websites used (repo URLs + commit hashes)

## Notes

- Focus on features used by GitHub Pages sites (the most common Jekyll deployment)
- The goal is to become a practical drop-in replacement, not 100% feature parity with every obscure Jekyll option
- This issue produces a report, not code -- new implementation issues will be created based on findings
- Test websites go in `websites/` (gitignored) -- see setup instructions above
