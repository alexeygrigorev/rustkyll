# Jekyll Compatibility Matrix

This document lists every major Jekyll feature and its implementation status in rustkyll.

**Status key:** "yes" = fully implemented, "partial" = implemented with limitations, "no" = not implemented.

## Summary

| Status | Count |
|--------|-------|
| yes    | 125   |
| partial | 8     |
| no     | 28    |
| **Total** | **161** |

---

## Core

| Feature | Jekyll | rustkyll | Notes |
|---------|--------|----------|-------|
| `_config.yml` parsing | yes | yes | `src/config.rs` -- supports all standard keys plus catch-all for extras |
| Multiple config files | yes | no | Jekyll supports `--config a.yml,b.yml`; rustkyll reads only `_config.yml` |
| YAML front matter | yes | yes | `src/frontmatter.rs` -- `---` delimited, parsed to HashMap |
| Front matter defaults | yes | yes | `src/config.rs` -- `defaults:` with scope/values matching by type and path |
| Markdown rendering (kramdown/GFM) | yes | yes | `src/frontmatter.rs` -- uses pulldown-cmark with GFM extensions |
| Layouts | yes | yes | `src/template/layout.rs` -- nested layout inheritance supported |
| Includes | yes | yes | `src/template/include_tag.rs` -- supports parameters, subdirectory paths, dynamic paths |
| Include parameters | yes | yes | `src/template/include_tag.rs` -- `{% include file.html param=value %}` |
| Static file copying | yes | yes | `src/static_files.rs` -- parallel copying with rayon |
| Permalinks | yes | yes | `src/collection.rs` -- named styles (date, pretty, ordinal, none) and custom patterns |
| Permalink placeholders | yes | yes | Supports `:title`, `:year`, `:month`, `:day`, `:categories`, `:collection`, `:short_year`, `:i_month`, `:i_day`, `:path` |
| Exclude files | yes | yes | `src/config.rs` -- `exclude:` list respected during static file copying and page loading |
| Site URL / baseurl | yes | yes | `src/config.rs` and `src/template/filters/` -- `url` and `baseurl` config keys used by `relative_url` and `absolute_url` filters |
| Excerpt support | yes | partial | `src/frontmatter.rs` -- supports `<!--more-->` separator only; custom `excerpt_separator` config not supported |
| Null YAML value handling | yes | yes | `src/config.rs` -- null values default to empty strings |
| Duplicate YAML key handling | yes | yes | `src/yaml.rs` -- last-wins semantics matching Ruby YAML behavior |

## Collections

| Feature | Jekyll | rustkyll | Notes |
|---------|--------|----------|-------|
| Posts (`_posts/`) | yes | yes | `src/collection.rs` -- date extraction from filename, sorted by date |
| Custom collections | yes | yes | `src/collection.rs` -- any collection defined in `_config.yml` `collections:` |
| Collection `output: true/false` | yes | yes | `src/config.rs` -- controls whether pages are generated |
| Collection permalinks | yes | yes | `src/collection.rs` -- per-collection permalink patterns |
| Drafts (`_drafts/`) | yes | no | No `_drafts` directory support or `--drafts` flag |
| Pagination (jekyll-paginate) | yes | no | No paginator object or multi-page post listings |
| Post categories | yes | yes | `src/collection.rs` -- extracted from front matter (`categories` array or `category` string) |
| Post tags | yes | yes | `src/collection.rs` -- extracted from front matter (`tags` array or `tag` string) |
| `site.categories` | yes | yes | `src/generator.rs` -- maps category name to array of posts |
| `site.tags` | yes | yes | `src/generator.rs` -- maps tag name to array of posts |
| `page.previous` / `page.next` | yes | yes | `src/generator.rs` -- chronological prev/next for posts |
| `site.related_posts` | yes | partial | `src/generator.rs` -- returns 10 most recent posts (Jekyll uses LSI by default which is also just recent posts without classifier-reborn) |
| `site.pages` | yes | yes | `src/generator.rs` -- standalone page objects available in templates |
| Post date from filename | yes | yes | `src/collection.rs` -- `YYYY-MM-DD-title.md` pattern |

