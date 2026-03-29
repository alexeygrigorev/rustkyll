# Porting Your Jekyll Site to Rustkyll

Rustkyll is a drop-in replacement for Jekyll, written in Rust. Most Jekyll sites
build with rustkyll without any changes. This guide covers what works
out of the box, what needs minor adaptation, and how to handle custom plugins.

---

## 1. Quick Start

### Step 1: Install rustkyll

Install via pip (includes pre-built binaries for Linux, macOS, and Windows):

```bash
pip install rustkyll
```

Or build from source:

```bash
git clone https://github.com/DataTalksClub/rustkyll.git
cd rustkyll
cargo build --release
# Binary is at target/release/rustkyll
```

### Step 2: Build your site

Run rustkyll in your Jekyll site directory (where `_config.yml` lives):

```bash
rustkyll build --source /path/to/your-site --destination /path/to/output
```

Or from within the site directory:

```bash
cd your-site
rustkyll build
```

The default source is `.` and the default destination is `_site`, matching Jekyll.

### Step 3: Serve locally

```bash
rustkyll serve --source /path/to/your-site --port 4000
```

This starts a local server with live reload. Edit files and see changes
instantly.

### Step 4: Verify output

Compare rustkyll output against Jekyll output using the DOM comparison tool:

```bash
# Build with Jekyll first
cd your-site && bundle exec jekyll build -d _site_jekyll

# Build with rustkyll
rustkyll build --source your-site --destination _site_rustkyll

# Compare
uv run python scripts/dom_compare.py \
  --jekyll-dir _site_jekyll \
  --rustkyll-dir _site_rustkyll
```

The DOM comparison normalizes whitespace, sorts attributes, and performs a
structural comparison. It reports the number of matching pages out of the total.
A perfect score (e.g., 790/790) means the output is structurally identical.

### Most sites just work

Rustkyll has been tested against 35+ real-world Jekyll sites. If your site uses
standard Jekyll features (Liquid templates, Markdown, SASS, collections, data
files, front matter defaults), it should build correctly without changes.

---

## 2. Theme Support

### Local/Custom Themes (works as-is)

If your site has its own `_layouts/` and `_includes/` directories, everything
works exactly as in Jekyll. Rustkyll loads layouts and includes from these
directories and renders them with the Liquid template engine.

```
your-site/
  _layouts/
    default.html
    post.html
  _includes/
    header.html
    footer.html
  _config.yml
  index.html
```

No changes needed. Rustkyll resolves layouts by name, supports layout
inheritance (via `layout:` in a layout's front matter), and processes includes
with parameters.

### GitHub Pages Themes (fully supported)

All 10 official GitHub Pages themes are fully supported and tested:

| Theme | Gem Name | Status |
|-------|----------|--------|
| Architect | `jekyll-theme-architect` | Full match |
| Cayman | `jekyll-theme-cayman` | Full match |
| Dinky | `jekyll-theme-dinky` | Full match |
| Hacker | `jekyll-theme-hacker` | Full match |
| Leap Day | `jekyll-theme-leap-day` | Full match |
| Merlot | `jekyll-theme-merlot` | Full match |
| Midnight | `jekyll-theme-midnight` | Full match |
| Primer | `jekyll-theme-primer` | Full match |
| Slate | `jekyll-theme-slate` | Full match |
| Time Machine | `jekyll-theme-time-machine` | Full match |

These themes are distributed as Ruby gems, but rustkyll does not need the gems
installed. When your `_config.yml` declares `theme: jekyll-theme-cayman` (or any
GitHub Pages theme), rustkyll resolves the theme's layouts and includes from its
own bundled copies. Your site just needs `_config.yml` -- no Gemfile or
`bundle install` required.

### Popular Community Themes (tested)

These widely-used community themes have been tested and work with rustkyll:

| Theme | GitHub Stars | Status |
|-------|-------------|--------|
| minimal-mistakes | ~13,000 | Supported |
| academicpages | ~12,000 | Supported |
| just-the-docs | ~7,500 | Supported |
| beautiful-jekyll | ~5,500 | Supported |
| documentation-theme-jekyll | ~4,300 | Supported |
| minima | ~3,500 | Supported |
| hyde | ~3,500 | Supported |
| chirpy | ~7,500 | Supported |
| lanyon | ~3,200 | Supported |
| so-simple | ~1,800 | Supported |

If you use one of these themes, your site should work with rustkyll. Copy the
theme's `_layouts/`, `_includes/`, and `_sass/` directories into your site (if
they are not already present) so rustkyll can find them.

