# rustkyll

A static site generator written in Rust, designed as a drop-in replacement for Jekyll. The primary goal is to speed up site builds for the DataTalks.Club website (datatalksclub.github.io), which has hundreds of pages across multiple collections.

rustkyll reads the same source files as Jekyll -- Markdown with YAML front matter, Liquid templates, YAML data files, and collection directories -- and produces equivalent HTML output.


## Installation

Prerequisites:

- Rust toolchain (stable, 2021 edition or later)
- Git (to clone the repository)

Clone and build:

```
git clone https://github.com/alexeygrigorev/rustkyll.git
cd rustkyll
cargo build --release
```

The binary will be at `target/release/rustkyll`.


## Usage

### build

Generate the static site from source files:

```
rustkyll build --source /path/to/site --destination /path/to/output
```

Flags:

- `--source` -- path to the Jekyll site directory (default: current directory)
- `--destination` -- output directory for generated files (default: `_site`)
- `--incremental` -- only rebuild pages whose source files have changed
- `--force` -- force a full rebuild, ignoring the incremental manifest

### serve

Build and serve the site locally with a development server:

```
rustkyll serve --source /path/to/site --port 4000
```

Flags:

- `--source` -- path to the Jekyll site directory (default: current directory)
- `--destination` -- output directory (default: `_site`)
- `--port` -- port number for the HTTP server (default: 4000)
- `--livereload` -- enable live reload in the browser when files change (default: enabled)
- `--no-livereload` -- disable live reload


## Features

- YAML front matter and Markdown rendering (via pulldown-cmark)
- Liquid template engine with tags: for, if/elsif/else, unless, assign, capture, include, highlight, break
- 40+ Liquid filters including where, sort, date, markdownify, slugify, jsonify, relative_url, and more
- Layout chain (nested layouts wrapping content)
- Collections: posts, pages, and custom collections with configurable permalinks
- Data files: YAML data loaded from `_data/` and accessible as `site.data`
- Front matter defaults (scoped by path and type, matching Jekyll defaults)
- Categories and tags with `site.categories` and `site.tags`
- Sitemap generation (sitemap.xml)
- RSS/Atom feed generation (feed.xml)
- JSON-LD structured data (articles, books, podcasts)
- jekyll-seo-tag plugin support (Open Graph, Twitter Cards, JSON-LD)
- Static file copying (assets, images, and other non-template files)
- Incremental builds (skip unchanged pages)
- Local development server with live reload via WebSocket
- Parallel collection loading (via rayon)
- Duplicate YAML key handling (matches Ruby YAML behavior: last value wins)
- Dynamic include paths and include parameters


## How It Was Built

rustkyll was developed using an agent-driven development process. Three AI agents collaborate through a structured pipeline:

1. Product Manager -- grooms issues by adding acceptance criteria and test scenarios
2. Software Engineer -- implements code and writes tests
3. Tester (QA) -- verifies acceptance criteria, runs tests, and checks output

The project uses a file-based issue tracker in `docs/tracker/`. Each issue is a Markdown file whose filename encodes its status:

- `.todo.md` -- not yet groomed
- `.groomed.md` -- groomed by PM, ready for engineering
- `.in-progress.md` -- engineer is working on it
- `.done.md` -- accepted and committed

The pipeline for each issue follows this flow:

```
PM grooms -> Engineer implements -> Tester verifies -> PM accepts -> committed
```

Issues are processed in batches of two, running in parallel. If the tester finds problems, the issue goes back to the engineer. If the PM rejects, it goes back to the engineer. Only after PM acceptance is the code committed.

Over 40 issues have been completed through this process, covering everything from initial project setup through config parsing, template rendering, collection loading, and cross-site compatibility testing.


## Tested Sites

rustkyll has been tested against multiple real Jekyll sites to verify compatibility.

### DataTalks.Club

The primary reference site. Generates 779 pages and 1455 static files. Six collections: posts, people, books, podcast, courses, conferences.

- datatalksclub.github.io -- full build, 779 pages in ~17 seconds

### alexeygrigorev repositories

Personal Jekyll sites used for cross-site compatibility testing:

- kids-horror-stories-ru -- 1344 pages (1343 posts), builds in ~4 seconds
- alexeygrigorev.github.io -- simple personal site, 16 static files
- snippets -- minimal site, 5 static files
- data-science-interviews -- 24 static files
- mlwiki.org -- minimal wiki site
- little-book-of-metals-ru -- builds after normalize_whitespace filter was added
- aihero -- builds after jekyll-seo-tag support was added

### DataTalksClub repositories

- courses -- course listing site
- docs -- builds after include subdirectory path support was added

### External complex sites

These sites were used to stress-test rustkyll against diverse Jekyll features:

- wtf-html-css (mdo/wtf-html-css) -- single-page site, builds successfully
- hyde (poole/hyde) -- classic Jekyll theme, builds after highlight tag and site.related_posts support
- opensource.guide (github/opensource.guide) -- multi-language documentation, partial build (needs hash integer indexing)
- bitcoin.org (bitcoin/bitcoin.org) -- large site (~270 pages), partial build (needed duplicate YAML key handling)
- academicpages (academicpages/academicpages.github.io) -- academic portfolio, builds after include subdirectory path support
- jekyll-docs (jekyll/jekyll docs/) -- Jekyll's own documentation, builds after missing filter support
- government.github.com (github/government.github.com) -- builds after dynamic include path support
- edition-template (CloudCannon/edition-jekyll-template) -- builds after jekyll-seo-tag support


## Known Limitations

- No Sass/SCSS compilation. Jekyll sites that rely on Sass stylesheets will need to pre-compile their CSS or use plain CSS files.
- No general plugin system. Only jekyll-seo-tag is supported as a built-in. Other plugins (jekyll-paginate, jekyll-redirect-from, jekyll-mentions, etc.) are not available.
- No pagination support. The jekyll-paginate plugin for splitting post listings across multiple pages is not implemented.
- Some edge-case Liquid filters may be missing. While 40+ filters are supported, site-specific or rarely-used filters may not be recognized.
- No Ruby gem theme support. Themes must be present as local layout and include files, not installed as gems.
- Incremental builds do not track layout or include file changes. If you modify a layout or include, use `--force` to trigger a full rebuild.


## Project Structure

```
src/           -- Rust source code
docs/tracker/  -- file-based issue tracker
docs/plan.md   -- project vision and architecture
```


## License

This project is not yet published under a specific license. See the repository for updates.
