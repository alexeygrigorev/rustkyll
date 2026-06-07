//! Integration tests for Lanyon theme support.
//!
//! Verifies that rustkyll correctly builds a Lanyon-style Jekyll site,
//! including layouts, includes, pagination, sidebar, syntax highlighting,
//! date filters, absolute_url filter, related posts, and static assets.
//!
//! Uses self-contained fixtures created in temp dirs (no external website dependency).
//!
//! Run with: cargo test -p integration-tests --test integration_lanyon

use std::path::{Path, PathBuf};
use std::process::Command;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn rustkyll_binary() -> PathBuf {
    let release = project_root().join("target/release/rustkyll");
    if release.exists() {
        return release;
    }
    project_root().join("target/debug/rustkyll")
}

/// Create a minimal Lanyon-like site fixture in a temp directory.
fn create_lanyon_fixture() -> tempfile::TempDir {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path();

    // _config.yml
    std::fs::write(
        root.join("_config.yml"),
        r#"title: Lanyon
tagline: 'A Jekyll theme'
description: 'A reserved <a href="https://jekyllrb.com" target="_blank">Jekyll</a> theme.'
url: http://lanyon.getpoole.com
baseurl: ''
paginate: 5
permalink: pretty

author:
  name: Mark Otto
  url: https://twitter.com/mdo
  email: markdotto@gmail.com

plugins:
  - jekyll-paginate

version: 1.1.0
"#,
    )
    .unwrap();

    // _layouts/
    std::fs::create_dir_all(root.join("_layouts")).unwrap();

    std::fs::write(
        root.join("_layouts/default.html"),
        r#"<!DOCTYPE html>
<html lang="en-us">

  {% include head.html %}

  <body>

    {% include sidebar.html %}

    <div class="wrap">
      <div class="masthead">
        <div class="container">
          <h3 class="masthead-title">
            <a href="{{ site.baseurl }}/" title="Home">{{ site.title }}</a>
            <small>{{ site.tagline }}</small>
          </h3>
        </div>
      </div>

      <div class="container content">
        {{ content }}
      </div>
    </div>

    <label for="sidebar-checkbox" class="sidebar-toggle"></label>

    <script src='{{ site.baseurl }}/public/js/script.js'></script>
  </body>
</html>
"#,
    )
    .unwrap();

    std::fs::write(
        root.join("_layouts/post.html"),
        r#"---
layout: default
---

<div class="post">
  <h1 class="post-title">{{ page.title }}</h1>
  <span class="post-date">{{ page.date | date_to_string }}</span>
  {{ content }}
</div>

{% if site.related_posts.size >= 1 %}
<div class="related">
  <h2>Related posts</h2>
  <ul class="related-posts">
    {% for post in site.related_posts limit:3 %}
      <li>
        <h3>
          <a href="{{ site.baseurl }}{{ post.url }}">
            {{ post.title }}
            <small>{{ post.date | date_to_string }}</small>
          </a>
        </h3>
      </li>
    {% endfor %}
  </ul>
</div>
{% endif %}
"#,
    )
    .unwrap();

    std::fs::write(
        root.join("_layouts/page.html"),
        r#"---
layout: default
---

<div class="page">
  <h1 class="page-title">{{ page.title }}</h1>
  {{ content }}
</div>
"#,
    )
    .unwrap();

    // _includes/
    std::fs::create_dir_all(root.join("_includes")).unwrap();

    std::fs::write(
        root.join("_includes/head.html"),
        r#"<head>
  <link href="http://gmpg.org/xfn/11" rel="profile">
  <meta http-equiv="content-type" content="text/html; charset=utf-8">

  <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1">

  <title>
    {% if page.title == "Home" %}
      {{ site.title }} &middot; {{ site.tagline }}
    {% else %}
      {{ page.title }} &middot; {{ site.title }}
    {% endif %}
  </title>

  {% if page.url and site.baseurl %}
  <link rel="canonical" href="{{ page.url | absolute_url }}">
  {% endif %}

  <link rel="stylesheet" href="{{ '/public/css/poole.css' | absolute_url }}">
  <link rel="stylesheet" href="{{ '/public/css/syntax.css' | absolute_url }}">
  <link rel="stylesheet" href="{{ '/public/css/lanyon.css' | absolute_url }}">
  <link rel="stylesheet" href="https://fonts.googleapis.com/css?family=PT+Serif:400,400italic,700%7CPT+Sans:400">

  <link rel="apple-touch-icon-precomposed" sizes="144x144" href="{{ '/public/apple-touch-icon-precomposed.png' | absolute_url }}">
  <link rel="shortcut icon" href="{{ '/public/favicon.ico' | absolute_url }}">

  <link rel="alternate" type="application/rss+xml" title="RSS" href="{{ '/atom.xml' | absolute_url }}">
</head>
"#,
    )
    .unwrap();

    std::fs::write(
        root.join("_includes/sidebar.html"),
        r#"<input type="checkbox" class="sidebar-checkbox" id="sidebar-checkbox">

<div class="sidebar" id="sidebar">
  <div class="sidebar-item">
    <p>{{ site.description }}</p>
  </div>

  <nav class="sidebar-nav">
    <a class="sidebar-nav-item{% if page.title == 'Home' %} active{% endif %}" href="{{ '/' | absolute_url }}">Home</a>

    {% assign pages_list = site.pages | sort:"url" %}
    {% for node in pages_list %}
      {% if node.title != null %}
        {% if node.layout == "page" %}
          <a class="sidebar-nav-item{% if page.url == node.url %} active{% endif %}" href="{{ node.url | absolute_url }}">{{ node.title }}</a>
        {% endif %}
      {% endif %}
    {% endfor %}

    <span class="sidebar-nav-item">Currently v{{ site.version }}</span>
  </nav>

  <div class="sidebar-item">
    <p>
      &copy; {{ site.time | date: '%Y' }}. All rights reserved.
    </p>
  </div>
</div>
"#,
    )
    .unwrap();

    // _posts/
    std::fs::create_dir_all(root.join("_posts")).unwrap();

    std::fs::write(
        root.join("_posts/2020-04-01-whats-jekyll.md"),
        r#"---
layout: post
title: "What's Jekyll?"
---

[Jekyll](https://jekyllrb.com) is a static site generator, an open-source tool for creating simple yet powerful websites of all shapes and sizes.

It's an immensely useful tool and one we encourage you to use here with Lanyon.
"#,
    )
    .unwrap();

    // Post with syntax highlighting (highlight tag with js language)
    std::fs::write(
        root.join("_posts/2020-04-02-example-content.md"),
        r##"---
layout: post
title: Example content
---

<div class="message">
  Howdy! This is an example blog post that shows several types of HTML content supported in this theme.
</div>

Cum sociis natoque penatibus et magnis <a href="#">dis parturient montes</a>, nascetur ridiculus mus. *Aenean eu leo quam.*

### Code

{% highlight js %}
// Example can be run directly in your JavaScript console
var adder = new Function("a", "b", "return a + b");
adder(2, 6);
// > 8
{% endhighlight %}

Want to see something else added? <a href="https://github.com/poole/poole/issues/new">Open an issue.</a>
"##,
    )
    .unwrap();

    std::fs::write(
        root.join("_posts/2020-04-03-introducing-lanyon.md"),
        r#"---
layout: post
title: Introducing Lanyon
---

Lanyon is an unassuming [Jekyll](http://jekyllrb.com) theme that places content first by tucking away navigation in a hidden drawer.

### Built on Poole

Poole is the Jekyll Butler, serving as an upstanding and effective foundation for Jekyll themes by [@mdo](https://twitter.com/mdo).

### Browser support

Lanyon is by preference a forward-thinking project.

Thanks!
"#,
    )
    .unwrap();

    // about.md
    std::fs::write(
        root.join("about.md"),
        r#"---
layout: page
title: About
---

<p class="message">
  Hey there! This page is included as an example. Feel free to customize it for your own use upon downloading. Carry on!
</p>

In the novel, *The Strange Case of Dr. Jeykll and Mr. Hyde*, Mr. Poole is Dr. Jekyll's virtuous and loyal butler.
"#,
    )
    .unwrap();

    // 404.md
    std::fs::write(
        root.join("404.md"),
        r#"---
layout: default
title: "404: Page not found"
permalink: 404.html
---

# 404: Page not found
Sorry, we've misplaced that URL or it's pointing to something that doesn't exist.
"#,
    )
    .unwrap();

    // index.html (with pagination)
    std::fs::write(
        root.join("index.html"),
        r#"---
layout: default
title: Home
---

<div class="posts">
  {% for post in paginator.posts %}
  <div class="post">
    <h1 class="post-title">
      <a href="{{ post.url | absolute_url }}">
        {{ post.title }}
      </a>
    </h1>

    <span class="post-date">{{ post.date | date_to_string }}</span>

    {{ post.content }}
  </div>
  {% endfor %}
</div>

<div class="pagination">
  {% if paginator.next_page %}
    <a class="pagination-item older" href="{{ paginator.next_page_path | absolute_url }}">Older</a>
  {% else %}
    <span class="pagination-item older">Older</span>
  {% endif %}
  {% if paginator.previous_page %}
    {% if paginator.page == 2 %}
      <a class="pagination-item newer" href="{{ '/' | absolute_url }}">Newer</a>
    {% else %}
      <a class="pagination-item newer" href="{{ paginator.previous_page_path | absolute_url }}">Newer</a>
    {% endif %}
  {% else %}
    <span class="pagination-item newer">Newer</span>
  {% endif %}
</div>
"#,
    )
    .unwrap();

    // atom.xml (feed template)
    std::fs::write(
        root.join("atom.xml"),
        r#"---
layout: null
---

<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">

 <title>{{ site.title }}</title>
 <link href="{{ site.url }}{{ site.baseurl }}/atom.xml" rel="self"/>
 <link href="{{ site.url }}{{ site.baseurl }}/"/>
 <updated>{{ site.time | date_to_xmlschema }}</updated>
 <id>{{ site.url }}</id>
 <author>
   <name>{{ site.author.name }}</name>
   <email>{{ site.author.email }}</email>
 </author>

 {% for post in site.posts %}
 <entry>
   <title>{{ post.title }}</title>
   <link href="{{ site.url }}{{ post.url }}"/>
   <updated>{{ post.date | date_to_xmlschema }}</updated>
   <id>{{ site.url }}{{ site.baseurl }}{{ post.id }}</id>
   <content type="html">{{ post.content | xml_escape }}</content>
 </entry>
 {% endfor %}

</feed>
"#,
    )
    .unwrap();

    // public/ static assets
    std::fs::create_dir_all(root.join("public/css")).unwrap();
    std::fs::create_dir_all(root.join("public/js")).unwrap();

    std::fs::write(
        root.join("public/css/lanyon.css"),
        "/* Lanyon CSS */\n.sidebar { display: block; }\n",
    )
    .unwrap();
    std::fs::write(
        root.join("public/css/poole.css"),
        "/* Poole CSS */\nbody { margin: 0; }\n",
    )
    .unwrap();
    std::fs::write(
        root.join("public/css/syntax.css"),
        "/* Syntax CSS */\n.highlight { background: #f8f8f8; }\n",
    )
    .unwrap();
    std::fs::write(
        root.join("public/js/script.js"),
        "(function(document) { /* sidebar toggle */ })(document);\n",
    )
    .unwrap();
    std::fs::write(root.join("public/favicon.ico"), [0u8; 16]).unwrap();
    std::fs::write(
        root.join("public/apple-touch-icon-precomposed.png"),
        [0u8; 16],
    )
    .unwrap();

    tmp
}

fn build_lanyon_fixture() -> (tempfile::TempDir, tempfile::TempDir) {
    let binary = rustkyll_binary();
    assert!(
        binary.exists(),
        "rustkyll binary not found at {:?}. Run `cargo build` first.",
        binary
    );
    let source_dir = create_lanyon_fixture();
    let dest_dir = tempfile::TempDir::new().unwrap();
    let output = Command::new(&binary)
        .args(["build", "--source"])
        .arg(source_dir.path())
        .arg("--destination")
        .arg(dest_dir.path())
        .output()
        .expect("failed to run rustkyll");
    assert!(
        output.status.success(),
        "Lanyon build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Return both so the source TempDir isn't dropped (and deleted) prematurely
    (source_dir, dest_dir)
}

fn read_file(dir: &Path, relative: &str) -> String {
    let path = dir.join(relative);
    assert!(path.exists(), "Expected file not found: {}", path.display());
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e))
}

/// Count HTML files in a directory recursively.
fn count_html_files(dir: &Path) -> usize {
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                count += count_html_files(&path);
            } else if path.extension().map(|e| e == "html").unwrap_or(false) {
                count += 1;
            }
        }
    }
    count
}

