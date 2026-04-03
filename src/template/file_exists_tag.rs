//! `{% file_exists path %}` tag.
//!
//! Checks if a file exists relative to the site source directory and outputs
//! `"true"` or `"false"`. Matches the al-folio Jekyll plugin
//! (`_plugins/file-exists.rb`).

use std::io::Write;
use std::path::PathBuf;

use liquid_core::{
    Language, ParseTag, Renderable, Runtime, TagReflection, TagTokenIter, ValueView,
};

/// The `{% file_exists %}` tag.
#[derive(Copy, Clone, Debug, Default)]
pub struct FileExistsTag;

impl TagReflection for FileExistsTag {
    fn tag(&self) -> &'static str {
        "file_exists"
    }

    fn description(&self) -> &'static str {
        "Check if a file exists relative to site source"
    }
}

impl ParseTag for FileExistsTag {
    fn parse(
        &self,
        mut arguments: TagTokenIter<'_>,
        _options: &Language,
    ) -> liquid_core::Result<Box<dyn Renderable>> {
        // Collect all argument tokens as the file path
        let mut path_parts = Vec::new();
        while let Ok(token) = arguments.expect_next("") {
            path_parts.push(token.as_str().to_string());
        }
        let path = path_parts.join(" ").trim().to_string();

        Ok(Box::new(FileExistsRenderable { path }))
    }

    fn reflection(&self) -> &dyn TagReflection {
        self
    }
}

#[derive(Debug)]
struct FileExistsRenderable {
    path: String,
}

impl std::fmt::Display for FileExistsRenderable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "file_exists")
    }
}

impl Renderable for FileExistsRenderable {
    fn render_to(&self, writer: &mut dyn Write, runtime: &dyn Runtime) -> liquid_core::Result<()> {
        if self.path.is_empty() {
            write!(writer, "false")
                .map_err(|e| liquid_core::Error::with_msg(format!("Write error: {}", e)))?;
            return Ok(());
        }

        // Get the site source directory from the context
        let source = get_site_source(runtime);
        let file_path = if let Some(src) = source {
            let mut p = PathBuf::from(src);
            p.push(self.path.trim_start_matches('/'));
            p
        } else {
            PathBuf::from(self.path.trim_start_matches('/'))
        };

        let exists = file_path.exists();
        write!(writer, "{}", exists)
            .map_err(|e| liquid_core::Error::with_msg(format!("Write error: {}", e)))?;

        Ok(())
    }
}

/// Extract `site.source` from the Liquid runtime context.
fn get_site_source(runtime: &dyn Runtime) -> Option<String> {
    let path: Vec<liquid_core::model::ScalarCow<'_>> = vec![
        liquid_core::model::ScalarCow::new("site"),
        liquid_core::model::ScalarCow::new("source"),
    ];
    let val = runtime.try_get(&path)?;
    let s = val.to_kstr().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::super::engine::TemplateEngine;
    use liquid::model::Value;
    use liquid::Object;

    fn engine() -> TemplateEngine {
        TemplateEngine::new().unwrap()
    }

    #[test]
    fn test_file_exists_true() {
        let eng = engine();
        // Use Cargo.toml as a file that definitely exists
        let mut site = Object::new();
        site.insert("source".into(), Value::scalar("."));
        let mut ctx = Object::new();
        ctx.insert("site".into(), Value::Object(site));

        let out = eng
            .parse_and_render("{% file_exists Cargo.toml %}", &ctx)
            .unwrap();
        assert_eq!(out, "true", "Cargo.toml should exist: {}", out);
    }

    #[test]
    fn test_file_exists_false() {
        let eng = engine();
        let mut site = Object::new();
        site.insert("source".into(), Value::scalar("."));
        let mut ctx = Object::new();
        ctx.insert("site".into(), Value::Object(site));

        let out = eng
            .parse_and_render("{% file_exists nonexistent_file_xyz.txt %}", &ctx)
            .unwrap();
        assert_eq!(out, "false", "Nonexistent file should be false: {}", out);
    }

    #[test]
    fn test_file_exists_empty_path() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng.parse_and_render("{% file_exists %}", &ctx).unwrap();
        assert_eq!(out, "false", "Empty path should return false: {}", out);
    }

    #[test]
    fn test_file_exists_no_site_source() {
        // Without site.source, should still work using current directory
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render("{% file_exists Cargo.toml %}", &ctx)
            .unwrap();
        assert_eq!(
            out, "true",
            "Should find Cargo.toml from current directory: {}",
            out
        );
    }

    #[test]
    fn test_file_exists_in_capture() {
        // Test the common al-folio pattern: {% capture x %}{% file_exists path %}{% endcapture %}
        let eng = engine();
        let mut site = Object::new();
        site.insert("source".into(), Value::scalar("."));
        let mut ctx = Object::new();
        ctx.insert("site".into(), Value::Object(site));

        let out = eng
            .parse_and_render(
                r#"{% capture exists %}{% file_exists Cargo.toml %}{% endcapture %}{% if exists == "true" %}found{% else %}missing{% endif %}"#,
                &ctx,
            )
            .unwrap();
        assert_eq!(out, "found", "capture + file_exists pattern: {}", out);
    }
}
