//! Layout system for Jekyll-compatible template wrapping.
//!
//! Layouts are HTML templates that wrap page content. The page's rendered
//! HTML is substituted into the layout via the `{{ content }}` variable.
//!
//! # Layout Wrapping Flow
//!
//! 1. Render the page's body content through the template engine
//! 2. Look up the layout from the page's front matter
//! 3. Build a context with `page.*`, `site.*`, and `content` variables
//! 4. Render the layout template with this context
//! 5. If the layout specifies a parent layout (chaining), repeat

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use liquid::model::Value as LiquidValue;
use liquid::Object;

use super::context::{normalize_arrays, yaml_to_liquid};
use super::engine::{CachedSiteContext, Template, TemplateEngine};
use super::error::TemplateError;
use crate::frontmatter::FrontMatter;

/// A loaded layout template with its source and optional parent layout name.
#[derive(Debug, Clone)]
pub struct Layout {
    /// The raw template source of this layout.
    pub source: String,
    /// The name of the parent layout, if this layout chains to another.
    pub parent_layout: Option<String>,
}

/// Engine for loading and rendering layouts with includes.
///
/// Holds all loaded layouts and a configured `TemplateEngine` with includes
/// registered as partials.
pub struct LayoutEngine {
    /// Map of layout name (without extension) to Layout.
    layouts: HashMap<String, Layout>,
    /// Pre-compiled layout templates (parsed once, rendered many times).
    compiled_layouts: HashMap<String, Template>,
    /// Template engine with includes registered as partials.
    engine: TemplateEngine,
}

impl LayoutEngine {
    /// Create a new `LayoutEngine` by loading layouts and includes from directories.
    ///
    /// # Arguments
    ///
    /// * `layouts_dir` - Path to the `_layouts/` directory
    /// * `includes_dir` - Path to the `_includes/` directory
    ///
    /// # Errors
    ///
    /// Returns `TemplateError` if directories cannot be read or the template
    /// engine fails to build.
    pub fn new(layouts_dir: &Path, includes_dir: &Path) -> Result<Self, TemplateError> {
        let layouts = load_layouts(layouts_dir)?;
        let engine = TemplateEngine::with_includes(includes_dir)?;
        let compiled_layouts = Self::compile_layouts(&layouts, &engine)?;
        Ok(Self {
            layouts,
            compiled_layouts,
            engine,
        })
    }

    /// Create a `LayoutEngine` from pre-loaded layouts and includes.
    ///
    /// Useful for testing.
    pub fn from_maps(
        layouts: HashMap<String, Layout>,
        includes: &HashMap<String, String>,
    ) -> Result<Self, TemplateError> {
        let engine = TemplateEngine::with_includes_map(includes)?;
        let compiled_layouts = Self::compile_layouts(&layouts, &engine)?;
        Ok(Self {
            layouts,
            compiled_layouts,
            engine,
        })
    }

    fn compile_layouts(
        layouts: &HashMap<String, Layout>,
        engine: &TemplateEngine,
    ) -> Result<HashMap<String, Template>, TemplateError> {
        let mut compiled = HashMap::new();
        for (name, layout) in layouts {
            let template = engine.parse(&layout.source)?;
            compiled.insert(name.clone(), template);
        }
        Ok(compiled)
    }

    /// Get a reference to the underlying template engine.
    pub fn engine(&self) -> &TemplateEngine {
        &self.engine
    }

    /// Get the names of all loaded layouts.
    pub fn layout_names(&self) -> Vec<&str> {
        self.layouts.keys().map(|s| s.as_str()).collect()
    }

    /// Check if a layout exists.
    pub fn has_layout(&self, name: &str) -> bool {
        self.layouts.contains_key(name)
    }

    /// Render page content wrapped in a layout.
    ///
    /// # Arguments
    ///
    /// * `layout_name` - Name of the layout (e.g. "post", "home")
    /// * `content` - The rendered page body HTML
    /// * `page_front_matter` - The page's front matter as YAML key-value pairs
    /// * `site_context` - The site-level context object (config, collections, data)
    ///
    /// # Errors
    ///
    /// Returns `TemplateError::LayoutNotFound` if the layout does not exist.
    /// Returns `TemplateError::ParseError` or `TemplateError::RenderError` for
    /// template issues.
    pub fn render(
        &self,
        layout_name: &str,
        content: &str,
        page_front_matter: &FrontMatter,
        site_context: &Object,
    ) -> Result<String, TemplateError> {
        let layout = self
            .layouts
            .get(layout_name)
            .ok_or_else(|| TemplateError::LayoutNotFound(layout_name.to_string()))?;

        let ctx = build_render_context(content, page_front_matter, site_context);

        // Use pre-compiled template if available, otherwise parse on the fly
        let result = if let Some(compiled) = self.compiled_layouts.get(layout_name) {
            self.engine.render(compiled, &ctx)?
        } else {
            self.engine.parse_and_render(&layout.source, &ctx)?
        };

        // Support layout chaining: if the layout specifies a parent layout, wrap again
        if let Some(ref parent_name) = layout.parent_layout {
            self.render(parent_name, &result, page_front_matter, site_context)
        } else {
            Ok(result)
        }
    }

    /// Render page content through the template engine (resolve Liquid tags
    /// in the content itself), then wrap in a layout.
    ///
    /// This is the full rendering pipeline:
    /// 1. Render the page body content (resolving any Liquid tags)
    /// 2. Wrap the result in the specified layout
    ///
    /// # Errors
    ///
    /// Returns `TemplateError` for any template or layout issues.
    pub fn render_page(
        &self,
        layout_name: &str,
        raw_content: &str,
        page_front_matter: &FrontMatter,
        site_context: &Object,
    ) -> Result<String, TemplateError> {
        // Step 1: Render the page content itself (it may contain Liquid tags).
        // Skip Liquid parsing for plain HTML content (no {{ or {% tags).
        let rendered_content = if raw_content.contains("{{") || raw_content.contains("{%") {
            let page_ctx = build_render_context("", page_front_matter, site_context);
            self.engine.parse_and_render(raw_content, &page_ctx)?
        } else {
            raw_content.to_string()
        };

        // Step 2: Wrap in layout
        self.render(
            layout_name,
            &rendered_content,
            page_front_matter,
            site_context,
        )
    }

