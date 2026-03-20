# Theme Support in rustkyll

This guide explains how to use Jekyll themes with rustkyll, including step-by-step instructions, worked examples with real build output, and a table of known blockers.

## How Themes Work in Jekyll

Jekyll supports two types of themes:

1. **Gem-based themes** -- Installed via Ruby's `gem` package manager. The theme's layouts, includes, SASS, and assets are bundled inside a Ruby gem and automatically resolved at build time. The site's `_config.yml` declares `theme: theme-name` and Jekyll finds the files inside the installed gem.

2. **File-based themes** -- The theme files (`_layouts/`, `_includes/`, `_sass/`, `assets/`) live directly in the site directory. No gem installation is needed. Jekyll uses these files directly.

There is also a hybrid approach used by GitHub Pages: `remote_theme: user/repo` fetches a theme from a GitHub repository at build time, without requiring a local gem install.

### Why gem-based themes do not work with rustkyll

rustkyll is a Rust binary. It cannot install Ruby gems, run Bundler, or resolve gem paths. When a site's `_config.yml` says `theme: jekyll-theme-chirpy`, rustkyll has no way to find the layouts and includes that live inside that gem.

The solution is to convert any gem-based theme to a file-based theme by cloning the theme repository and using its files directly.

## Using a Theme with rustkyll

### Step 1: Clone the theme repository

Every gem-based Jekyll theme has a source repository on GitHub. Clone it:

```bash
git clone https://github.com/crmne/jekyll-vitepress-theme websites/jekyll-vitepress-theme
```

The cloned repository already contains `_layouts/`, `_includes/`, `_sass/`, `assets/`, and often example content.

### Step 2: Inspect the theme structure

Check what the theme provides:

```bash
ls websites/jekyll-vitepress-theme/
# Expected: _layouts/ _includes/ _config.yml assets/ ...
```

Themes typically have these directories:
- `_layouts/` -- HTML layout templates (e.g., `default.html`, `post.html`)
- `_includes/` -- Partial HTML fragments included by layouts
- `_sass/` -- SCSS/SASS stylesheets (rustkyll does NOT compile these; see Troubleshooting)
- `assets/` -- Static files (CSS, JS, images)
- `_data/` -- Data files (YAML, JSON)
- `_config.yml` -- Site configuration

### Step 3: Edit `_config.yml`

Open `_config.yml` and make these changes:

1. **Remove or comment out the `theme:` line:**

```yaml
# BEFORE:
theme: jekyll-vitepress-theme

# AFTER:
# theme: jekyll-vitepress-theme
```

2. **Remove or comment out `remote_theme:` if present:**

```yaml
# remote_theme: user/repo
```

3. **Remove unsupported plugins from the `plugins:` list.** rustkyll does not support Ruby plugins. Remove any that are not built-in:

```yaml
# BEFORE:
plugins:
  - jekyll-vitepress-theme
  - jekyll-redirect-from

# AFTER:
plugins: []
```

4. **Check `includes_dir:`** -- Some themes override the includes directory. Make sure the path exists relative to the site root.

### Step 4: Build with rustkyll

```bash
rustkyll build --source websites/jekyll-vitepress-theme --destination /tmp/theme-test
```

Or if running from the rustkyll source tree:

```bash
cargo run -- build --source websites/jekyll-vitepress-theme --destination /tmp/theme-test
```

### Step 5: Review the output

Check for:
- **Warnings in build output** -- rustkyll prints warnings for pages that fail to render
- **Missing CSS** -- If the theme uses SASS, the compiled CSS will be missing (see Troubleshooting)
- **Broken pages** -- Open the generated HTML files in a browser and check for layout issues

### Step 6: For a separate site using a theme

If you have your own content and want to use a cloned theme:

1. Clone the theme repo
2. Copy your content files (`_posts/`, `_pages/`, `index.md`, etc.) into the theme directory
3. Or copy the theme's `_layouts/`, `_includes/`, and `assets/` into your site directory
4. Edit `_config.yml` as described in Step 3
5. Build with rustkyll