## Templates (Liquid)

### Tags

| Feature | Jekyll | rustkyll | Notes |
|---------|--------|----------|-------|
| `{% if %}` / `{% elsif %}` / `{% else %}` / `{% endif %}` | yes | yes | Provided by liquid crate stdlib |
| `{% for %}` / `{% endfor %}` | yes | yes | Provided by liquid crate stdlib; includes `forloop` variables |
| `{% assign %}` | yes | yes | Provided by liquid crate stdlib |
| `{% capture %}` / `{% endcapture %}` | yes | yes | Provided by liquid crate stdlib |
| `{% include %}` | yes | yes | `src/template/include_tag.rs` -- custom implementation with parameters and dynamic paths |
| `{% comment %}` / `{% endcomment %}` | yes | yes | Provided by liquid crate stdlib |
| `{% raw %}` / `{% endraw %}` | yes | yes | Provided by liquid crate stdlib |
| `{% highlight %}` / `{% endhighlight %}` | yes | partial | `src/template/highlight_tag.rs` -- outputs `<pre><code>` structure but no syntax coloring; relies on client-side highlighting |
| `{% unless %}` / `{% endunless %}` | yes | yes | Provided by liquid crate stdlib |
| `{% case %}` / `{% when %}` / `{% endcase %}` | yes | yes | Provided by liquid crate stdlib |
| `{% cycle %}` | yes | yes | Provided by liquid crate stdlib |
| `{% tablerow %}` / `{% endtablerow %}` | yes | yes | Provided by liquid crate stdlib |
| `{% increment %}` / `{% decrement %}` | yes | yes | Provided by liquid crate stdlib |
| `{% seo %}` | yes | yes | `src/template/seo_tag.rs` -- generates title, meta description, Open Graph, Twitter Card, JSON-LD, canonical URL |
| `{% avatar %}` | yes | yes | `src/template/avatar_tag.rs` -- generates GitHub avatar `<img>` with srcset |
| `{% link %}` | yes | no | Not implemented |
| `{% post_url %}` | yes | no | Not implemented |

### Filters (Liquid stdlib -- provided by liquid crate)

| Feature | Jekyll | rustkyll | Notes |
|---------|--------|----------|-------|
| `date` | yes | yes | Liquid stdlib |
| `size` | yes | yes | Liquid stdlib |
| `strip_html` | yes | yes | Liquid stdlib |
| `url_encode` | yes | yes | Liquid stdlib |
| `default` | yes | yes | Liquid stdlib |
| `first` | yes | yes | Liquid stdlib |
| `last` | yes | yes | Liquid stdlib |
| `join` | yes | yes | Liquid stdlib |
| `map` | yes | yes | Liquid stdlib |
| `concat` | yes | yes | Liquid stdlib |
| `replace` | yes | yes | Liquid stdlib |
| `split` | yes | yes | Liquid stdlib |
| `strip` | yes | yes | Liquid stdlib |
| `downcase` | yes | yes | Liquid stdlib |
| `upcase` | yes | yes | Liquid stdlib |
| `capitalize` | yes | yes | Liquid stdlib |
| `truncate` | yes | yes | Liquid stdlib |
| `escape` | yes | yes | Liquid stdlib |
| `plus` | yes | yes | Liquid stdlib |
| `minus` | yes | yes | Liquid stdlib |
| `times` | yes | yes | Liquid stdlib |
| `divided_by` | yes | yes | Liquid stdlib |
| `modulo` | yes | yes | Liquid stdlib |
| `append` | yes | yes | Liquid stdlib |
| `prepend` | yes | yes | Liquid stdlib |
| `remove` | yes | yes | Liquid stdlib |
| `remove_first` | yes | yes | Liquid stdlib |
| `replace_first` | yes | yes | Liquid stdlib |
| `sort` | yes | yes | Liquid stdlib |
| `reverse` | yes | yes | Liquid stdlib |
| `uniq` | yes | yes | Liquid stdlib |
| `compact` | yes | yes | Liquid stdlib |
| `where` | yes | yes | `src/template/filters/where_filter.rs` -- custom implementation for Jekyll compatibility |
| `sort_natural` | yes | yes | Liquid stdlib |
| `abs` | yes | yes | Liquid stdlib |
| `ceil` | yes | yes | Liquid stdlib |
| `floor` | yes | yes | Liquid stdlib |
| `round` | yes | yes | Liquid stdlib |
| `at_least` | yes | yes | Liquid stdlib |
| `at_most` | yes | yes | Liquid stdlib |
| `strip_newlines` | yes | yes | Liquid stdlib |
| `lstrip` | yes | yes | Liquid stdlib |
| `rstrip` | yes | yes | Liquid stdlib |