    /// Build a `CachedSiteContext` from a site Object.
    ///
    /// Call this ONCE before rendering many pages, then pass the result to
    /// `render_with_cached_site` / `render_page_with_cached_site`.
    /// This converts the site Object into a `LenientValue` tree once,
    /// avoiding O(n^2) deep-cloning on large sites.
    pub fn build_cached_site_context(site_context: &Object) -> CachedSiteContext {
        CachedSiteContext::new(site_context)
    }

    /// Render page content wrapped in a layout, using a pre-built cached site context.
    ///
    /// This is the performance-optimized version of `render()`. The site
    /// `LenientValue` tree is built once and reused, avoiding the O(n^2)
    /// deep-clone that was the main bottleneck on large sites.
    pub fn render_with_cached_site(
        &self,
        layout_name: &str,
        content: &str,
        page_front_matter: &FrontMatter,
        cached_site: &CachedSiteContext,
    ) -> Result<String, TemplateError> {
        let layout = self
            .layouts
            .get(layout_name)
            .ok_or_else(|| TemplateError::LayoutNotFound(layout_name.to_string()))?;

        let ctx = build_render_context_page_only(content, page_front_matter);

        let result = if let Some(compiled) = self.compiled_layouts.get(layout_name) {
            self.engine
                .render_with_cached_site(compiled, &ctx, cached_site)?
        } else {
            self.engine
                .parse_and_render_with_cached_site(&layout.source, &ctx, cached_site)?
        };

        if let Some(ref parent_name) = layout.parent_layout {
            self.render_with_cached_site(parent_name, &result, page_front_matter, cached_site)
        } else {
            Ok(result)
        }
    }

    /// Render page content through the template engine then wrap in a layout,
    /// using a pre-built cached site context.
    ///
    /// This is the performance-optimized version of `render_page()`.
    pub fn render_page_with_cached_site(
        &self,
        layout_name: &str,
        raw_content: &str,
        page_front_matter: &FrontMatter,
        cached_site: &CachedSiteContext,
    ) -> Result<String, TemplateError> {
        // Optimization: skip Liquid parsing for content that has no Liquid tags.
        // Many collection items (podcast, books, people) have plain HTML content
        // with no Liquid tags. Parsing plain HTML through the Liquid parser is
        // pure overhead.
        let rendered_content = if raw_content.contains("{{") || raw_content.contains("{%") {
            let page_ctx = build_render_context_page_only("", page_front_matter);
            self.engine
                .parse_and_render_with_cached_site(raw_content, &page_ctx, cached_site)?
        } else {
            raw_content.to_string()
        };

        self.render_with_cached_site(
            layout_name,
            &rendered_content,
            page_front_matter,
            cached_site,
        )
    }
}

/// Load all layout files from a directory.
///
/// Layout files are expected to be `.html` files. The layout name is the
/// file stem (e.g., `post.html` -> `"post"`).
///
/// If a layout file contains front matter with a `layout` key, it is
/// recorded as the parent layout for chaining.
///
/// # Errors
///
/// Returns `TemplateError::IoError` if the directory or any file cannot be read.
pub fn load_layouts(layouts_dir: &Path) -> Result<HashMap<String, Layout>, TemplateError> {
    let mut layouts = HashMap::new();

    if !layouts_dir.exists() {
        return Ok(layouts);
    }

    let entries = fs::read_dir(layouts_dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        if !filename.ends_with(".html") {
            continue;
        }

        let name = filename
            .strip_suffix(".html")
            .unwrap_or(&filename)
            .to_string();

        let source = fs::read_to_string(&path)?;

        // Check for front matter in layout (for layout chaining)
        let (parent_layout, clean_source) = extract_layout_front_matter(&source);

        layouts.insert(
            name,
            Layout {
                source: clean_source,
                parent_layout,
            },
        );
    }

    Ok(layouts)
}

/// Extract front matter from a layout file, returning the parent layout name
/// (if any) and the content without front matter.
fn extract_layout_front_matter(source: &str) -> (Option<String>, String) {
    let trimmed = source.trim_start_matches('\u{feff}');
    if !trimmed.starts_with("---") {
        return (None, source.to_string());
    }

    let after_opening = &trimmed[3..];
    let rest = if let Some(stripped) = after_opening.strip_prefix('\n') {
        stripped
    } else if let Some(stripped) = after_opening.strip_prefix("\r\n") {
        stripped
    } else {
        return (None, source.to_string());
    };

    for (i, line) in rest.lines().enumerate() {
        if line.trim() == "---" {
            let byte_offset: usize = rest.lines().take(i).map(|l| l.len() + 1).sum();
            let yaml_str = &rest[..byte_offset];

            let body = &rest[byte_offset..];
            let body = if let Some(pos) = body.find('\n') {
                &body[pos + 1..]
            } else {
                ""
            };

            // Parse the YAML to find a `layout` key
            let parent_layout =
                serde_yaml::from_str::<HashMap<String, serde_yaml::Value>>(yaml_str)
                    .ok()
                    .and_then(|fm| {
                        fm.get("layout")
                            .and_then(|v| v.as_str().map(|s| s.to_string()))
                    });

            return (parent_layout, body.to_string());
        }
    }

    (None, source.to_string())
}

/// Build the rendering context for a layout or page.
///
/// Includes:
/// - `page.*` from front matter
/// - `site.*` from site context
/// - `content` with the rendered page body
/// - `site.time` with the current build time
pub fn build_render_context(
    content: &str,
    page_front_matter: &FrontMatter,
    site_context: &Object,
) -> Object {
    let mut ctx = Object::new();

    // Build page object from front matter, normalizing arrays so that objects
    // in arrays have uniform keys (prevents "Unknown index" in Liquid for loops)
    let mut page = Object::new();
    for (key, value) in page_front_matter {
        page.insert(key.clone().into(), normalize_arrays(yaml_to_liquid(value)));
    }
    ctx.insert("page".into(), LiquidValue::Object(page));

    // Insert site context
    ctx.insert("site".into(), LiquidValue::Object(site_context.clone()));

    // Insert content
    ctx.insert("content".into(), LiquidValue::scalar(content.to_owned()));

    ctx
}

