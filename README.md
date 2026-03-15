# rustkyll

A static site generator written in Rust, designed as a drop-in replacement for Jekyll. The primary goal is to speed up site builds for the DataTalks.Club website (datatalksclub.github.io), which has hundreds of pages across multiple collections.

rustkyll reads the same source files as Jekyll - Markdown with YAML front matter, Liquid templates, YAML data files, and collection directories - and produces equivalent HTML output.

Quickstart (recommended):

```
uvx rustkyll build
uvx rustkyll serve
```

Or download a binary from [GitHub Releases](https://github.com/alexeygrigorev/rustkyll/releases), rename it to `rustkyll` (or `rustkyll.exe` on Windows), and put it in your PATH (e.g. `~/bin`).

## Installation

### Install with uv (recommended)

The fastest way to install rustkyll is with [uv](https://docs.astral.sh/uv/):

```
# Run without installing
uvx rustkyll build --source /path/to/site

# Or install as a global tool
uv tool install rustkyll
rustkyll build --source /path/to/site
```

You can also install with pip:

```
pip install rustkyll
```

### Pre-built binaries

Download the latest release for your platform from the [GitHub Releases](https://github.com/alexeygrigorev/rustkyll/releases) page.

Available binaries:

| Platform | Binary |
|----------|--------|
| Linux x86_64 | `rustkyll-linux-amd64` |
| Linux ARM64 | `rustkyll-linux-arm64` |
| macOS Intel | `rustkyll-darwin-amd64` |
| macOS Apple Silicon | `rustkyll-darwin-arm64` |
| Windows x86_64 | `rustkyll-windows-amd64.exe` |
| Windows ARM64 | `rustkyll-windows-arm64.exe` |

On Linux and macOS, make the binary executable after downloading:

```
chmod +x rustkyll-*
```

### Build from source

Prerequisites:

- Rust toolchain (stable, 2021 edition or later)

Clone and build:

```
git clone https://github.com/alexeygrigorev/rustkyll.git
cd rustkyll
cargo build --release
```

The binary will be at `target/release/rustkyll`.

Alternatively, install directly with cargo:

```
cargo install --path .
```


## Usage

### build

Generate the static site from source files:

```
rustkyll build --source /path/to/site --destination /path/to/output
```

For development, build and run from source:

```
cargo run --release -- build --source /path/to/site
cargo run --release -- serve --source /path/to/site
```

The `--release` flag is important for performance — debug builds are 5-10x slower.

Flags:

- `--source` - path to the Jekyll site directory (default: current directory)
- `--destination` - output directory for generated files (default: `_site`)
- `--incremental` - only rebuild pages whose source files have changed
- `--force` - force a full rebuild, ignoring the incremental manifest

### serve

Build and serve the site locally with a development server:

```
rustkyll serve --source /path/to/site --port 4000
```

Flags:

- `--source` - path to the Jekyll site directory (default: current directory)
- `--destination` - output directory (default: `_site`)
- `--port` - port number for the HTTP server (default: 4000)
- `--livereload` - enable live reload in the browser when files change (default: enabled)
- `--no-livereload` - disable live reload


## How It Was Built

rustkyll was developed using an agent-driven development process. Three AI agents collaborate through a structured pipeline:

1. Product Manager - grooms issues by adding acceptance criteria and test scenarios
2. Software Engineer - implements code and writes tests
3. Tester (QA) - verifies acceptance criteria, runs tests, and checks output

The project uses a file-based issue tracker in `docs/tracker/`. Each issue is a Markdown file whose filename encodes its status:

- `.todo.md` - not yet groomed
- `.groomed.md` - groomed by PM, ready for engineering
- `.in-progress.md` - engineer is working on it
- `.done.md` - accepted and committed

The pipeline for each issue follows this flow:

```
PM grooms -> Engineer implements -> Tester verifies -> PM accepts -> committed
```

Issues are processed in batches of two, running in parallel. If the tester finds problems, the issue goes back to the engineer. If the PM rejects, it goes back to the engineer. Only after PM acceptance is the code committed.



## Tested sites

| Site | Pages | Jekyll | rustkyll | Speedup |
|------|-------|--------|----------|---------|
| [datatalksclub.github.io](https://github.com/DataTalksClub/datatalksclub.github.io) | 787 | 19.1s | 1.0s | 19x |
| [kids-horror-stories-ru](https://github.com/alexeygrigorev/kids-horror-stories-ru) | 1345 | 3.8s | 0.4s | 9.5x |
| [muan-blog](https://github.com/muan/site) | 2218 | 15.9s | 0.4s | 40x |
| [large-docs-site](websites/large-docs-site) | 801 | 23.4s | 0.3s | 78x |
| [large-blog-3000](websites/large-blog-3000) | 3001 | 4.3s | 1.4s | 3x |

22 of 43 sites build with both tools. See [docs/benchmark/results.md](docs/benchmark/results.md) for full results including structural equivalence and visual comparison.

Other tested sites

- [alexeygrigorev.github.io](https://github.com/alexeygrigorev/alexeygrigorev.github.io)
- [snippets](https://github.com/alexeygrigorev/snippets)
- [data-science-interviews](https://github.com/alexeygrigorev/data-science-interviews)
- [mlwiki.org](https://github.com/alexeygrigorev/mlwiki.org)
- [little-book-of-metals-ru](https://github.com/alexeygrigorev/little-book-of-metals-ru)
- [aihero](https://github.com/alexeygrigorev/aihero)
- [DataTalksClub/courses](https://github.com/DataTalksClub/courses)
- [DataTalksClub/docs](https://github.com/DataTalksClub/docs)
- [wtf-html-css](https://github.com/mdo/wtf-html-css)
- [hyde](https://github.com/poole/hyde)
- [opensource.guide](https://github.com/github/opensource.guide)
- [bitcoin.org](https://github.com/bitcoin/bitcoin.org)
- [government.github.com](https://github.com/github/government.github.com)
- [edition-template](https://github.com/CloudCannon/edition-jekyll-template)
- [beautiful-jekyll](https://github.com/daattali/beautiful-jekyll)


## Jekyll Compatibility

See [docs/jekyll-compatibility.md](docs/jekyll-compatibility.md) for a detailed feature-by-feature comparison between rustkyll and Jekyll.

## Known Limitations

- No Sass/SCSS compilation. Jekyll sites that rely on Sass stylesheets will need to pre-compile their CSS or use plain CSS files.
- No general plugin system. Only jekyll-seo-tag is supported as a built-in. Other plugins (jekyll-paginate, jekyll-redirect-from, jekyll-mentions, etc.) are not available.
- No pagination support. The jekyll-paginate plugin for splitting post listings across multiple pages is not implemented.
- Some edge-case Liquid filters may be missing. While 40+ filters are supported, site-specific or rarely-used filters may not be recognized.
- No Ruby gem theme support. Themes must be present as local layout and include files, not installed as gems.
- Incremental builds do not track layout or include file changes. If you modify a layout or include, use `--force` to trigger a full rebuild.


## Project Structure

```
src/           - Rust source code
docs/tracker/  - file-based issue tracker
docs/plan.md   - project vision and architecture
```


## License

This project is not yet published under a specific license. See the repository for updates.