### Gem-Based Themes

Jekyll gem-based themes package layouts, includes, and assets inside a Ruby gem.
Since rustkyll does not use Ruby, it cannot load gems directly. To use a
gem-based theme:

1. **For GitHub Pages themes**: no action needed -- rustkyll handles these
   natively (see above).

2. **For other gem-based themes** (minima, just-the-docs, etc.): copy the
   theme's `_layouts/`, `_includes/`, `_sass/`, and `assets/` directories into
   your site root. You can find these files by running:

   ```bash
   # Find where the gem is installed
   bundle info --path minima

   # Copy the theme files into your site
   cp -r $(bundle info --path minima)/_layouts .
   cp -r $(bundle info --path minima)/_includes .
   cp -r $(bundle info --path minima)/_sass .
   cp -r $(bundle info --path minima)/assets .
   ```

   After copying, rustkyll finds and uses these files just like Jekyll does.

### Remote Themes (`jekyll-remote-theme`)

The `jekyll-remote-theme` plugin downloads a theme from GitHub at build time.
Rustkyll does not support this plugin directly. To port a site using remote
themes:

1. Identify the remote theme repository from your `_config.yml`:
   ```yaml
   remote_theme: owner/repo
   ```

2. Clone or download that repository.

3. Copy its `_layouts/`, `_includes/`, `_sass/`, and `assets/` into your site.

4. Remove the `remote_theme` line from `_config.yml` (optional; rustkyll ignores
   it, but removing it keeps the config clean).

---

## 3. Plugin Compatibility

### Fully Supported Plugins

These Jekyll plugins are natively implemented in rustkyll. They work
automatically when your `_config.yml` references them -- no gem installation
needed.

| Plugin | Implementation | Notes |
|--------|---------------|-------|
| `jekyll-seo-tag` | Native `{% seo %}` tag | Full support: Open Graph, Twitter Cards, JSON-LD structured data, canonical URLs. Supports `{% seo title=false %}`. |
| `jekyll-feed` | Native `{% feed_meta %}` tag + Atom feed generator | Generates `feed.xml` with configurable post count. Supports `site.feed.categories` for per-category feeds. |
| `jekyll-sitemap` | Native sitemap generator | Generates `sitemap.xml` with all pages and collection items. |
| `jekyll-paginate` | Native pagination engine | Full paginator support: `paginator.posts`, `paginator.page`, `paginator.total_pages`, `paginator.previous_page_path`, `paginator.next_page_path`, etc. Configurable via `paginate` and `paginate_path` in `_config.yml`. |
| `jekyll-github-metadata` | Native `{% github_edit_link %}` tag | Produces edit-on-GitHub links when `site.github.repository_url` is set. The `site.github` namespace is populated from `_config.yml`. |
| `jekyll-avatar` | Native `{% avatar %}` tag | Generates GitHub avatar `<img>` tags with `srcset` for 1x-4x resolution. Supports `{% avatar USERNAME %}`, `{% avatar user=variable %}`, and `{% avatar user=variable size=N %}`. |
| `jekyll-redirect-from` | Native redirect page generation | Reads `redirect_from` front matter (single string or array) and generates HTML redirect pages. |
| `jekyll-archives` | Native archive page generator | Generates per-category and per-tag archive pages. Configured via the `jekyll-archives` key in `_config.yml`. |