/// Build a render context with only page and content -- no site.
///
/// Used with `render_with_cached_site` / `render_page_with_cached_site` where
/// the site context is provided separately via a `CachedSiteContext`. This
/// avoids the expensive `site_context.clone()` that was the main O(n^2)
/// bottleneck on large sites.
pub fn build_render_context_page_only(content: &str, page_front_matter: &FrontMatter) -> Object {
    let mut ctx = Object::new();

    let mut page = Object::new();
    for (key, value) in page_front_matter {
        page.insert(key.clone().into(), normalize_arrays(yaml_to_liquid(value)));
    }
    ctx.insert("page".into(), LiquidValue::Object(page));

    ctx.insert("content".into(), LiquidValue::scalar(content.to_owned()));

    ctx
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::load_includes;
    use std::path::PathBuf;

    fn site_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("datatalksclub.github.io")
    }

    fn layouts_dir() -> PathBuf {
        site_dir().join("_layouts")
    }

    fn includes_dir() -> PathBuf {
        site_dir().join("_includes")
    }

    // ========================================================================
    // Unit: Layout loading
    // ========================================================================

    #[test]
    fn test_load_all_six_layouts() {
        let layouts = load_layouts(&layouts_dir()).unwrap();
        let expected = ["home", "page", "post", "book", "podcast", "author"];
        for name in &expected {
            assert!(layouts.contains_key(*name), "Missing layout: {}", name);
        }
        assert_eq!(layouts.len(), 6);
    }

    #[test]
    fn test_load_layouts_each_parses_without_error() {
        let layouts = load_layouts(&layouts_dir()).unwrap();
        let includes = load_includes(&includes_dir()).unwrap();
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        for (name, layout) in &layouts {
            let result = engine.parse(&layout.source);
            assert!(
                result.is_ok(),
                "Layout '{}' failed to parse: {:?}",
                name,
                result.err()
            );
        }
    }

    #[test]
    fn test_load_nonexistent_layout_dir_returns_empty() {
        let layouts = load_layouts(Path::new("/nonexistent/dir")).unwrap();
        assert!(layouts.is_empty());
    }

    #[test]
    fn test_no_layout_chaining_in_real_site() {
        let layouts = load_layouts(&layouts_dir()).unwrap();
        for (name, layout) in &layouts {
            assert!(
                layout.parent_layout.is_none(),
                "Layout '{}' unexpectedly has parent layout: {:?}",
                name,
                layout.parent_layout
            );
        }
    }

    // ========================================================================
    // Unit: Include loading and registration
    // ========================================================================

    #[test]
    fn test_load_all_includes() {
        let includes = load_includes(&includes_dir()).unwrap();
        // 15 top-level files + 6 files in course-structured-data/
        assert!(
            includes.len() >= 20,
            "Expected at least 20 include files, got {}",
            includes.len()
        );
    }

    #[test]
    fn test_includes_contain_expected_files() {
        let includes = load_includes(&includes_dir()).unwrap();
        let expected = [
            "head.html",
            "header.html",
            "footer.html",
            "subscribe.html",
            "subscribe-main.html",
            "authors.html",
            "book.html",
            "event.html",
            "youtube.html",
            "anchor.html",
            "mathjax.html",
            "charts.html",
            "related-posts.html",
            "faq-accordion.html",
        ];
        for name in &expected {
            assert!(includes.contains_key(*name), "Missing include: {}", name);
        }
    }

    #[test]
    fn test_includes_contain_course_structured_data() {
        let includes = load_includes(&includes_dir()).unwrap();
        assert!(
            includes.contains_key(
                "course-structured-data/data-engineering-zoomcamp-structured-data.html"
            ),
            "Missing course-structured-data include"
        );
    }

    #[test]
    fn test_engine_with_includes_builds_successfully() {
        let engine = TemplateEngine::with_includes(&includes_dir());
        assert!(
            engine.is_ok(),
            "Failed to build engine with includes: {:?}",
            engine.err()
        );
    }

    // ========================================================================
    // Unit: Jekyll include syntax compatibility
    // ========================================================================

    #[test]
    fn test_include_unquoted_filename() {
        let mut includes = HashMap::new();
        includes.insert("simple.html".to_string(), "INCLUDED".to_string());
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let output = engine
            .parse_and_render("{% include simple.html %}", &ctx)
            .unwrap();
        assert_eq!(output, "INCLUDED");
    }

    #[test]
    fn test_include_with_string_param() {
        let mut includes = HashMap::new();
        includes.insert(
            "sub.html".to_string(),
            "sub={{ include.subscribe }}".to_string(),
        );
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let output = engine
            .parse_and_render(r#"{% include sub.html subscribe="true" %}"#, &ctx)
            .unwrap();
        assert_eq!(output, "sub=true");
    }

    #[test]
    fn test_include_with_variable_param() {
        let mut includes = HashMap::new();
        includes.insert(
            "auth.html".to_string(),
            "{% for a in include.authors %}{{ a }}{% endfor %}".to_string(),
        );
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();

        let mut page = Object::new();
        page.insert(
            "authors".into(),
            LiquidValue::Array(vec![
                LiquidValue::scalar("alice"),
                LiquidValue::scalar("bob"),
            ]),
        );
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page));

        let output = engine
            .parse_and_render("{% include auth.html authors=page.authors %}", &ctx)
            .unwrap();
        assert_eq!(output, "alicebob");
    }

    #[test]
    fn test_include_with_multiple_params() {
        let mut includes = HashMap::new();
        includes.insert(
            "ev.html".to_string(),
            "title={{ include.event }} speakers={{ include.speakers }}".to_string(),
        );
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let output = engine
            .parse_and_render(
                r#"{% include ev.html event="test event" speakers=false %}"#,
                &ctx,
            )
            .unwrap();
        assert!(output.contains("title=test event"));
        assert!(output.contains("speakers=false"));
    }

    #[test]
    fn test_include_no_params_include_object_exists() {
        let mut includes = HashMap::new();
        includes.insert("noparam.html".to_string(), "OK".to_string());
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let output = engine
            .parse_and_render("{% include noparam.html %}", &ctx)
            .unwrap();
        assert_eq!(output, "OK");
    }

    #[test]
    fn test_include_nonexistent_file_errors() {
        let includes = HashMap::new();
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let result = engine.parse_and_render("{% include missing.html %}", &ctx);
        assert!(result.is_err());
    }

    // ========================================================================
    // Unit: Nested includes
    // ========================================================================

    #[test]
    fn test_nested_includes() {
        let mut includes = HashMap::new();
        includes.insert("inner.html".to_string(), "INNER".to_string());
        includes.insert(
            "outer.html".to_string(),
            "OUTER[{% include inner.html %}]".to_string(),
        );
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let output = engine
            .parse_and_render("{% include outer.html %}", &ctx)
            .unwrap();
        assert_eq!(output, "OUTER[INNER]");
    }

    #[test]
    fn test_nested_includes_with_params() {
        let mut includes = HashMap::new();
        includes.insert(
            "authors.html".to_string(),
            "{% for a in include.authors %}{{ a }}{% endfor %}".to_string(),
        );
        includes.insert(
            "book.html".to_string(),
            "BOOK:{% include authors.html authors=include.authors %}".to_string(),
        );
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();

        let mut ctx = Object::new();
        let authors = LiquidValue::Array(vec![
            LiquidValue::scalar("alice"),
            LiquidValue::scalar("bob"),
        ]);
        // The outer include passes authors from context
        let mut page = Object::new();
        page.insert("authors".into(), authors);
        ctx.insert("page".into(), LiquidValue::Object(page));

        let output = engine
            .parse_and_render("{% include book.html authors=page.authors %}", &ctx)
            .unwrap();
        assert_eq!(output, "BOOK:alicebob");
    }

    // ========================================================================
    // Unit: Layout wrapping
    // ========================================================================

    #[test]
    fn test_layout_wrapping_simple() {
        let mut layouts = HashMap::new();
        layouts.insert(
            "simple".to_string(),
            Layout {
                source: "<html><body>{{ content }}</body></html>".to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();
        let fm = FrontMatter::new();
        let site = Object::new();

        let output = engine.render("simple", "Hello World", &fm, &site).unwrap();
        assert_eq!(output, "<html><body>Hello World</body></html>");
    }

    #[test]
    fn test_layout_wrapping_with_page_vars() {
        let mut layouts = HashMap::new();
        layouts.insert(
            "test".to_string(),
            Layout {
                source: "<h1>{{ page.title }}</h1>{{ content }}".to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();

        let mut fm = FrontMatter::new();
        fm.insert(
            "title".to_string(),
            serde_yaml::Value::String("My Page".to_string()),
        );
        let site = Object::new();

        let output = engine.render("test", "Body here", &fm, &site).unwrap();
        assert_eq!(output, "<h1>My Page</h1>Body here");
    }

    #[test]
    fn test_layout_wrapping_with_site_vars() {
        let mut layouts = HashMap::new();
        layouts.insert(
            "test".to_string(),
            Layout {
                source: "{{ site.name }} | {{ content }}".to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();

        let fm = FrontMatter::new();
        let mut site = Object::new();
        site.insert("name".into(), LiquidValue::scalar("DataTalks.Club"));

        let output = engine.render("test", "Content", &fm, &site).unwrap();
        assert_eq!(output, "DataTalks.Club | Content");
    }

    #[test]
    fn test_layout_not_found_error() {
        let layouts = HashMap::new();
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();
        let fm = FrontMatter::new();
        let site = Object::new();

        let result = engine.render("nonexistent", "Content", &fm, &site);
        assert!(matches!(result, Err(TemplateError::LayoutNotFound(_))));
    }

    #[test]
    fn test_layout_with_no_content_marker() {
        let mut layouts = HashMap::new();
        layouts.insert(
            "nocontent".to_string(),
            Layout {
                source: "<html><body>Static only</body></html>".to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();
        let fm = FrontMatter::new();
        let site = Object::new();

        // No crash, content is just lost
        let output = engine.render("nocontent", "Discarded", &fm, &site).unwrap();
        assert_eq!(output, "<html><body>Static only</body></html>");
    }

    #[test]
    fn test_layout_with_empty_content() {
        let mut layouts = HashMap::new();
        layouts.insert(
            "test".to_string(),
            Layout {
                source: "<div>{{ content }}</div>".to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();
        let fm = FrontMatter::new();
        let site = Object::new();

        let output = engine.render("test", "", &fm, &site).unwrap();
        assert_eq!(output, "<div></div>");
    }

    #[test]
    fn test_layout_missing_page_vars_render_empty() {
        let mut layouts = HashMap::new();
        layouts.insert(
            "test".to_string(),
            Layout {
                source: "Title: {{ page.title }}, Desc: {{ page.missing_field }}".to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();
        let fm = FrontMatter::new();
        let site = Object::new();

        let output = engine.render("test", "", &fm, &site).unwrap();
        assert_eq!(output, "Title: , Desc: ");
    }

    #[test]
    fn test_layout_chaining() {
        let mut layouts = HashMap::new();
        layouts.insert(
            "inner".to_string(),
            Layout {
                source: "[INNER:{{ content }}]".to_string(),
                parent_layout: Some("outer".to_string()),
            },
        );
        layouts.insert(
            "outer".to_string(),
            Layout {
                source: "[OUTER:{{ content }}]".to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();
        let fm = FrontMatter::new();
        let site = Object::new();

        let output = engine.render("inner", "Hello", &fm, &site).unwrap();
        assert_eq!(output, "[OUTER:[INNER:Hello]]");
    }

    // ========================================================================
    // Unit: Layout wrapping with includes
    // ========================================================================

    #[test]
    fn test_layout_with_include() {
        let mut layouts = HashMap::new();
        layouts.insert(
            "test".to_string(),
            Layout {
                source: "{% include header.html %}{{ content }}{% include footer.html %}"
                    .to_string(),
                parent_layout: None,
            },
        );
        let mut includes = HashMap::new();
        includes.insert("header.html".to_string(), "<header>H</header>".to_string());
        includes.insert("footer.html".to_string(), "<footer>F</footer>".to_string());

        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();
        let fm = FrontMatter::new();
        let site = Object::new();

        let output = engine.render("test", "BODY", &fm, &site).unwrap();
        assert_eq!(output, "<header>H</header>BODY<footer>F</footer>");
    }

    // ========================================================================
    // Integration: LayoutEngine with real site files
    // ========================================================================

    #[test]
    fn test_layout_engine_new_with_real_dirs() {
        let engine = LayoutEngine::new(&layouts_dir(), &includes_dir());
        assert!(
            engine.is_ok(),
            "Failed to create LayoutEngine: {:?}",
            engine.err()
        );
        let engine = engine.unwrap();
        assert_eq!(engine.layout_names().len(), 6);
    }

    #[test]
    fn test_render_home_layout_with_simple_content() {
        let engine = LayoutEngine::new(&layouts_dir(), &includes_dir()).unwrap();
        let mut fm = FrontMatter::new();
        fm.insert(
            "title".to_string(),
            serde_yaml::Value::String("Welcome".to_string()),
        );

        let mut site = Object::new();
        site.insert("name".into(), LiquidValue::scalar("DataTalks.Club"));
        site.insert("url".into(), LiquidValue::scalar("https://datatalks.club"));
        site.insert("twitter".into(), LiquidValue::scalar("@DataTalksClub"));

        // Provide minimal data for header/footer
        let mut data = Object::new();
        let mut nav = Object::new();
        nav.insert("top".into(), LiquidValue::Array(vec![]));
        nav.insert("bottom".into(), LiquidValue::Array(vec![]));
        data.insert("navigation".into(), LiquidValue::Object(nav));
        data.insert("header".into(), LiquidValue::Object(Object::new()));
        site.insert("data".into(), LiquidValue::Object(data));

        let mut github = Object::new();
        github.insert(
            "repository_url".into(),
            LiquidValue::scalar("https://github.com/DataTalksClub/datatalksclub.github.io"),
        );
        site.insert("github".into(), LiquidValue::Object(github));

        let output = engine.render("home", "<p>Hello</p>", &fm, &site).unwrap();
        assert!(output.contains("<html"), "Output should contain <html");
        assert!(output.contains("<head>"), "Output should contain <head>");
        assert!(
            output.contains("<p>Hello</p>"),
            "Output should contain the content"
        );
        assert!(
            output.contains("DataTalks.Club"),
            "Output should contain site name in footer"
        );
    }

    #[test]
    fn test_render_page_layout_with_subscribe_include() {
        let engine = LayoutEngine::new(&layouts_dir(), &includes_dir()).unwrap();
        let mut fm = FrontMatter::new();
        fm.insert(
            "title".to_string(),
            serde_yaml::Value::String("Test Page".to_string()),
        );

        let mut site = Object::new();
        site.insert("name".into(), LiquidValue::scalar("DataTalks.Club"));
        site.insert("url".into(), LiquidValue::scalar("https://datatalks.club"));
        site.insert("twitter".into(), LiquidValue::scalar("@DataTalksClub"));
        let mut data = Object::new();
        let mut nav = Object::new();
        nav.insert("top".into(), LiquidValue::Array(vec![]));
        data.insert("navigation".into(), LiquidValue::Object(nav));
        data.insert("header".into(), LiquidValue::Object(Object::new()));
        site.insert("data".into(), LiquidValue::Object(data));
        let mut github = Object::new();
        github.insert(
            "repository_url".into(),
            LiquidValue::scalar("https://github.com/DataTalksClub/datatalksclub.github.io"),
        );
        site.insert("github".into(), LiquidValue::Object(github));

        let output = engine
            .render("page", "Page content here", &fm, &site)
            .unwrap();
        assert!(output.contains("Page content here"));
        assert!(
            output.contains("mc-embedded-subscribe-form"),
            "Should contain subscribe form"
        );
    }

    #[test]
    fn test_render_post_layout_with_page_context() {
        let engine = LayoutEngine::new(&layouts_dir(), &includes_dir()).unwrap();
        let mut fm = FrontMatter::new();
        fm.insert(
            "title".to_string(),
            serde_yaml::Value::String("My Blog Post".to_string()),
        );
        fm.insert(
            "date".to_string(),
            serde_yaml::Value::String("2024-01-15".to_string()),
        );
        fm.insert(
            "description".to_string(),
            serde_yaml::Value::String("A test post".to_string()),
        );
        fm.insert(
            "authors".to_string(),
            serde_yaml::Value::Sequence(vec![serde_yaml::Value::String("testauthor".to_string())]),
        );
        fm.insert(
            "url".to_string(),
            serde_yaml::Value::String("/blog/test.html".to_string()),
        );

        let mut site = Object::new();
        site.insert("name".into(), LiquidValue::scalar("DataTalks.Club"));
        site.insert("url".into(), LiquidValue::scalar("https://datatalks.club"));
        site.insert("twitter".into(), LiquidValue::scalar("@DataTalksClub"));

        // site.people for author lookup -- provide a matching entry
        let mut author_obj = Object::new();
        author_obj.insert("short".into(), LiquidValue::scalar("testauthor"));
        author_obj.insert("title".into(), LiquidValue::scalar("Test Author"));
        site.insert(
            "people".into(),
            LiquidValue::Array(vec![LiquidValue::Object(author_obj)]),
        );
        site.insert("posts".into(), LiquidValue::Array(vec![]));

        let mut data = Object::new();
        let mut nav = Object::new();
        nav.insert("top".into(), LiquidValue::Array(vec![]));
        data.insert("navigation".into(), LiquidValue::Object(nav));
        data.insert("header".into(), LiquidValue::Object(Object::new()));
        site.insert("data".into(), LiquidValue::Object(data));
        let mut github = Object::new();
        github.insert(
            "repository_url".into(),
            LiquidValue::scalar("https://github.com/DataTalksClub/datatalksclub.github.io"),
        );
        site.insert("github".into(), LiquidValue::Object(github));

        let output = engine
            .render("post", "<p>Post body content</p>", &fm, &site)
            .unwrap();
        assert!(output.contains("<html"), "Output should contain <html");
        assert!(
            output.contains("My Blog Post"),
            "Output should contain the title"
        );
        assert!(
            output.contains("<p>Post body content</p>"),
            "Output should contain the content"
        );
        assert!(
            output.contains("schema.org"),
            "Output should contain JSON-LD"
        );
        assert!(
            output.contains("mc-embedded-subscribe-form"),
            "Should contain subscribe form"
        );
    }

    #[test]
    fn test_render_author_layout_with_social_links() {
        let engine = LayoutEngine::new(&layouts_dir(), &includes_dir()).unwrap();
        let mut fm = FrontMatter::new();
        fm.insert(
            "title".to_string(),
            serde_yaml::Value::String("Test Author".to_string()),
        );
        fm.insert(
            "short".to_string(),
            serde_yaml::Value::String("testauthor".to_string()),
        );
        fm.insert(
            "picture".to_string(),
            serde_yaml::Value::String("images/authors/test.jpg".to_string()),
        );
        fm.insert(
            "twitter".to_string(),
            serde_yaml::Value::String("testtwitter".to_string()),
        );
        fm.insert(
            "linkedin".to_string(),
            serde_yaml::Value::String("testlinkedin".to_string()),
        );
        fm.insert(
            "github".to_string(),
            serde_yaml::Value::String("testgithub".to_string()),
        );
        fm.insert(
            "url".to_string(),
            serde_yaml::Value::String("/people/testauthor.html".to_string()),
        );

        let mut site = Object::new();
        site.insert("name".into(), LiquidValue::scalar("DataTalks.Club"));
        site.insert("url".into(), LiquidValue::scalar("https://datatalks.club"));
        site.insert("twitter".into(), LiquidValue::scalar("@DataTalksClub"));
        site.insert("posts".into(), LiquidValue::Array(vec![]));
        site.insert("books".into(), LiquidValue::Array(vec![]));

        let mut data = Object::new();
        let mut nav = Object::new();
        nav.insert("top".into(), LiquidValue::Array(vec![]));
        data.insert("navigation".into(), LiquidValue::Object(nav));
        data.insert("header".into(), LiquidValue::Object(Object::new()));
        data.insert("events".into(), LiquidValue::Array(vec![]));
        site.insert("data".into(), LiquidValue::Object(data));
        let mut github = Object::new();
        github.insert(
            "repository_url".into(),
            LiquidValue::scalar("https://github.com/DataTalksClub/datatalksclub.github.io"),
        );
        site.insert("github".into(), LiquidValue::Object(github));

        let output = engine
            .render("author", "<p>Author bio</p>", &fm, &site)
            .unwrap();
        assert!(output.contains("Test Author"), "Should contain author name");
        assert!(
            output.contains("testtwitter"),
            "Should contain twitter handle"
        );
        assert!(
            output.contains("testlinkedin"),
            "Should contain linkedin handle"
        );
        assert!(
            output.contains("testgithub"),
            "Should contain github handle"
        );
        assert!(
            output.contains("<p>Author bio</p>"),
            "Should contain author bio content"
        );
    }

    #[test]
    fn test_render_book_layout_with_authors_include() {
        let engine = LayoutEngine::new(&layouts_dir(), &includes_dir()).unwrap();
        let mut fm = FrontMatter::new();
        fm.insert(
            "title".to_string(),
            serde_yaml::Value::String("Test Book".to_string()),
        );
        fm.insert(
            "start".to_string(),
            serde_yaml::Value::String("2024-01-01".to_string()),
        );
        fm.insert(
            "end".to_string(),
            serde_yaml::Value::String("2024-01-05".to_string()),
        );
        fm.insert(
            "cover".to_string(),
            serde_yaml::Value::String("images/cover.jpg".to_string()),
        );
        fm.insert(
            "authors".to_string(),
            serde_yaml::Value::Sequence(vec![serde_yaml::Value::String("testauthor".to_string())]),
        );
        fm.insert(
            "links".to_string(),
            serde_yaml::Value::Sequence(vec![{
                let mut link = serde_yaml::Mapping::new();
                link.insert(
                    serde_yaml::Value::String("text".to_string()),
                    serde_yaml::Value::String("Buy".to_string()),
                );
                link.insert(
                    serde_yaml::Value::String("link".to_string()),
                    serde_yaml::Value::String("https://example.com".to_string()),
                );
                serde_yaml::Value::Mapping(link)
            }]),
        );

        let mut site = Object::new();
        site.insert("name".into(), LiquidValue::scalar("DataTalks.Club"));
        site.insert("url".into(), LiquidValue::scalar("https://datatalks.club"));
        site.insert("twitter".into(), LiquidValue::scalar("@DataTalksClub"));

        // Provide a people collection with testauthor for author lookup
        let mut author_obj = Object::new();
        author_obj.insert("short".into(), LiquidValue::scalar("testauthor"));
        author_obj.insert("title".into(), LiquidValue::scalar("Test Author Name"));
        site.insert(
            "people".into(),
            LiquidValue::Array(vec![LiquidValue::Object(author_obj)]),
        );

        let mut data = Object::new();
        let mut nav = Object::new();
        nav.insert("top".into(), LiquidValue::Array(vec![]));
        data.insert("navigation".into(), LiquidValue::Object(nav));
        data.insert("header".into(), LiquidValue::Object(Object::new()));
        site.insert("data".into(), LiquidValue::Object(data));
        let mut github = Object::new();
        github.insert(
            "repository_url".into(),
            LiquidValue::scalar("https://github.com/DataTalksClub/datatalksclub.github.io"),
        );
        site.insert("github".into(), LiquidValue::Object(github));

        let output = engine
            .render("book", "<p>Book description</p>", &fm, &site)
            .unwrap();
        assert!(output.contains("Test Book"), "Should contain book title");
        assert!(
            output.contains("Test Author Name"),
            "Should contain resolved author name"
        );
        assert!(
            output.contains("<p>Book description</p>"),
            "Should contain book content"
        );
        assert!(
            output.contains("https://example.com"),
            "Should contain link URL"
        );
    }

    #[test]
    fn test_render_page_output_no_unresolved_tags() {
        let engine = LayoutEngine::new(&layouts_dir(), &includes_dir()).unwrap();
        let mut fm = FrontMatter::new();
        fm.insert(
            "title".to_string(),
            serde_yaml::Value::String("Test".to_string()),
        );

        let mut site = Object::new();
        site.insert("name".into(), LiquidValue::scalar("DataTalks.Club"));
        site.insert("url".into(), LiquidValue::scalar("https://datatalks.club"));
        site.insert("twitter".into(), LiquidValue::scalar("@DataTalksClub"));
        let mut data = Object::new();
        let mut nav = Object::new();
        nav.insert("top".into(), LiquidValue::Array(vec![]));
        data.insert("navigation".into(), LiquidValue::Object(nav));
        data.insert("header".into(), LiquidValue::Object(Object::new()));
        site.insert("data".into(), LiquidValue::Object(data));
        let mut github = Object::new();
        github.insert(
            "repository_url".into(),
            LiquidValue::scalar("https://github.com/DataTalksClub/datatalksclub.github.io"),
        );
        site.insert("github".into(), LiquidValue::Object(github));

        let output = engine.render("home", "Content", &fm, &site).unwrap();
        // No unresolved Liquid tags should remain in the output
        assert!(
            !output.contains("{%"),
            "Output should not contain unresolved {{% tags: {}",
            &output[..output.len().min(500)]
        );
        assert!(
            !output.contains("{{"),
            "Output should not contain unresolved {{{{ variables: {}",
            &output[..output.len().min(500)]
        );
    }

    // ========================================================================
    // Integration: render_page (full pipeline)
    // ========================================================================

    #[test]
    fn test_render_page_full_pipeline() {
        let mut layouts = HashMap::new();
        layouts.insert(
            "test".to_string(),
            Layout {
                source: "<title>{{ page.title }}</title><body>{{ content }}</body>".to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();

        let mut fm = FrontMatter::new();
        fm.insert(
            "title".to_string(),
            serde_yaml::Value::String("Hello".to_string()),
        );
        let site = Object::new();

        // Raw content with a Liquid variable
        let output = engine
            .render_page("test", "Welcome {{ page.title }}", &fm, &site)
            .unwrap();
        assert_eq!(output, "<title>Hello</title><body>Welcome Hello</body>");
    }

    #[test]
    fn test_include_param_with_special_chars() {
        let mut includes = HashMap::new();
        includes.insert("test.html".to_string(), "val={{ include.msg }}".to_string());
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let output = engine
            .parse_and_render(r#"{% include test.html msg="hello & goodbye" %}"#, &ctx)
            .unwrap();
        assert_eq!(output, "val=hello & goodbye");
    }

    // ========================================================================
    // build_render_context
    // ========================================================================

    #[test]
    fn test_build_render_context() {
        let mut fm = FrontMatter::new();
        fm.insert(
            "title".to_string(),
            serde_yaml::Value::String("Test".to_string()),
        );

        let mut site = Object::new();
        site.insert("url".into(), LiquidValue::scalar("https://example.com"));

        let ctx = build_render_context("Hello", &fm, &site);

        // Check page
        if let Some(LiquidValue::Object(page)) = ctx.get("page") {
            assert_eq!(page.get("title"), Some(&LiquidValue::scalar("Test")));
        } else {
            panic!("Expected page object");
        }

        // Check site
        if let Some(LiquidValue::Object(site_obj)) = ctx.get("site") {
            assert_eq!(
                site_obj.get("url"),
                Some(&LiquidValue::scalar("https://example.com"))
            );
        } else {
            panic!("Expected site object");
        }

        // Check content
        assert_eq!(ctx.get("content"), Some(&LiquidValue::scalar("Hello")));
    }

    // ========================================================================
    // Issue 26: Include parameters -- numeric values
    // ========================================================================

    #[test]
    fn test_include_numeric_param_renders() {
        let mut includes = HashMap::new();
        includes.insert(
            "counter.html".to_string(),
            "{{ include.max_posts }}".to_string(),
        );
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let output = engine
            .parse_and_render("{% include counter.html max_posts=5 %}", &ctx)
            .unwrap();
        assert_eq!(output, "5");
    }

    #[test]
    fn test_include_numeric_param_not_nil() {
        let mut includes = HashMap::new();
        includes.insert(
            "counter.html".to_string(),
            "{% assign x = include.max_posts | default: 3 %}{{ x }}".to_string(),
        );
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let output = engine
            .parse_and_render("{% include counter.html max_posts=5 %}", &ctx)
            .unwrap();
        // Value should be 5 (not default 3), confirming it's not nil
        assert_eq!(output, "5");
    }

    // ========================================================================
    // Issue 26: Include parameters -- boolean values
    // ========================================================================

    #[test]
    fn test_include_boolean_true_param() {
        let mut includes = HashMap::new();
        includes.insert(
            "toggle.html".to_string(),
            "{% if include.show %}YES{% endif %}".to_string(),
        );
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let output = engine
            .parse_and_render("{% include toggle.html show=true %}", &ctx)
            .unwrap();
        assert_eq!(output, "YES");
    }

    #[test]
    fn test_include_boolean_false_param() {
        let mut includes = HashMap::new();
        includes.insert(
            "toggle.html".to_string(),
            "{% if include.show %}YES{% else %}NO{% endif %}".to_string(),
        );
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let output = engine
            .parse_and_render("{% include toggle.html show=false %}", &ctx)
            .unwrap();
        assert_eq!(output, "NO");
    }

    // ========================================================================
    // Issue 26: Include parameters -- bracket notation
    // ========================================================================

    #[test]
    fn test_include_bracket_notation_numeric() {
        let mut includes = HashMap::new();
        includes.insert(
            "data.html".to_string(),
            r#"{{ include["count"] }}"#.to_string(),
        );
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let output = engine
            .parse_and_render("{% include data.html count=5 %}", &ctx)
            .unwrap();
        assert_eq!(output, "5");
    }

    #[test]
    fn test_include_bracket_notation_string() {
        let mut includes = HashMap::new();
        includes.insert(
            "data.html".to_string(),
            r#"{{ include["name"] }}"#.to_string(),
        );
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let output = engine
            .parse_and_render(r#"{% include data.html name="test" %}"#, &ctx)
            .unwrap();
        assert_eq!(output, "test");
    }

    #[test]
    fn test_include_bracket_notation_boolean() {
        let mut includes = HashMap::new();
        includes.insert(
            "data.html".to_string(),
            r#"{% if include["flag"] %}OK{% endif %}"#.to_string(),
        );
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let output = engine
            .parse_and_render("{% include data.html flag=true %}", &ctx)
            .unwrap();
        assert_eq!(output, "OK");
    }

    // ========================================================================
    // Issue 26: Include parameters -- missing params (lenient)
    // ========================================================================

    #[test]
    fn test_include_missing_param_renders_empty() {
        let mut includes = HashMap::new();
        includes.insert(
            "simple.html".to_string(),
            "{{ include.missing_param }}".to_string(),
        );
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let output = engine
            .parse_and_render("{% include simple.html %}", &ctx)
            .unwrap();
        assert_eq!(output, "");
    }

    #[test]
    fn test_include_missing_param_default_filter() {
        let mut includes = HashMap::new();
        includes.insert(
            "simple.html".to_string(),
            "{% assign x = include.max | default: 3 %}{{ x }}".to_string(),
        );
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let output = engine
            .parse_and_render("{% include simple.html %}", &ctx)
            .unwrap();
        assert_eq!(output, "3");
    }

    // ========================================================================
    // Issue 26: Include parameters -- multiple params
    // ========================================================================

    #[test]
    fn test_include_multiple_params_dot_notation() {
        let mut includes = HashMap::new();
        includes.insert(
            "card.html".to_string(),
            "{{ include.title }}-{{ include.count }}-{% if include.show %}on{% endif %}"
                .to_string(),
        );
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let output = engine
            .parse_and_render(
                r#"{% include card.html title="Hello" count=3 show=true %}"#,
                &ctx,
            )
            .unwrap();
        assert_eq!(output, "Hello-3-on");
    }

    #[test]
    fn test_include_multiple_params_bracket_notation() {
        let mut includes = HashMap::new();
        includes.insert(
            "card.html".to_string(),
            r#"{{ include["title"] }}-{{ include["count"] }}-{% if include["show"] %}on{% endif %}"#.to_string(),
        );
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let output = engine
            .parse_and_render(
                r#"{% include card.html title="Hello" count=3 show=true %}"#,
                &ctx,
            )
            .unwrap();
        assert_eq!(output, "Hello-3-on");
    }

    // ========================================================================
    // Issue 26: Nested includes with parameter forwarding
    // ========================================================================

    #[test]
    fn test_nested_include_param_forwarding() {
        let mut includes = HashMap::new();
        includes.insert(
            "inner.html".to_string(),
            "inner={{ include.x }}".to_string(),
        );
        includes.insert(
            "outer.html".to_string(),
            "outer[{% include inner.html x=include.x %}]".to_string(),
        );
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let output = engine
            .parse_and_render(r#"{% include outer.html x="hello" %}"#, &ctx)
            .unwrap();
        assert_eq!(output, "outer[inner=hello]");
    }

    // ========================================================================
    // Issue 39: Subdirectory include paths
    // ========================================================================

    #[test]
    fn test_include_subdirectory_path() {
        let mut includes = HashMap::new();
        includes.insert("subdir/file.html".to_string(), "SUBDIR_CONTENT".to_string());
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let output = engine
            .parse_and_render("{% include subdir/file.html %}", &ctx)
            .unwrap();
        assert_eq!(output, "SUBDIR_CONTENT");
    }

    #[test]
    fn test_include_deeply_nested_path() {
        let mut includes = HashMap::new();
        includes.insert("a/b/c.html".to_string(), "DEEP_CONTENT".to_string());
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let output = engine
            .parse_and_render("{% include a/b/c.html %}", &ctx)
            .unwrap();
        assert_eq!(output, "DEEP_CONTENT");
    }

    #[test]
    fn test_include_subdirectory_with_param() {
        let mut includes = HashMap::new();
        includes.insert(
            "subdir/file.html".to_string(),
            "hello={{ include.param }}".to_string(),
        );
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let output = engine
            .parse_and_render(r#"{% include subdir/file.html param="world" %}"#, &ctx)
            .unwrap();
        assert_eq!(output, "hello=world");
    }

    #[test]
    fn test_include_subdirectory_missing_param_returns_nil() {
        let mut includes = HashMap::new();
        includes.insert(
            "subdir/file.html".to_string(),
            "val={{ include.nonexistent }}".to_string(),
        );
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let output = engine
            .parse_and_render("{% include subdir/file.html %}", &ctx)
            .unwrap();
        assert_eq!(output, "val=");
    }

    #[test]
    fn test_include_simple_still_works_after_subdirectory_support() {
        let mut includes = HashMap::new();
        includes.insert("simple.html".to_string(), "SIMPLE".to_string());
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let output = engine
            .parse_and_render("{% include simple.html %}", &ctx)
            .unwrap();
        assert_eq!(output, "SIMPLE");
    }

    // ========================================================================
    // Issue 41: Dynamic include paths
    // ========================================================================

    #[test]
    fn test_dynamic_include_simple_variable() {
        let mut includes = HashMap::new();
        includes.insert("contact.html".to_string(), "<p>Contact</p>".to_string());
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();

        let mut page = liquid::Object::new();
        page.insert("form".into(), LiquidValue::scalar("contact.html"));
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page));

        let output = engine
            .parse_and_render(r#"{% include {{ page.form }} %}"#, &ctx)
            .unwrap();
        assert!(
            output.contains("<p>Contact</p>"),
            "Expected contact content, got: {}",
            output
        );
    }

    #[test]
    fn test_dynamic_include_with_filter() {
        let mut includes = HashMap::new();
        includes.insert("survey.html".to_string(), "<p>Survey</p>".to_string());
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();

        let mut page = liquid::Object::new();
        page.insert("form".into(), LiquidValue::scalar("survey"));
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page));

        let output = engine
            .parse_and_render(r#"{% include {{ page.form | append: '.html' }} %}"#, &ctx)
            .unwrap();
        assert!(
            output.contains("<p>Survey</p>"),
            "Expected survey content, got: {}",
            output
        );
    }

    #[test]
    fn test_dynamic_include_nil_path_returns_error() {
        let mut includes = HashMap::new();
        includes.insert("dummy.html".to_string(), "DUMMY".to_string());
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();

        // page.form is not set, so it should be nil
        let mut page = liquid::Object::new();
        page.insert("other".into(), LiquidValue::scalar("irrelevant"));
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page));

        let result = engine.parse_and_render(r#"{% include {{ page.form }} %}"#, &ctx);
        assert!(
            result.is_err(),
            "Expected error for nil dynamic include path"
        );
    }

    #[test]
    fn test_dynamic_include_nonexistent_partial_returns_error() {
        let mut includes = HashMap::new();
        includes.insert("exists.html".to_string(), "EXISTS".to_string());
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();

        let mut page = liquid::Object::new();
        page.insert("form".into(), LiquidValue::scalar("nonexistent.html"));
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page));

        let result = engine.parse_and_render(r#"{% include {{ page.form }} %}"#, &ctx);
        assert!(result.is_err(), "Expected error for nonexistent partial");
    }

    #[test]
    fn test_dynamic_include_with_params() {
        let mut includes = HashMap::new();
        includes.insert(
            "form.html".to_string(),
            "<form>{{ include.action }}</form>".to_string(),
        );
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();

        let mut page = liquid::Object::new();
        page.insert("form".into(), LiquidValue::scalar("form.html"));
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page));

        let output = engine
            .parse_and_render(r#"{% include {{ page.form }} action="submit" %}"#, &ctx)
            .unwrap();
        assert!(
            output.contains("<form>submit</form>"),
            "Expected form with action, got: {}",
            output
        );
    }

    #[test]
    fn test_static_include_still_works_after_dynamic_support() {
        // Verify literal includes are unaffected by dynamic include support
        let mut includes = HashMap::new();
        includes.insert("header.html".to_string(), "<h1>Header</h1>".to_string());
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let output = engine
            .parse_and_render("{% include header.html %}", &ctx)
            .unwrap();
        assert_eq!(output, "<h1>Header</h1>");
    }
}
