//! Static file server for local development.
//!
//! Serves files from a directory with correct Content-Type headers,
//! directory index fallback, and clean URL support.

use std::path::{Path, PathBuf};

/// Determine the MIME content type for a file based on its extension.
pub fn content_type_for_extension(ext: &str) -> &'static str {
    match ext.to_lowercase().as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "xml" => "application/xml; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "eot" => "application/vnd.ms-fontobject",
        "txt" => "text/plain; charset=utf-8",
        "webp" => "image/webp",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

/// Resolve a URL path to a file path within the root directory.
///
/// Returns `None` if the path escapes the root (directory traversal) or
/// no matching file is found.
///
/// Resolution order:
/// 1. Exact file match
/// 2. If path ends with `/`, try `<path>/index.html`
/// 3. Try `<path>/index.html` (directory index)
/// 4. Try `<path>.html` (clean URL)
pub fn resolve_file_path(root: &Path, url_path: &str) -> Option<PathBuf> {
    // Normalize the URL path: strip leading slash, decode basics
    let clean = url_path.trim_start_matches('/');

    // Reject directory traversal attempts
    if clean.contains("..") {
        return None;
    }

    let candidate = if clean.is_empty() {
        root.join("index.html")
    } else {
        root.join(clean)
    };

    // Ensure the resolved path is actually within root
    if let Ok(canonical_root) = root.canonicalize() {
        // For exact file match or directory index, check containment
        if candidate.exists() {
            if let Ok(canonical_candidate) = candidate.canonicalize() {
                if !canonical_candidate.starts_with(&canonical_root) {
                    return None;
                }
                if canonical_candidate.is_file() {
                    return Some(canonical_candidate);
                }
            }
        }
    }

    // If path ends with `/`, try index.html
    if url_path.ends_with('/') || clean.is_empty() {
        let index = if clean.is_empty() {
            root.join("index.html")
        } else {
            root.join(clean).join("index.html")
        };
        if index.is_file() {
            return Some(index);
        }
        return None;
    }

    // Try <path>/index.html (directory with clean URL)
    let dir_index = root.join(clean).join("index.html");
    if dir_index.is_file() {
        return Some(dir_index);
    }

    // Try <path>.html (clean URL fallback)
    let html_path = root.join(format!("{}.html", clean));
    if html_path.is_file() {
        return Some(html_path);
    }

    None
}

/// Get the file extension from a path, lowercased.
pub fn get_extension(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
}

