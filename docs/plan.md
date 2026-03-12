# rustkyll -- Project Plan

## Vision

Replace Jekyll with a fast Rust static site generator tailored for the DataTalks.Club website. The generator reads the same content files (Markdown with YAML front matter, YAML data files, Liquid-compatible templates) and produces equivalent HTML output.

## Non-Goals

- General-purpose static site generator (we only need to support DataTalks.Club)
- Python scripts reimplementation (Airtable sync, preview generation, etc.)
- Node.js preview system reimplementation
- Perfect Liquid compatibility (only the subset actually used)

## Architecture

```
CLI (main.rs)
  ├── Config loader (_config.yml)
  ├── Content loader
  │   ├── Front matter parser (YAML + Markdown)
  │   ├── Collection loader (_people, _books, _podcast, _posts, etc.)
  │   └── Data loader (_data/*.yaml)
  ├── Template engine
  │   ├── Liquid-subset renderer (for/if/include/assign/capture)
  │   ├── Filters (where, sort, date, markdownify, etc.)
  │   └── Layout chain (layout wrapping)
  ├── Page generator
  │   ├── Collection pages (one HTML per collection item)
  │   ├── Standalone pages (index.md, articles.md, etc.)
  │   └── Blog posts
  ├── Feed generator (RSS/Atom via jekyll-feed equivalent)
  ├── Sitemap generator
  └── Static file copier (assets/, images/)
```

## Content Types

| Collection | Count | Layout | Permalink |
|-----------|-------|--------|-----------|
| `_posts` | ~55 | post | `/blog/:title.html` |
| `_people` | ~427 | author | `/people/:title.html` |
| `_books` | ~99 | book | `/books/:title.html` |
| `_podcast` | ~196 | podcast | `/podcast/:title.html` |
| `_courses` | ~1 | page | `/courses/:title.html` |
| `_conferences` | ~2 | page | `/conferences/:title.html` |
| `_tools` | ~2 | page | `/tools/:title.html` |

## Liquid Features Used

**Tags:** for, if/elsif/else, unless, assign, capture, include, break
**Filters:** where, where_exp, sort, reverse, map, uniq, first, last, size, join, push, slice, append, prepend, default, strip, strip_html, strip_newlines, truncate, slugify, markdownify, newline_to_br, date_to_string, date_to_xmlschema, jsonify, relative_url, split, plus, minus, times, divided_by, modulo

## Issue Dependency Graph

```
01 (project setup)
  ├── 02 (config parsing)
  ├── 03 (front matter + markdown)
  │     └── 05 (collection loader)
  │           ├── 09 (people pages)
  │           ├── 10 (blog posts)
  │           ├── 11 (books pages)
  │           └── 12 (podcast pages)
  ├── 04 (data file loading)
  │     └── 13 (events rendering)
  ├── 06 (template engine core)
  │     ├── 07 (template filters)
  │     │     └── 08 (layout + includes)
  │     │           ├── 09, 10, 11, 12 (collection pages)
  │     │           └── 14 (standalone pages)
  │     └── 08 (layout + includes)
  ├── 15 (static file copying)
  ├── 16 (sitemap)
  ├── 17 (RSS feed)
  ├── 18 (JSON-LD schemas)
  └── 19 (CLI + full build)
        └── 20 (output comparison)
```