## Worked Example: jekyll-vitepress-theme

**Repository:** https://github.com/crmne/jekyll-vitepress-theme

### Theme overview

jekyll-vitepress-theme is a VitePress-style documentation theme for Jekyll. It provides:
- 2 layouts: `default.html`, `home.html`
- Multiple includes for navigation, sidebar, search, alerts
- Pre-compiled CSS and JS in `assets/`
- 4 collections: `introduction`, `core_features`, `advanced`, `reference`

### Commands run

```bash
git clone https://github.com/crmne/jekyll-vitepress-theme websites/jekyll-vitepress-theme

cargo run -- build \
  --source websites/jekyll-vitepress-theme \
  --destination /tmp/vitepress-test
```

### Build output

```
Source:      websites/jekyll-vitepress-theme
Destination: /tmp/vitepress-test

Loading data files... 4 files
Loading collections... 5 collections, 16 items
Conflict: The URL '/:name/' is the destination for the following pages:
  _advanced/customizing-styles.md, _advanced/deployment.md,
  _advanced/extending-behavior.md, _core_features/code-blocks.md,
  _core_features/custom-blocks.md, _core_features/markdown-extensions.md,
  _core_features/navigation-layout.md, _core_features/search-and-outline.md,
  _introduction/configuration.md, _introduction/getting-started.md,
  _introduction/overview.md, _introduction/what-is-jekyll-vitepress-theme.md,
  _reference/configuration-reference.md, _reference/frontmatter-reference.md,
  _reference/troubleshooting.md, _reference/vitepress-parity-and-extensions.md
Copying static files... 67 files
Generating sitemap... 18 entries
Generating feed... 0 posts
Build complete!
  Collection pages: 16
  Standalone pages: 2
  Total pages:      18
  Sitemap entries:  18
  Static files:     67
  Time:             0.42s
```

### What worked

- **Layouts rendered correctly.** The `default.html` and `home.html` layouts were found and applied. The generated HTML includes proper `<head>`, navigation, sidebar, and footer markup.
- **Includes resolved.** All `{% include %}` tags in the theme's layouts found their target files and rendered.
- **Static assets copied.** All 67 static files (CSS, JS, images) from `assets/` were copied to the output.
- **Data files loaded.** The 4 YAML data files in `_data/` were loaded and accessible in templates.
- **Collections detected.** All 5 collections (introduction, core_features, advanced, reference, plus the implicit posts) were found with 16 items total.
- **Index page rendered.** The `index.md` home page rendered with the `home.html` layout.
- **No Liquid errors.** All Liquid tags and filters in the theme's templates were supported.

### What broke

1. **Permalink `:name` placeholder not resolved.** The `_config.yml` uses `permalink: "/:name/"` for collections. rustkyll output all 16 collection pages to the literal path `/:name/index.html` instead of resolving `:name` to each document's slug. Only the last-rendered page survives at that path.

2. **Non-HTML files copied that should be excluded.** Files like `eslint.config.mjs`, `package.json`, `Rakefile`, `stylelint.config.cjs` were copied to the output. The `_config.yml` has an `exclude:` list but these files are not in it; Jekyll would also copy them, so this matches Jekyll behavior.

### Summary

jekyll-vitepress-theme is close to fully working with rustkyll. The single blocking issue is the unresolved `:name` permalink placeholder for collection pages. The theme does not require SASS compilation (it ships pre-compiled CSS), making it one of the easiest themes to support.

## Worked Example: jekyll-theme-chirpy

**Repository:** https://github.com/cotes2020/jekyll-theme-chirpy

### Theme overview

jekyll-theme-chirpy is a popular, feature-rich blogging theme. It provides:
- 10 layouts including `post.html`, `page.html`, `home.html`, `archives.html`, `categories.html`, `tags.html`
- 30+ includes for analytics, comments, TOC, reading time, sharing, etc.
- SASS-based styling (no pre-compiled CSS in assets)
- A `tabs` collection for navigation pages
- A Ruby plugin (`posts-lastmod-hook.rb`) for last-modified dates
- Relies on `jekyll-archives` plugin for category/tag pages

