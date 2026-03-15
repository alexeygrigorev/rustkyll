#!/usr/bin/env bash
# generate-synthetic-sites.sh -- Generate synthetic benchmark sites for testing.
#
# Creates two sites in websites/:
#   - large-blog-3000: A blog with 3000 posts across multiple categories
#   - large-docs-site: A documentation site with 800 pages across 10 sections
#
# These are deterministic: running twice produces identical output.
#
# Usage: scripts/generate-synthetic-sites.sh [--dir WEBSITES_DIR]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
WEBSITES_DIR="$PROJECT_DIR/websites"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dir) WEBSITES_DIR="$2"; shift 2 ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

mkdir -p "$WEBSITES_DIR"

# =============================================================================
# large-blog-3000
# =============================================================================

generate_large_blog() {
    local site_dir="$WEBSITES_DIR/large-blog-3000"

    if [[ -d "$site_dir" ]] && [[ -f "$site_dir/_config.yml" ]]; then
        local post_count
        post_count=$(find "$site_dir/_posts" -name "*.md" 2>/dev/null | wc -l)
        if [[ "$post_count" -eq 3000 ]]; then
            echo "large-blog-3000 already exists with 3000 posts, skipping."
            return
        fi
    fi

    echo "Generating large-blog-3000..."
    rm -rf "$site_dir"
    mkdir -p "$site_dir/_posts" "$site_dir/_layouts"

    # _config.yml
    cat > "$site_dir/_config.yml" << 'YAML'
title: Large Blog Benchmark
description: A synthetic Jekyll blog with 500 posts for benchmarking
url: "https://example.com"
baseurl: ""
permalink: /:year/:month/:day/:title/
defaults:
  - scope:
      path: ""
      type: "posts"
    values:
      layout: "post"
  - scope:
      path: ""
    values:
      layout: "default"
YAML

    # _layouts/default.html
    cat > "$site_dir/_layouts/default.html" << 'HTML'
<!DOCTYPE html>
<html>
<head><title>{{ page.title }}</title></head>
<body>
<nav><a href="/">Home</a></nav>
<main>{{ content }}</main>
<footer>Built with Jekyll</footer>
</body>
</html>
HTML

    # _layouts/post.html
    cat > "$site_dir/_layouts/post.html" << 'HTML'
---
layout: default
---
<article>
<h1>{{ page.title }}</h1>
<time>{{ page.date | date: "%B %d, %Y" }}</time>
{% if page.categories %}<p>Categories: {{ page.categories | join: ", " }}</p>{% endif %}
{% if page.tags %}<p>Tags: {{ page.tags | join: ", " }}</p>{% endif %}

{{ content }}

<h2>Posts in Same Category</h2>
<ul>
{% for cat in page.categories %}
{% assign cat_posts = site.categories[cat] %}
{% for post in cat_posts limit:10 %}
{% if post.url != page.url %}
<li><a href="{{ post.url }}">{{ post.title }}</a></li>
{% endif %}
{% endfor %}
{% endfor %}
</ul>

<h2>Posts with Same Tags</h2>
<ul>
{% for tag in page.tags %}
{% assign tag_posts = site.tags[tag] %}
{% for post in tag_posts limit:5 %}
{% if post.url != page.url %}
<li><a href="{{ post.url }}">{{ post.title }}</a></li>
{% endif %}
{% endfor %}
{% endfor %}
</ul>
</article>
HTML

    # index.html
    cat > "$site_dir/index.html" << 'HTML'
---
layout: default
title: Home
---
<h1>Blog Posts ({{ site.posts | size }} total)</h1>

{% for category in site.categories %}
<h2>{{ category[0] | capitalize }} ({{ category[1] | size }} posts)</h2>
<ul>
{% for post in category[1] limit:5 %}
<li><a href="{{ post.url }}">{{ post.title }}</a> - {{ post.date | date: "%Y-%m-%d" }}</li>
{% endfor %}
</ul>
{% endfor %}

<h2>All Posts</h2>
<ul>
{% for post in site.posts %}
<li>
  <a href="{{ post.url }}">{{ post.title }}</a>
  <span>{{ post.date | date: "%Y-%m-%d" }}</span>
  {% if post.tags %}<span>Tags: {{ post.tags | join: ", " }}</span>{% endif %}
</li>
{% endfor %}
</ul>
HTML

    # Generate 3000 posts deterministically
    local categories=("technology" "science" "business" "lifestyle" "education"
                      "health" "travel" "food" "sports" "entertainment")
    local tags=("popular" "featured" "review" "tutorial" "opinion"
                "news" "guide" "analysis" "interview" "case-study")

    for i in $(seq 0 2999); do
        # Deterministic date: spread across 2015-2024 (about 300 posts/year)
        local year=$((2015 + i / 420))
        if [[ $year -gt 2024 ]]; then year=2024; fi
        local day_of_year=$(( (i * 37) % 365 + 1 ))
        local date
        date=$(date -d "$year-01-01 +$((day_of_year - 1)) days" +%Y-%m-%d 2>/dev/null || echo "$year-01-01")

        local cat_idx=$(( i % ${#categories[@]} ))
        local tag1_idx=$(( (i * 3) % ${#tags[@]} ))
        local tag2_idx=$(( (i * 7 + 1) % ${#tags[@]} ))
        local category="${categories[$cat_idx]}"
        local tag1="${tags[$tag1_idx]}"
        local tag2="${tags[$tag2_idx]}"

        local post_num=$((i + 1))
        local filename="${date}-post-${post_num}.md"

        cat > "$site_dir/_posts/$filename" << EOF
---
title: "Blog Post Number ${post_num}: A Discussion on ${category}"
date: ${date}
categories: ${category}
tags:
  - ${tag1}
  - ${tag2}
---

This is blog post number ${post_num} about ${category}.

## Introduction

Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor
incididunt ut labore et dolore magna aliqua.

## Main Content

Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu
fugiat nulla pariatur.

### Section ${post_num}.1

Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut
aliquip ex ea commodo consequat.

### Section ${post_num}.2

Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia
deserunt mollit anim id est laborum.

## Conclusion

This concludes post number ${post_num} in the ${category} category.
EOF
    done

    echo "Generated large-blog-3000 with 3000 posts."
}

# =============================================================================
# large-docs-site
# =============================================================================

generate_large_docs() {
    local site_dir="$WEBSITES_DIR/large-docs-site"

    if [[ -d "$site_dir" ]] && [[ -f "$site_dir/_config.yml" ]]; then
        local page_count
        page_count=$(find "$site_dir/docs" -name "*.md" 2>/dev/null | wc -l)
        if [[ "$page_count" -eq 800 ]]; then
            echo "large-docs-site already exists with 800 pages, skipping."
            return
        fi
    fi

    echo "Generating large-docs-site..."
    rm -rf "$site_dir"
    mkdir -p "$site_dir/_layouts"

    # _config.yml
    cat > "$site_dir/_config.yml" << 'YAML'
title: Large Documentation Site
description: A synthetic documentation site with 300 pages for benchmarking
url: "https://docs.example.com"
baseurl: ""
defaults:
  - scope:
      path: "docs"
    values:
      layout: "doc"
  - scope:
      path: ""
    values:
      layout: "default"
YAML

    # _layouts/default.html
    cat > "$site_dir/_layouts/default.html" << 'HTML'
<!DOCTYPE html>
<html>
<head><title>{{ page.title }} | Documentation</title></head>
<body>
<nav>
<a href="/">Home</a>
{% for page in site.pages %}
  {% if page.section %}
  <a href="{{ page.url }}">{{ page.title }}</a>
  {% endif %}
{% endfor %}
</nav>
<main>{{ content }}</main>
</body>
</html>
HTML

    # _layouts/doc.html
    cat > "$site_dir/_layouts/doc.html" << 'HTML'
---
layout: default
---
<article class="doc">
<h1>{{ page.title }}</h1>
{% if page.section %}<p class="section">Section: {{ page.section }}</p>{% endif %}
{{ content }}
{% if page.see_also %}
<h2>See Also</h2>
<ul>
{% for ref in page.see_also %}
<li><a href="{{ ref }}">{{ ref }}</a></li>
{% endfor %}
</ul>
{% endif %}
</article>
HTML

    # index.html
    cat > "$site_dir/index.html" << 'HTML'
---
layout: default
title: Documentation Home
---
<h1>Documentation</h1>

<h2>All Pages</h2>
<ul>
{% assign doc_pages = site.pages | where_exp: "p", "p.section != nil" %}
{% for p in doc_pages %}
<li><a href="{{ p.url }}">{{ p.title }}</a> ({{ p.section }})</li>
{% endfor %}
</ul>
HTML

    # Generate 800 pages across 10 sections (80 per section)
    local sections=("api-reference" "getting-started" "configuration"
                    "deployment" "integrations" "security"
                    "troubleshooting" "best-practices" "architecture"
                    "migration")

    for section in "${sections[@]}"; do
        mkdir -p "$site_dir/docs/$section"
    done

    local page_num=0
    for section_idx in $(seq 0 $((${#sections[@]} - 1))); do
        local section="${sections[$section_idx]}"
        for j in $(seq 1 80); do
            page_num=$((page_num + 1))
            local see_also_idx=$(( (page_num * 13 + 7) % 800 + 1 ))

            cat > "$site_dir/docs/$section/page-${page_num}.md" << EOF
---
title: "${section} Guide Part ${page_num}"
section: ${section}
order: ${page_num}
---

# ${section} - Part ${page_num}

This documentation page covers aspect ${page_num} of ${section}.

## Overview

This section provides detailed information about configuring and using
the ${section} features of the system.

## Configuration

To configure this feature, add the following to your configuration file:

\`\`\`yaml
${section}:
  enabled: true
  option_${page_num}: value
\`\`\`

## Usage

After configuration, you can use the ${section} feature as follows:

1. Step one for page ${page_num}
2. Step two for page ${page_num}
3. Step three for page ${page_num}

## Examples

Here is an example of using ${section} feature ${page_num}:

\`\`\`
example code for ${section} page ${page_num}
\`\`\`

## Troubleshooting

If you encounter issues with ${section} page ${page_num}, check the following:

- Verify your configuration is correct
- Check the logs for error messages
- Ensure all dependencies are installed
EOF
        done
    done

    echo "Generated large-docs-site with $page_num pages across ${#sections[@]} sections."
}

# =============================================================================
# Main
# =============================================================================

generate_large_blog
generate_large_docs

echo "Synthetic site generation complete."