// --- Page count and file existence ---

#[test]
fn test_lanyon_html_page_count() {
    let (_src, dest) = build_lanyon_fixture();
    let count = count_html_files(dest.path());
    assert_eq!(count, 6, "Expected 6 HTML files, got {}", count);
}

#[test]
fn test_lanyon_all_expected_files_exist() {
    let (_src, dest) = build_lanyon_fixture();
    let expected = [
        "index.html",
        "about/index.html",
        "404.html",
        "2020/04/01/whats-jekyll/index.html",
        "2020/04/02/example-content/index.html",
        "2020/04/03/introducing-lanyon/index.html",
        "atom.xml",
        "feed.xml",
        "sitemap.xml",
    ];
    for file in &expected {
        let path = dest.path().join(file);
        assert!(path.exists(), "Missing expected file: {}", file);
    }
}

#[test]
fn test_lanyon_static_assets_copied() {
    let (_src, dest) = build_lanyon_fixture();
    let assets = [
        "public/css/lanyon.css",
        "public/css/poole.css",
        "public/css/syntax.css",
        "public/js/script.js",
        "public/favicon.ico",
        "public/apple-touch-icon-precomposed.png",
    ];
    for asset in &assets {
        let path = dest.path().join(asset);
        assert!(path.exists(), "Missing static asset: {}", asset);
    }
}