### Commands run

```bash
git clone --depth 1 https://github.com/cotes2020/jekyll-theme-chirpy websites/jekyll-theme-chirpy

cargo run -- build \
  --source websites/jekyll-theme-chirpy \
  --destination /tmp/chirpy-test
```

### Build output (warnings)

```
Loading data files... 6 files
Loading collections... 2 collections, 8 items

Warning: failed to render posts/getting-started, writing fallback:
  template render error: liquid:  --> 3:19
  |
3 | {% assign words = include.content | strip_html | number_of_words: 'auto' %}
  |                   ^-----------------------------------------------------^
  |
  = unexpected FilterChain; expected FilterChain
  from: {% include "read-time.html" %}

Warning: failed to render posts/write-a-new-post, writing fallback:
  template parse error: liquid:    --> 387:31
    |
387 | > The Jekyll tag `{% highlight %}` is not compatible with this theme.
    |                               ^---
    |
    = language identifier expected

Warning: failed to render tabs/categories, writing fallback:
  template render error: liquid:    --> 133:44
    |
133 |         {% include "analytics/{{" platform }}.html %}
    |                                            ^---
    |
    = expected Value, Range, ">", "<", "=", ",", ":", "==", "!=", ...
  from: {% include "head.html" %}

Warnings (1):
  - Failed to compile SCSS for page jekyll-theme-chirpy:
    Error: Can't find stylesheet to import.
    3 | @use 'main';
      | ^^^^^^^^^^^
```

### What worked

- **Config parsed.** The complex `_config.yml` with nested analytics, comments, PWA settings, and collection definitions was parsed correctly.
- **Collections detected.** The `tabs` collection (4 items) and `posts` collection (4 items) were found.
- **Data files loaded.** All 6 data files (locales, etc.) were loaded.
- **Static files copied.** 14 static files from `assets/` were copied.
- **Post URLs generated.** Posts got correct URLs like `/posts/getting-started/`.
- **Sitemap and feed generated.** 14 sitemap entries and 4 feed posts.

### What broke

1. **`number_of_words` filter with argument not supported.** The `read-time.html` include uses `number_of_words: 'auto'` (a filter with a colon-separated argument). rustkyll's Liquid parser fails on this. This broke 3 of 4 posts.

2. **Dynamic include paths not supported.** Chirpy uses `{% include analytics/{{ platform }}.html %}` where the include path contains a Liquid expression. rustkyll's parser does not support variable interpolation inside `{% include %}` paths. This broke all tab pages, the 404 page, and the home page.

3. **`{% highlight %}` tag inside content confuses parser.** One post mentions `{% highlight %}` in a Markdown code fence (as documentation text, not actual Liquid). rustkyll's parser tries to interpret it as a real tag and fails because the `highlight` tag is not implemented.

4. **SASS/SCSS compilation fails.** Chirpy relies on `_sass/main.scss` and the `@use` directive. rustkyll's SCSS support cannot resolve theme-internal imports. The site renders without any CSS styling.

5. **`jekyll-archives` plugin not supported.** Chirpy uses `jekyll-archives` to generate `/tags/:name/` and `/categories/:name/` pages. rustkyll does not implement this plugin, so no tag or category archive pages are generated.

6. **Ruby plugin ignored.** The `_plugins/posts-lastmod-hook.rb` plugin is silently ignored (expected behavior -- rustkyll cannot run Ruby).

### Summary

jekyll-theme-chirpy triggers many more rustkyll limitations than the vitepress theme. The core issues are: filter arguments, dynamic include paths, the highlight tag, and SASS compilation. Even with config edits, the site cannot render correctly without engine-level fixes.

## Common Theme Blockers

The table below lists every unsupported feature encountered across the two tested themes.