/// Start an HTTP server serving static files from `root` on the given `port`.
///
/// This function blocks until the server is shut down (e.g., via Ctrl+C).
/// If `livereload_port` is `Some`, a live-reload script is injected into
/// HTML responses pointing to the given WebSocket port.
pub fn start_server(
    root: &Path,
    port: u16,
    livereload_port: Option<u16>,
) -> Result<(), std::io::Error> {
    let addr = format!("0.0.0.0:{}", port);
    let server = tiny_http::Server::http(&addr)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::AddrInUse, e.to_string()))?;

    println!("Serving at http://127.0.0.1:{}", port);

    for request in server.incoming_requests() {
        let url_path = request.url().to_string();
        // Strip query string if present
        let url_path = url_path.split('?').next().unwrap_or(&url_path);

        match resolve_file_path(root, url_path) {
            Some(file_path) => {
                let ext = get_extension(&file_path);
                let content_type = content_type_for_extension(&ext);
                let is_html = ext == "html" || ext == "htm";

                if let (true, Some(lr_port)) = (is_html, livereload_port) {
                    // Read file, inject livereload script, serve modified content
                    match std::fs::read_to_string(&file_path) {
                        Ok(html) => {
                            let modified =
                                crate::livereload::inject_livereload_script(&html, lr_port);
                            let header = tiny_http::Header::from_bytes(
                                &b"Content-Type"[..],
                                content_type.as_bytes(),
                            )
                            .unwrap();
                            let response =
                                tiny_http::Response::from_string(modified).with_header(header);
                            let _ = request.respond(response);
                        }
                        Err(_) => {
                            let response =
                                tiny_http::Response::from_string("500 Internal Server Error")
                                    .with_status_code(500);
                            let _ = request.respond(response);
                        }
                    }
                } else {
                    // Serve the file directly
                    match std::fs::File::open(&file_path) {
                        Ok(file) => {
                            let header = tiny_http::Header::from_bytes(
                                &b"Content-Type"[..],
                                content_type.as_bytes(),
                            )
                            .unwrap();
                            let response = tiny_http::Response::from_file(file).with_header(header);
                            let _ = request.respond(response);
                        }
                        Err(_) => {
                            let response = tiny_http::Response::from_string("404 Not Found")
                                .with_status_code(404);
                            let _ = request.respond(response);
                        }
                    }
                }
            }
            None => {
                let response =
                    tiny_http::Response::from_string("404 Not Found").with_status_code(404);
                let _ = request.respond(response);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // --- Content-Type tests ---

    #[test]
    fn test_content_type_html() {
        assert_eq!(
            content_type_for_extension("html"),
            "text/html; charset=utf-8"
        );
    }

    #[test]
    fn test_content_type_css() {
        assert_eq!(content_type_for_extension("css"), "text/css; charset=utf-8");
    }

    #[test]
    fn test_content_type_js() {
        assert_eq!(
            content_type_for_extension("js"),
            "application/javascript; charset=utf-8"
        );
    }

    #[test]
    fn test_content_type_json() {
        assert_eq!(
            content_type_for_extension("json"),
            "application/json; charset=utf-8"
        );
    }

    #[test]
    fn test_content_type_xml() {
        assert_eq!(
            content_type_for_extension("xml"),
            "application/xml; charset=utf-8"
        );
    }

    #[test]
    fn test_content_type_png() {
        assert_eq!(content_type_for_extension("png"), "image/png");
    }

    #[test]
    fn test_content_type_jpg() {
        assert_eq!(content_type_for_extension("jpg"), "image/jpeg");
    }

    #[test]
    fn test_content_type_jpeg() {
        assert_eq!(content_type_for_extension("jpeg"), "image/jpeg");
    }

    #[test]
    fn test_content_type_svg() {
        assert_eq!(content_type_for_extension("svg"), "image/svg+xml");
    }

    #[test]
    fn test_content_type_unknown() {
        assert_eq!(
            content_type_for_extension("xyz"),
            "application/octet-stream"
        );
    }

    #[test]
    fn test_content_type_case_insensitive() {
        assert_eq!(
            content_type_for_extension("HTML"),
            "text/html; charset=utf-8"
        );
        assert_eq!(content_type_for_extension("CSS"), "text/css; charset=utf-8");
    }

    // --- File resolution tests ---

    #[test]
    fn test_resolve_root_index() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("index.html"), "<html></html>").unwrap();
        let result = resolve_file_path(dir.path(), "/");
        assert!(result.is_some());
        assert!(result.unwrap().ends_with("index.html"));
    }

    #[test]
    fn test_resolve_exact_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("style.css"), "body {}").unwrap();
        let result = resolve_file_path(dir.path(), "/style.css");
        assert!(result.is_some());
        assert!(result.unwrap().ends_with("style.css"));
    }

    #[test]
    fn test_resolve_nonexistent_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let result = resolve_file_path(dir.path(), "/nonexistent.html");
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_directory_index() {
        let dir = tempfile::tempdir().unwrap();
        let blog_dir = dir.path().join("blog");
        fs::create_dir_all(&blog_dir).unwrap();
        fs::write(blog_dir.join("index.html"), "<html>blog</html>").unwrap();
        let result = resolve_file_path(dir.path(), "/blog/");
        assert!(result.is_some());
        assert!(result.unwrap().ends_with("index.html"));
    }

    #[test]
    fn test_resolve_clean_url_with_html_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("about.html"), "<html>about</html>").unwrap();
        let result = resolve_file_path(dir.path(), "/about");
        assert!(result.is_some());
        assert!(result.unwrap().ends_with("about.html"));
    }

    #[test]
    fn test_resolve_clean_url_with_directory_index() {
        let dir = tempfile::tempdir().unwrap();
        let about_dir = dir.path().join("about");
        fs::create_dir_all(&about_dir).unwrap();
        fs::write(about_dir.join("index.html"), "<html>about</html>").unwrap();
        let result = resolve_file_path(dir.path(), "/about");
        assert!(result.is_some());
        assert!(result.unwrap().ends_with("index.html"));
    }

    #[test]
    fn test_resolve_directory_traversal_rejected() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("secret.txt"), "secret").unwrap();
        let result = resolve_file_path(dir.path(), "/../secret.txt");
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_double_dot_in_path_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let result = resolve_file_path(dir.path(), "/foo/../../../etc/passwd");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_extension() {
        assert_eq!(get_extension(Path::new("file.html")), "html");
        assert_eq!(get_extension(Path::new("file.CSS")), "css");
        assert_eq!(get_extension(Path::new("noext")), "");
    }
}