// --- Post page content ---

#[test]
fn test_lanyon_post_title_and_date() {
    let (_src, dest) = build_lanyon_fixture();
    let html = read_file(dest.path(), "2020/04/03/introducing-lanyon/index.html");
    assert!(
        html.contains(r#"<h1 class="post-title">Introducing Lanyon</h1>"#),
        "Post page should contain post title in h1"
    );
    assert!(
        html.contains(r#"<span class="post-date">03 Apr 2020</span>"#),
        "Post page should contain formatted date via date_to_string filter"
    );
}

#[test]
fn test_lanyon_post_layout_chain() {
    let (_src, dest) = build_lanyon_fixture();
    let html = read_file(dest.path(), "2020/04/03/introducing-lanyon/index.html");
    // post.html layout wraps content in div.post
    assert!(
        html.contains(r#"class="post""#),
        "Post page should have div.post from post.html layout"
    );
    // default.html layout wraps in body structure with sidebar
    assert!(
        html.contains(r#"class="wrap""#) || html.contains(r#"class="container"#),
        "Post page should have container/wrap from default.html layout"
    );
}

#[test]
fn test_lanyon_related_posts() {
    let (_src, dest) = build_lanyon_fixture();
    let html = read_file(dest.path(), "2020/04/03/introducing-lanyon/index.html");
    assert!(
        html.contains(r#"class="related-posts""#),
        "Post page should contain related-posts section"
    );
    // Should have links to other posts
    assert!(
        html.contains("Example content") || html.contains("example-content"),
        "Related posts should link to other posts"
    );
}

// --- About page ---

#[test]
fn test_lanyon_about_page_layout() {
    let (_src, dest) = build_lanyon_fixture();
    let html = read_file(dest.path(), "about/index.html");
    // page.html layout wraps content in div.page
    assert!(
        html.contains(r#"class="page""#),
        "About page should have div.page from page.html layout"
    );
    assert!(
        html.contains("About"),
        "About page should contain the word 'About'"
    );
}

// --- Sidebar ---

#[test]
fn test_lanyon_sidebar_navigation() {
    let (_src, dest) = build_lanyon_fixture();
    let html = read_file(dest.path(), "index.html");
    assert!(
        html.contains(r#"class="sidebar-nav-item"#),
        "Pages should have sidebar navigation items"
    );
    assert!(
        html.contains("About"),
        "Sidebar should contain link to About page"
    );
}

#[test]
fn test_lanyon_sidebar_on_post_page() {
    let (_src, dest) = build_lanyon_fixture();
    let html = read_file(dest.path(), "2020/04/01/whats-jekyll/index.html");
    assert!(
        html.contains(r#"class="sidebar-nav-item"#),
        "Post pages should also have sidebar navigation"
    );
}

// --- Index page / pagination ---

#[test]
fn test_lanyon_index_shows_all_posts() {
    let (_src, dest) = build_lanyon_fixture();
    let html = read_file(dest.path(), "index.html");
    // Should contain all 3 post titles
    assert!(
        html.contains("Introducing Lanyon"),
        "Index should show 'Introducing Lanyon' post"
    );
    assert!(
        html.contains("Example content"),
        "Index should show 'Example content' post"
    );
    assert!(
        html.contains("What's Jekyll?")
            || html.contains("What&#39;s Jekyll?")
            || html.contains("What&rsquo;s Jekyll?"),
        "Index should show 'What's Jekyll?' post"
    );
}

#[test]
fn test_lanyon_index_has_post_links() {
    let (_src, dest) = build_lanyon_fixture();
    let html = read_file(dest.path(), "index.html");
    assert!(
        html.contains("/2020/04/03/introducing-lanyon/"),
        "Index should link to introducing-lanyon post"
    );
    assert!(
        html.contains("/2020/04/02/example-content/"),
        "Index should link to example-content post"
    );
    assert!(
        html.contains("/2020/04/01/whats-jekyll/"),
        "Index should link to whats-jekyll post"
    );
}

// --- Syntax highlighting ---

#[test]
fn test_lanyon_syntax_highlighting_on_post_page() {
    let (_src, dest) = build_lanyon_fixture();
    let html = read_file(dest.path(), "2020/04/02/example-content/index.html");
    assert!(
        html.contains(r#"class="highlight"#),
        "Example content post should have syntax highlighting"
    );
    assert!(
        html.contains("<code") && html.contains("language-js"),
        "Highlight block should contain code element with language-js class"
    );
}

// --- absolute_url filter ---

#[test]
fn test_lanyon_absolute_url_filter() {
    let (_src, dest) = build_lanyon_fixture();
    let html = read_file(dest.path(), "index.html");
    assert!(
        html.contains("http://lanyon.getpoole.com"),
        "Pages should use absolute_url filter with configured url"
    );
}

// --- Feed and sitemap ---

#[test]
fn test_lanyon_feed_xml_has_rendered_content() {
    let (_src, dest) = build_lanyon_fixture();
    let xml = read_file(dest.path(), "feed.xml");
    // Auto-generated feed should have rendered HTML, not raw Markdown
    assert!(
        xml.contains("<p>") || xml.contains("&lt;p&gt;"),
        "feed.xml should contain rendered HTML (p tags), not raw Markdown"
    );
    assert!(
        xml.contains("Introducing Lanyon") || xml.contains("Lanyon"),
        "feed.xml should contain post titles"
    );
}

#[test]
fn test_lanyon_sitemap_lists_all_pages() {
    let (_src, dest) = build_lanyon_fixture();
    let xml = read_file(dest.path(), "sitemap.xml");
    // Sitemap should reference all pages
    assert!(
        xml.contains("introducing-lanyon"),
        "Sitemap should list introducing-lanyon post"
    );
    assert!(
        xml.contains("example-content"),
        "Sitemap should list example-content post"
    );
    assert!(
        xml.contains("whats-jekyll"),
        "Sitemap should list whats-jekyll post"
    );
    assert!(xml.contains("about"), "Sitemap should list about page");
}

// --- Unicode content (per project conventions) ---

#[test]
fn test_lanyon_handles_smart_quotes_in_title() {
    let (_src, dest) = build_lanyon_fixture();
    let html = read_file(dest.path(), "2020/04/01/whats-jekyll/index.html");
    // The post title "What's Jekyll?" should render (may use smart quote or HTML entity)
    let has_title = html.contains("What's Jekyll?")
        || html.contains("What&#39;s Jekyll?")
        || html.contains("What&rsquo;s Jekyll?")
        || html.contains("What\u{2019}s Jekyll?");
    assert!(
        has_title,
        "What's Jekyll post should render title with apostrophe"
    );
}