### Liquid Filters

Rustkyll implements all standard Jekyll Liquid filters plus several extras
commonly used by themes:

| Filter | Notes |
|--------|-------|
| `absolute_url` | Prepends `site.url` + `site.baseurl` |
| `relative_url` | Prepends `site.baseurl` |
| `date`, `date_to_string`, `date_to_long_string` | Full date formatting with timezone support |
| `date_to_rfc822`, `date_to_xmlschema` | RFC 822 and ISO 8601 date formats |
| `markdownify` | Converts Markdown to HTML |
| `jsonify` | Converts values to JSON |
| `where`, `where_exp` | Collection filtering with expression support |
| `group_by`, `group_by_exp` | Collection grouping |
| `sort` | Stable sort matching Jekyll ordering |
| `sample` | Random sample from array |
| `xml_escape`, `cgi_escape`, `uri_escape`, `url_encode` | Various encoding filters |
| `strip_html`, `strip_index` | HTML stripping, index.html removal |
| `normalize_whitespace`, `newline_to_br` | Whitespace handling |
| `number_of_words`, `truncatewords` | Text length operations |
| `map`, `compact`, `uniq`, `join` | Array operations |
| `capitalizeall` | Capitalize every word (used by Jasper2 theme) |

All standard Liquid filters (`upcase`, `downcase`, `capitalize`, `append`,
`prepend`, `replace`, `split`, `size`, `first`, `last`, `slice`, `plus`,
`minus`, `times`, `divided_by`, `modulo`, `floor`, `ceil`, `round`, `abs`,
`default`, etc.) are also available via the underlying Liquid engine.

### Syntax Highlighting