| Blocker | Description | Themes Affected | Example |
|---------|-------------|-----------------|---------|
| `:name` permalink placeholder | Collection permalinks using `/:name/` are output literally instead of being resolved to the document slug | jekyll-vitepress-theme | `permalink: "/:name/"` outputs to `/:name/index.html` |
| `number_of_words` filter with argument | `number_of_words: 'auto'` (filter argument after colon) is not parsed | jekyll-theme-chirpy | `include.content \| strip_html \| number_of_words: 'auto'` |
| Dynamic include paths | `{% include analytics/{{ var }}.html %}` -- variable interpolation inside include tag path | jekyll-theme-chirpy | `{% include analytics/{{ platform }}.html %}` |
| `{% highlight %}` tag | The `highlight` Liquid tag is not implemented; even when it appears inside Markdown code fences, the parser tries to interpret it | jekyll-theme-chirpy | `{% highlight ruby %}...{% endhighlight %}` |
| SASS `@use` / `@import` resolution | SCSS files using `@use 'main'` or `@import` cannot resolve theme-internal paths | jekyll-theme-chirpy | `@use 'main';` in `assets/css/jekyll-theme-chirpy.scss` |
| `jekyll-archives` plugin | Category and tag archive page generation is not implemented | jekyll-theme-chirpy | `jekyll-archives: enabled: [categories, tags]` |
| `compress_html` layout | Some themes use a `compress.html` layout that minifies HTML using pure Liquid; this uses advanced Liquid features that may fail | jekyll-theme-chirpy | `layout: compress` in `default.html` front matter |
| `jekyll-redirect-from` plugin | Redirect page generation is not implemented | jekyll-vitepress-theme | `plugins: [jekyll-redirect-from]` |

## Blocker Priority

| Blocker | Effort | Rationale |
|---------|--------|-----------|
| `:name` permalink placeholder | Easy | The permalink resolution code already handles `:title`, `:slug`, etc. Adding `:name` is a small addition to the existing match table. |
| `number_of_words` filter with argument | Easy | The filter exists but does not accept the optional `'auto'` argument. Adding argument passthrough to the existing filter implementation is straightforward. |
| `{% highlight %}` tag | Easy | Implement as a no-op or pass-through tag that wraps content in `<pre><code>` blocks. Does not need full syntax highlighting -- just needs to not crash the parser. Also need to handle `{% highlight %}` appearing inside code fences (raw content). |
| Dynamic include paths | Medium | Requires the include tag to evaluate Liquid expressions in the path string before resolving the file. This touches the template parser and may need a two-pass approach. |
| `jekyll-redirect-from` plugin | Medium | Needs to generate redirect HTML pages based on front matter `redirect_from` / `redirect_to` values. Moderate scope but self-contained. |
| `compress_html` layout | Medium | This is a pure-Liquid layout that uses advanced string manipulation. It may work if the underlying Liquid filters are complete. Needs investigation. |
| SASS `@use` / `@import` resolution | Hard | Full SASS compilation with `@use` support requires either a SASS compiler library or shelling out to `sass`. The `@use` directive is modern SASS and not supported by older compilers. Workaround: themes can ship pre-compiled CSS. |
| `jekyll-archives` plugin | Hard | Requires generating pages dynamically for each tag and category. Needs new page generation logic, permalink resolution, and layout rendering. This is a significant feature addition. |

## Troubleshooting

### Error: "Failed to compile SCSS"

```
Failed to compile SCSS for page jekyll-theme-chirpy:
Error: Can't find stylesheet to import.
3 | @use 'main';
  | ^^^^^^^^^^^
```

**Cause:** The theme uses SASS/SCSS files that need compilation. rustkyll's SCSS support cannot resolve `@use` or complex `@import` paths.

**Workaround:** Build the theme's SASS once using the `sass` CLI tool, then place the compiled CSS in `assets/css/`:

```bash
# Install sass CLI (via npm or standalone)
npm install -g sass

# Compile the theme's SASS
sass websites/jekyll-theme-chirpy/_sass/main.scss websites/jekyll-theme-chirpy/assets/css/style.css

# Remove the SCSS source file in assets/ so rustkyll does not try to compile it
rm websites/jekyll-theme-chirpy/assets/css/jekyll-theme-chirpy.scss
```

Then update the layout's `<link>` tag to point to the compiled CSS file.

### Error: "unexpected FilterChain; expected FilterChain"

```
template render error: liquid:  --> 3:19
  |
3 | {% assign words = include.content | strip_html | number_of_words: 'auto' %}
  |                   ^-----------------------------------------------------^
  = unexpected FilterChain; expected FilterChain
```

**Cause:** The `number_of_words` filter is being called with an argument (`'auto'`). rustkyll's Liquid implementation does not support this filter argument syntax.

**Workaround:** Edit the include file to remove the argument:

```liquid
<!-- BEFORE -->
{% assign words = include.content | strip_html | number_of_words: 'auto' %}

<!-- AFTER -->
{% assign words = include.content | strip_html | number_of_words %}
```

This loses CJK word counting support but allows the template to render.

### Error: "language identifier expected" (highlight tag)

```
template parse error: liquid:    --> 387:31
387 | > The Jekyll tag `{% highlight %}` is not compatible with this theme.
    |                               ^---
    = language identifier expected
```

**Cause:** rustkyll's Liquid parser encounters `{% highlight %}` in the content and tries to parse it as a real Liquid tag, even when it appears inside Markdown code fences or backtick-quoted text.

**Workaround:** Replace `{% highlight %}` in content with `{% raw %}{% highlight %}{% endraw %}` or use HTML entities:

```markdown
<!-- BEFORE -->
The Jekyll tag `{% highlight %}` is not compatible with this theme.

<!-- AFTER -->
The Jekyll tag `{% raw %}{% highlight %}{% endraw %}` is not compatible with this theme.
```

### Error: "expected Value, Range, ..." (dynamic include)

```
template render error: liquid:    --> 133:44
133 |         {% include "analytics/{{" platform }}.html %}
    |                                            ^---
    = expected Value, Range, ">", ...
```

**Cause:** The include tag uses a dynamic path with variable interpolation (`{{ platform }}`). rustkyll does not support Liquid expressions inside include paths.

**Workaround:** Replace the dynamic include with explicit conditionals:

```liquid
<!-- BEFORE -->
{% include analytics/{{ platform }}.html %}

<!-- AFTER -->
{% if platform == "google" %}
  {% include analytics/google.html %}
{% elsif platform == "goatcounter" %}
  {% include analytics/goatcounter.html %}
{% endif %}
```

### Warning: "Conflict: The URL '/:name/' is the destination for the following pages"

**Cause:** The `:name` permalink placeholder is not being resolved. All collection pages with `permalink: "/:name/"` are output to the same literal path.

**Workaround:** Change the permalink pattern to use `:title` or `:slug` instead:

```yaml
# BEFORE
collections:
  introduction:
    output: true
    permalink: "/:name/"

# AFTER
collections:
  introduction:
    output: true
    permalink: "/:title/"
```

Note: `:title` and `:name` have slightly different semantics in Jekyll (`:name` is the filename without date prefix, `:title` is from front matter or filename). Check that URLs match your expectations.

### Pages render without CSS

**Cause:** The theme relies on SASS compilation to produce CSS. Without compiled CSS, pages render as unstyled HTML.

**Solution:** See the SCSS troubleshooting entry above. Pre-compile the theme's SASS and include the resulting CSS file in `assets/css/`.

### Plugin warnings or missing features

rustkyll silently ignores Ruby plugin files in `_plugins/`. If a theme depends on plugin behavior (e.g., `jekyll-archives` for tag pages, `jekyll-redirect-from` for redirects), those features will be missing from the output.

Check the theme's `_config.yml` `plugins:` list and `_plugins/` directory to understand what functionality may be missing.