### Filters (Jekyll-specific -- custom implementations)

| Feature | Jekyll | rustkyll | Notes |
|---------|--------|----------|-------|
| `slugify` | yes | yes | `liquid_lib::jekyll::Slugify` |
| `push` | yes | yes | `liquid_lib::jekyll::Push` |
| `array_to_sentence_string` | yes | yes | `liquid_lib::jekyll::ArrayToSentenceString` |
| `jsonify` | yes | yes | `src/template/filters/jsonify.rs` |
| `markdownify` | yes | yes | `src/template/filters/markdownify.rs` |
| `smartify` | yes | no | Not implemented |
| `relative_url` | yes | yes | `src/template/filters/relative_url.rs` -- prepends `site.baseurl` |
| `absolute_url` | yes | yes | `src/template/filters/absolute_url.rs` -- prepends `site.url + site.baseurl` |
| `date_to_string` | yes | yes | `src/template/filters/date_to_string.rs` |
| `date_to_long_string` | yes | yes | `src/template/filters/date_to_long_string.rs` |
| `date_to_xmlschema` | yes | yes | `src/template/filters/date_to_xmlschema.rs` |
| `xml_escape` | yes | yes | `src/template/filters/xml_escape.rs` |
| `where_exp` | yes | yes | `src/template/filters/where_exp.rs` |
| `group_by` | yes | yes | `src/template/filters/group_by.rs` |
| `group_by_exp` | yes | yes | `src/template/filters/group_by_exp.rs` |
| `number_of_words` | yes | yes | `src/template/filters/number_of_words.rs` |
| `truncatewords` | yes | yes | `src/template/filters/truncatewords.rs` |
| `newline_to_br` | yes | yes | `src/template/filters/newline_to_br.rs` |
| `normalize_whitespace` | yes | yes | `src/template/filters/normalize_whitespace.rs` |
| `sort_by` (via where/group_by) | yes | partial | Sort works via Liquid stdlib `sort`; Jekyll's `sort` filter with property argument uses different semantics |
| `pop` | yes | yes | `liquid_lib::jekyll::Pop` |
| `unshift` | yes | yes | `liquid_lib::jekyll::Unshift` |
| Unknown filters | yes | partial | `src/template/engine.rs` -- auto-registers passthrough filters for unrecognized filter names, so templates render but the filter has no effect |

### Template Variables

| Feature | Jekyll | rustkyll | Notes |
|---------|--------|----------|-------|
| `site.*` variables | yes | yes | `src/generator.rs` -- url, baseurl, name, title, time, collections, data, categories, tags, related_posts, pages, plus all extras from config |
| `page.*` variables | yes | yes | `src/generator.rs` -- all front matter keys plus url, content, title, previous, next |
| `content` | yes | yes | `src/template/layout.rs` -- rendered content injected into layouts |
| `paginator.*` | yes | no | No pagination support |
| `forloop` variables | yes | yes | Liquid stdlib -- `forloop.index`, `forloop.first`, `forloop.last`, etc. |
| `site.time` | yes | yes | `src/generator.rs` -- current build timestamp |
| `site.data.*` | yes | yes | `src/generator.rs` -- data from `_data/` directory |
| Lenient variable access | yes | yes | `src/template/engine.rs` -- missing keys return nil instead of erroring |

