# rustkyll

A fast, drop-in replacement for Jekyll, written in Rust.

rustkyll reads the same source files as Jekyll - Markdown with YAML front matter, Liquid templates, YAML data files, collections - and usually produces equivalent HTML output much faster.

```
uvx rustkyll serve
```

Run this in any Jekyll site directory. That's it.

## Benchmarks

Tested on 50+ Jekyll sites. Fresh benchmark run on 2026-04-02. Here are some highlights:

| Site | Pages | Jekyll | rustkyll | Speedup |
|------|------:|-------:|---------:|--------:|
| [DataTalksClub](https://github.com/DataTalksClub/datatalksclub.github.io) | 790 | 19.6s | 0.6s | 31x |
| [opensource.guide](https://github.com/github/opensource.guide) | 390 | 15.8s | 0.6s | 27x |
| [muan/site](https://github.com/muan/site) | 2,219 | 16.2s | 0.6s | 27x |
| [large-docs-site](websites/large-docs-site) | 801 | 23.9s | 0.7s | 34x |
| [large-blog-3000](websites/large-blog-3000) | 3,001 | 4.5s | 1.0s | 4x |
| [al-folio](https://github.com/alshedivat/al-folio) | 102 | 17.7s | 0.4s | 46x |
| [type-theme](https://github.com/rohanchandra/type-theme) | 8 | 2.2s | 0.02s | 104x |
| [academicpages](https://github.com/academicpages/academicpages.github.io) | 45 | 4.5s | 0.09s | 53x |

Median wall-clock time over 3 runs, clean builds, no caching. Full results in [docs/benchmark/results.md](docs/benchmark/results.md).

## Installation

### uv (recommended)

```
uvx rustkyll build --source /path/to/site

# Or install globally
uv tool install rustkyll
```

### pip

```
pip install rustkyll
```

### Install script (Linux / macOS)

```
curl -fsSL https://raw.githubusercontent.com/alexeygrigorev/rustkyll/main/scripts/install.sh | sh
```

Installs the latest release into `~/.local/bin`. Override with environment variables:

```
RUSTKYLL_VERSION=v0.4.5 RUSTKYLL_INSTALL_DIR=~/bin \
  curl -fsSL https://raw.githubusercontent.com/alexeygrigorev/rustkyll/main/scripts/install.sh | sh
```

### Pre-built binaries

Download from [GitHub Releases](https://github.com/alexeygrigorev/rustkyll/releases):

| Platform | Binary |
|----------|--------|
| Linux x86_64 | `rustkyll-linux-amd64` |
| Linux ARM64 | `rustkyll-linux-arm64` |
| macOS Intel | `rustkyll-darwin-amd64` |
| macOS Apple Silicon | `rustkyll-darwin-arm64` |
| Windows x86_64 | `rustkyll-windows-amd64.exe` |
| Windows ARM64 | `rustkyll-windows-arm64.exe` |

### Build from source

```
git clone https://github.com/alexeygrigorev/rustkyll.git
cd rustkyll
cargo build --release
```

## Usage

### `rustkyll build`

```
rustkyll build --source /path/to/site --destination /path/to/output
```

Flags:
- `--source` - path to Jekyll site directory (default: `.`)
- `--destination` - output directory (default: `_site`)
- `--incremental` - only rebuild changed pages
- `--force` - force full rebuild, ignoring incremental manifest

### `rustkyll serve`

```
rustkyll serve --source /path/to/site --port 4000
```

Starts a local dev server with live reload. Flags:
- `--port` - HTTP server port (default: 4000)
- `--livereload` / `--no-livereload` - toggle browser auto-refresh (default: on)
- `--no-browser` - don't auto-open browser

## What's supported

138 of 166 Jekyll features are fully implemented, with 7 more partially implemented. See [docs/jekyll-compatibility.md](docs/jekyll-compatibility.md) for the full matrix.

Core:
- Config parsing, front matter, Markdown (GFM)
- Layouts with inheritance, includes with parameters
- Permalinks (named styles and custom patterns)
- Sass/SCSS compilation, static file copying
- YAML and JSON data files
- Excerpts

Collections:
- Posts, custom collections, pagination
- Categories, tags, `page.previous`/`page.next`
- `site.categories`, `site.tags`, `site.related_posts`, `site.pages`

Liquid:
- All standard tags: `for`, `if`, `unless`, `case`, `capture`, `assign`, `raw`, `comment`, `highlight`, `tablerow`, `cycle`, `increment`/`decrement`
- Jekyll-specific tags: `link`, `post_url`, `seo`, `avatar`, `feed_meta`
- 70+ filters including `where`, `where_exp`, `group_by`, `group_by_exp`, `markdownify`, `slugify`, `jsonify`, `relative_url`, `absolute_url`, `sample`, `cgi_escape`, `uri_escape`, and all Liquid stdlib filters

Plugins (built-in):
- jekyll-seo-tag, jekyll-feed, jekyll-sitemap, jekyll-paginate, jekyll-avatar, jekyll-archives, jekyll-redirect-from

Extras:
- Parallel page generation with rayon
- Live reload via WebSocket
- Progress bar and build timing breakdown
- Lenient template rendering (unknown filters warn instead of failing)

## Known limitations

- No gem-based themes. Themes must be present as local layout/include files.
- No Ruby plugin system. Only the built-in plugin equivalents listed above are supported.
- No CSV/TSV data files. Only YAML and JSON are loaded.
- Incremental builds don't track layout/include changes. Use `--force` after modifying layouts.
- Syntax highlighting classes may differ slightly from Rouge (Jekyll uses Rouge, rustkyll uses syntect).

## How it was built

rustkyll was developed entirely by AI agents - a Product Manager, Software Engineer, and Tester - collaborating through a structured pipeline. See [docs/PROCESS.md](docs/PROCESS.md) for details.

## License

This project is not yet published under a specific license. See the repository for updates.