Rustkyll uses [syntect](https://github.com/trishume/syntect) for syntax
highlighting instead of Jekyll's Rouge. The `{% highlight %}` tag and fenced code
blocks both produce Rouge-compatible CSS class names, so existing `syntax.css` or
Rouge-generated stylesheets work without modification.

Supported: `{% highlight lang %}...{% endhighlight %}` blocks and fenced code
blocks in Markdown. The optional `linenos` parameter is accepted (for
compatibility) but currently ignored.

### Not Directly Supported

| Plugin | Workaround |
|--------|-----------|
| Custom Ruby generators | Use the YAML page generator (see Section 4) |
| Custom Ruby tags | If the tag produces simple output, it may be possible to replace with an include or Liquid logic. File an issue if you need a specific tag. |
| Custom Ruby filters | Check if rustkyll already implements the filter natively (many common custom filters are built in). File an issue for missing ones. |
| `jekyll-gist` | Replace `{% gist ID %}` with a `<script>` embed tag in your templates. |
| `jekyll-coffeescript` | Pre-compile CoffeeScript to JavaScript before building. |

---

## 4. Porting Custom Plugins

### The Problem

Many Jekyll sites use custom Ruby plugins -- generators that create pages
programmatically, tags that produce custom HTML, or filters that transform data.
Since rustkyll does not run Ruby, these plugins need alternatives.

### YAML Page Generator (for custom generators)

Most custom Jekyll generators follow a simple pattern: "for each item in a data
source, create a page from a template." Rustkyll provides a declarative YAML
configuration that replaces this pattern without any Ruby code.

Add a `generators` key to your `_config.yml` (or create `_generators.yml`):

```yaml
generators:
  # Create a page for each item in a data file
  - name: wallet_pages
    for_each: _data/wallets.yml
    variable: wallet
    template: _templates/wallet.html
    output: "wallets/{wallet.id}/index.html"

  # Create archive pages per tag
  - name: tag_pages
    for_each: site.tags
    variable: tag
    template: _layouts/tag.html
    output: "tags/{tag.name}/index.html"
```

Each generator rule specifies:
- **`for_each`**: the data source (a YAML/JSON file, a glob pattern, or a site
  collection like `site.tags`)
- **`variable`**: the name to expose each item as in templates
- **`template`**: the template file to render for each item
- **`output`**: the output path pattern with `{variable.field}` placeholders

### Case Study: bitcoin-org's TranslatePageGenerator

The bitcoin.org Jekyll site uses a custom Ruby generator that creates translated
pages. It iterates over translation files and template files, producing ~870
pages (29 templates x 30 languages).

**Before (Ruby plugin -- `_plugins/translate_page_generator.rb`):**

```ruby
class TranslatePageGenerator < Jekyll::Generator
  def generate(site)
    Dir.foreach('_translations') do |file|
      next if file == '.' || file == '..'
      lang = file.sub('.yml', '')
      translations = YAML.load_file("_translations/#{file}")

      Dir.foreach('_templates') do |template|
        next if template == '.' || template == '..'
        site.pages << TranslatePage.new(site, lang, template, translations)
      end
    end
  end
end
```

This plugin cannot run in rustkyll because it is Ruby code. But the pattern is
straightforward: for each translation file, for each template, generate a page.

**After (YAML generator config in `_config.yml`):**

```yaml
generators:
  - name: translated_pages
    for_each: _translations/*.yml    # iterate over YAML files in _translations/
    variable: translation            # expose each file's data as {{ translation }}
    nested_for_each: _templates/*.html  # for each translation, iterate templates
    nested_variable: template        # expose each template as {{ template }}
    output: "{translation.id}/{template.url}"  # output path pattern
```

**Step-by-step walkthrough:**

1. **Identify the data source.** The Ruby plugin reads files from
   `_translations/`. In the YAML config, this becomes
   `for_each: _translations/*.yml`.

2. **Identify the iteration variable.** The Ruby plugin uses `lang` and
   `translations`. In the YAML config, each YAML file's contents are exposed as
   `{{ translation }}`, with `translation.id` set to the filename stem (e.g.,
   `en`, `es`, `fr`).

3. **Identify the template.** The Ruby plugin iterates `_templates/*.html`. In
   the YAML config, this becomes `nested_for_each: _templates/*.html` with each
   template exposed as `{{ template }}`.

4. **Identify the output path.** The Ruby plugin constructs paths like
   `en/about.html`. In the YAML config, this becomes
   `output: "{translation.id}/{template.url}"`.

5. **Remove the Ruby plugin.** Delete or move the `_plugins/` directory entry.
   Rustkyll reads the YAML generator config and produces the same pages.

6. **Verify.** Build with both Jekyll and rustkyll, then run the DOM comparison
   to confirm the output matches.

This approach works for any generator that follows the "data x template = pages"
pattern, which covers the vast majority of custom Jekyll generators.

---

## 5. Known Differences

### Syntax Highlighting Token Classes

Rustkyll uses syntect (TextMate grammars) while Jekyll uses Rouge (its own
grammar definitions). Rustkyll maps syntect scopes to Rouge/Pygments CSS class
names, so existing stylesheets work. However, there can be minor differences in
token classification for edge cases:

- Some tokens may receive a slightly different CSS class (e.g., a Ruby method
  call classified as `nf` in Rouge but `n` in syntect). Rustkyll includes
  language-specific overrides for common cases (Ruby, Python, JavaScript, etc.)
  but rare tokens may differ.
- The visual appearance is usually identical because most syntax CSS themes style
  similar token types with similar colors.

**Fix:** If you see highlighting differences, check your `syntax.css` and ensure
the relevant CSS classes are styled. You can also file an issue with a specific
code snippet that renders differently.

### Smart Quotes and Typography

Jekyll's kramdown processor converts straight quotes to typographic ("smart")
quotes by default. Rustkyll's Markdown parser (pulldown-cmark) also handles
smart quotes, but there can be edge-case differences in how mixed content
(HTML + Markdown) is processed.

**Most common difference:** In code blocks or inline code, smart quote
conversion should not apply -- both Jekyll and rustkyll handle this correctly.
Differences are rare and typically only appear in unusual nesting of HTML and
Markdown.

### Date and Timezone Handling

Rustkyll handles dates and timezones, but there are edge cases to watch for:

- **YAML sexagesimal notation:** YAML interprets bare values like `12:30` as
  sexagesimal numbers (750), not time strings. Rustkyll handles this correctly,
  but if your dates look wrong, check that date values in front matter are
  quoted: `date: "2024-01-15 12:30:00 +0000"`.

- **Timezone defaults:** When no timezone is specified, rustkyll uses the system
  timezone (matching Jekyll behavior). Set `timezone` in `_config.yml` to be
  explicit:
  ```yaml
  timezone: America/New_York
  ```

- **Naive vs. offset-aware dates:** Dates without timezone offsets (e.g.,
  `2024-01-15`) are treated as naive dates in the site's configured timezone.
  This matches Jekyll's behavior.

### SASS/SCSS Compilation

Rustkyll uses [grass](https://github.com/connorskees/grass) (a pure-Rust Sass
compiler) instead of Jekyll's sassc/libsass. Grass is highly compatible, but:

- **Deprecated Sass features:** Grass follows the modern Sass specification
  more strictly. If your SCSS uses very old or deprecated syntax (e.g., `/` for
  division instead of `math.div()`), you may see warnings or different output.

- **`@import` resolution:** Rustkyll strips `.scss` and `.sass` extensions from
  `@import` statements and searches the `_sass/` directory (or the directory
  specified by `sass.sass_dir` in `_config.yml`). Additional search paths can
  be configured via `sass.load_paths`.

- **Output style:** Configure via `_config.yml`:
  ```yaml
  sass:
    sass_dir: _sass
    style: compressed   # or "expanded"
  ```

### HTML Entity Encoding

Minor differences in HTML entity encoding (e.g., `&amp;` vs `&#38;`) may appear
in the output. These are semantically identical and do not affect browser
rendering. The DOM comparison tool normalizes these differences.

---

## 6. Troubleshooting

### "My layout is not being applied"

**Symptoms:** Pages render without any layout wrapping (just raw content).

**Causes and fixes:**

1. **Missing `_layouts/` directory.** If you are using a gem-based theme, the
   layouts are inside the gem. Copy them into your site (see Section 2, "Gem-Based
   Themes").

2. **Layout name mismatch.** Check that the `layout` value in your page's front
   matter matches a file in `_layouts/`. Jekyll is case-sensitive:
   `layout: Post` looks for `_layouts/Post.html`, not `_layouts/post.html`.

3. **Missing front matter defaults.** If you rely on `defaults` in
   `_config.yml` to set layouts, verify the scope matches:
   ```yaml
   defaults:
     - scope:
         path: ""
         type: "posts"
       values:
         layout: "post"
   ```

### "Pages are missing from the output"

**Symptoms:** Some pages that Jekyll generates are not in the rustkyll output.

**Causes and fixes:**

1. **Custom generator plugin.** If your site uses a Ruby generator plugin in
   `_plugins/`, those pages will not be generated. See Section 4 for how to
   replace custom generators with YAML configuration.

2. **Collection not configured for output.** Check that your collection has
   `output: true` in `_config.yml`:
   ```yaml
   collections:
     my_collection:
       output: true
       permalink: /:collection/:name/
   ```

3. **File excluded.** Check the `exclude` list in `_config.yml`. Rustkyll
   respects the same exclusion rules as Jekyll.

### "Syntax highlighting looks different"

**Symptoms:** Code blocks have different colors or styling compared to Jekyll.

**Fixes:**

1. Make sure your CSS file includes styles for Rouge/Pygments token classes.
   Rustkyll maps syntect tokens to the same CSS classes that Rouge uses.

2. If specific tokens look wrong, the issue is likely a scope mapping edge case.
   File an issue with the specific language and code snippet.

3. You can generate a fresh Rouge-compatible CSS file and use it with rustkyll:
   ```bash
   # With Jekyll/Rouge installed:
   rougify style monokai > syntax.css
   ```

### "Build seems slow"

**Symptoms:** Rustkyll build takes longer than expected.

**Causes and fixes:**

1. **Large SASS files.** Complex SCSS with many imports can be slow to compile.
   Ensure you are using `style: compressed` in production (this is the default).

2. **Many pages.** Rustkyll uses parallel rendering (via rayon) by default.
   Build times scale well to thousands of pages. If you have 3000+ pages and it
   feels slow, check that you are using a release build:
   ```bash
   cargo build --release
   ```

3. **Incremental builds.** For development, use incremental mode to only rebuild
   changed pages:
   ```bash
   rustkyll build --incremental
   ```

### "SASS compilation fails"

**Symptoms:** Error messages about SCSS syntax or missing imports.

**Fixes:**

1. **Check `sass_dir`.** Verify your `_config.yml` points to the right
   directory:
   ```yaml
   sass:
     sass_dir: _sass
   ```

2. **Check load paths.** If your SCSS imports files from multiple directories:
   ```yaml
   sass:
     load_paths:
       - _sass
       - node_modules
   ```

3. **Deprecated Sass syntax.** If grass rejects your SCSS, check for deprecated
   features like `/` division. Modern Sass uses `math.div()` instead.

### "Liquid template errors"

**Symptoms:** Build fails with template parsing or rendering errors.

**Fixes:**

1. **Undefined variables.** Rustkyll follows Liquid's strict mode for some
   operations. Use the `default` filter to handle missing values:
   ```liquid
   {{ page.custom_field | default: "" }}
   ```

2. **Custom Liquid tags.** If your templates use custom tags from a Ruby plugin,
   those tags will not be recognized. Check if rustkyll implements the tag
   natively (see the plugin table in Section 3). If not, replace the tag with
   equivalent Liquid logic or an include.

3. **Include file not found.** Verify the file exists in `_includes/` and that
   the name matches exactly (including extension).

### "Redirects are not generated"

**Symptoms:** Pages with `redirect_from` front matter do not produce redirect
HTML files.

**Fix:** Make sure `jekyll-redirect-from` is listed in your `_config.yml`
plugins (even though rustkyll does not use the gem, it checks the plugin list to
decide whether to generate redirects):

```yaml
plugins:
  - jekyll-redirect-from
```

The `redirect_from` front matter supports both single values and arrays:

```yaml
---
redirect_from:
  - /old-url/
  - /another-old-url/
---
```

---

## Appendix: Verified Test Sites

Rustkyll has been tested against these real-world Jekyll sites with DOM
comparison:

| Site | Type | Notes |
|------|------|-------|
| DataTalks.Club | Custom theme, complex collections | 790/790 pages match |
| bitcoin.org | Custom generators, translations | Requires YAML page generator for full support |
| Jekyll docs | jekyll-docs theme | Official Jekyll documentation |
| Choose a License | Custom | GitHub's license picker |
| Homebrew | Custom | Package manager website |
| US Web Design System | Custom | Government design system |
| Open Source Guide | Custom with i18n | GitHub's open source guide |
| Programming Historian | Custom | Academic digital humanities |
| minimal-mistakes | Popular community theme | Most-used Jekyll theme |
| academicpages | Fork of minimal-mistakes | Academic personal sites |
| just-the-docs | Documentation theme | Technical documentation |
| beautiful-jekyll | Blog theme | Personal blogs |
| documentation-theme-jekyll | Docs theme | Technical documentation |
| All 10 GitHub Pages themes | Official themes | Full DOM match |