## Data Files

| Feature | Jekyll | rustkyll | Notes |
|---------|--------|----------|-------|
| YAML data files (`_data/*.yml`) | yes | yes | `src/data.rs` -- supports `.yml` and `.yaml` extensions |
| Nested data directories | yes | yes | `src/data.rs` -- subdirectories become nested mappings |
| JSON data files (`_data/*.json`) | yes | no | Only YAML files are loaded |
| CSV data files (`_data/*.csv`) | yes | no | Only YAML files are loaded |
| TSV data files (`_data/*.tsv`) | yes | no | Only YAML files are loaded |

## Plugins

| Feature | Jekyll | rustkyll | Notes |
|---------|--------|----------|-------|
| jekyll-seo-tag | yes | yes | `src/template/seo_tag.rs` -- built-in implementation |
| jekyll-feed | yes | yes | `src/feed.rs` -- generates Atom feed (feed.xml) from posts |
| jekyll-sitemap | yes | yes | `src/sitemap.rs` -- generates sitemap.xml |
| jekyll-avatar | yes | yes | `src/template/avatar_tag.rs` -- built-in implementation |
| jekyll-redirect-from | yes | no | |
| jekyll-paginate | yes | no | |
| jekyll-mentions | yes | no | |
| jekyll-include-cache | yes | partial | Includes are loaded eagerly; `{% include_cached %}` is not a distinct tag but includes are not re-parsed per render |
| jekyll-relative-links | yes | no | |
| jekyll-optional-front-matter | yes | no | |
| jekyll-titles-from-headings | yes | no | |
| jekyll-default-layout | yes | no | |
| Custom Ruby plugins | yes | no | No plugin system; only built-in plugins are supported |
| Gem-based themes | yes | no | Themes must be present as local layout/include files |

## Assets

| Feature | Jekyll | rustkyll | Notes |
|---------|--------|----------|-------|
| Sass/SCSS compilation | yes | no | Pre-compile CSS as a workaround |
| CoffeeScript compilation | yes | no | |
| Static asset copying | yes | yes | `src/static_files.rs` -- all non-underscored, non-excluded files are copied |

## CLI

| Feature | Jekyll | rustkyll | Notes |
|---------|--------|----------|-------|
| `build` command | yes | yes | `src/main.rs` -- `--source`, `--destination`, `--incremental`, `--force` flags |
| `serve` command | yes | yes | `src/main.rs` -- `--port`, `--livereload`, `--no-livereload` flags |
| `new` command | yes | no | |
| `doctor` command | yes | no | |
| `clean` command | yes | no | `build` performs a full clean by default |
| `--watch` flag | yes | partial | Built into `serve` command (file watcher triggers rebuild); not available as standalone `build --watch` |
| `--drafts` flag | yes | no | |
| `--config` flag | yes | no | Always reads `_config.yml` from the source directory |
| `--verbose` / `--quiet` flags | yes | no | |
| `--safe` mode | yes | no | |
| `--version` flag | yes | yes | `src/main.rs` -- `clap` provides `--version` |
| `--help` flag | yes | yes | `src/main.rs` -- `clap` provides `--help` |

## Other

| Feature | Jekyll | rustkyll | Notes |
|---------|--------|----------|-------|
| Incremental builds | yes | partial | `src/incremental.rs` -- tracks file modification times; does not detect layout/include dependency changes |
| Live reload | yes | yes | `src/livereload.rs` -- WebSocket-based reload on file changes |
| JSON-LD structured data | yes | yes | `src/jsonld.rs` -- Book and BreadcrumbList schemas |
| Parallel page generation | no | yes | `src/main.rs` -- uses rayon for parallel collection loading, page generation, and static file copying |
| Build timing breakdown | no | yes | `src/main.rs` -- per-phase timing output |
| Lenient template rendering | no | yes | `src/template/engine.rs` -- unknown filters become passthroughs instead of errors |
