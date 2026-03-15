//! Live reload support: script injection, WebSocket server, and file watching.

use std::path::{Path, PathBuf};
use std::time::Instant;

/// The live-reload script template. The `{}` placeholder is replaced with
/// the WebSocket URL.
const LIVERELOAD_SCRIPT: &str = r#"<script>
(function() {
  var ws = new WebSocket("ws://127.0.0.1:{}/__livereload");
  ws.onmessage = function(msg) {
    if (msg.data === "reload") {
      location.reload();
    }
  };
  ws.onclose = function() {
    // Try to reconnect after a delay
    setTimeout(function() { location.reload(); }, 2000);
  };
})();
</script>"#;

/// Inject the live-reload script into an HTML string.
///
/// The script is inserted just before `</body>` if present. If `</body>`
/// is not found, the script is appended to the end of the HTML.
pub fn inject_livereload_script(html: &str, ws_port: u16) -> String {
    let script = LIVERELOAD_SCRIPT.replace("{}", &ws_port.to_string());

    if let Some(pos) = html.to_lowercase().rfind("</body>") {
        let mut result = String::with_capacity(html.len() + script.len());
        result.push_str(&html[..pos]);
        result.push_str(&script);
        result.push('\n');
        result.push_str(&html[pos..]);
        result
    } else {
        let mut result = String::with_capacity(html.len() + script.len());
        result.push_str(html);
        result.push_str(&script);
        result
    }
}

/// Check whether a file path should trigger a rebuild.
///
/// Returns `true` for content files that are part of the site source.
pub fn should_watch_file(path: &Path) -> bool {
    let path_str = path.to_string_lossy();

    // Ignore common non-content files/directories
    if path_str.contains(".git/")
        || path_str.contains("node_modules/")
        || path_str.contains(".DS_Store")
    {
        return false;
    }

    // Ignore editor swap/backup files
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if name.ends_with(".swp")
            || name.ends_with(".swo")
            || name.ends_with('~')
            || name.starts_with('.')
        {
            return false;
        }
    }

    // Watch content files
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        matches!(
            ext.to_lowercase().as_str(),
            "md" | "html"
                | "htm"
                | "yml"
                | "yaml"
                | "css"
                | "js"
                | "json"
                | "xml"
                | "txt"
                | "png"
                | "jpg"
                | "jpeg"
                | "gif"
                | "svg"
                | "webp"
                | "ico"
                | "woff"
                | "woff2"
                | "ttf"
                | "eot"
        )
    } else {
        false
    }
}

/// Check whether a file path is within the destination directory.
pub fn is_in_destination(path: &Path, destination: &Path) -> bool {
    // Try canonical comparison first, fall back to starts_with
    if let (Ok(canon_path), Ok(canon_dest)) = (path.canonicalize(), destination.canonicalize()) {
        canon_path.starts_with(&canon_dest)
    } else {
        path.starts_with(destination)
    }
}

/// Check whether a file path is within the source directory.
///
/// Uses canonicalization for accurate comparison, falling back to
/// `starts_with` if canonicalization fails.
pub fn is_in_source(path: &Path, source: &Path) -> bool {
    if let (Ok(canon_path), Ok(canon_source)) = (path.canonicalize(), source.canonicalize()) {
        canon_path.starts_with(&canon_source)
    } else {
        path.starts_with(source)
    }
}

/// The kind of file that changed, used to determine rebuild scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileChangeKind {
    /// Config file (_config.yml) changed -- requires full rebuild.
    Config,
    /// Layout file (_layouts/*) changed -- requires full rebuild.
    Layout,
    /// Include file (_includes/*) changed -- requires full rebuild.
    Include,
    /// Data file (_data/*) changed -- requires full rebuild (data affects many pages).
    Data,
    /// Content file (post, page, collection item) changed -- can do partial rebuild.
    Content,
    /// Static asset (css, js, image, etc.) that just needs copying.
    StaticAsset,
}

/// What scope of rebuild the watcher requests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RebuildScope {
    /// Full site rebuild (config, layout, include, or data changed).
    Full,
    /// Only rebuild specific content files (relative paths from source).
    Partial(Vec<String>),
}

/// Classify a changed file path relative to the source directory.
///
/// The `rel_path` should be the path relative to the source root (using forward slashes).
pub fn classify_changed_file(rel_path: &str) -> FileChangeKind {
    if rel_path == "_config.yml" || rel_path == "_config.yaml" {
        FileChangeKind::Config
    } else if rel_path.starts_with("_layouts/") || rel_path.starts_with("_layouts\\") {
        FileChangeKind::Layout
    } else if rel_path.starts_with("_includes/") || rel_path.starts_with("_includes\\") {
        FileChangeKind::Include
    } else if rel_path.starts_with("_data/") || rel_path.starts_with("_data\\") {
        FileChangeKind::Data
    } else {
        // Check if it's a static asset (no front matter expected) or content
        let is_content_ext = rel_path.ends_with(".md")
            || rel_path.ends_with(".html")
            || rel_path.ends_with(".htm")
            || rel_path.ends_with(".markdown");
        if is_content_ext {
            FileChangeKind::Content
        } else {
            FileChangeKind::StaticAsset
        }
    }
}

/// Analyze a set of changed file paths and determine the rebuild scope.
///
/// If any file is config, layout, include, or data, returns `RebuildScope::Full`.
/// Otherwise returns `RebuildScope::Partial` with the relative paths of changed content files.
/// Static asset changes also trigger a full rebuild (they need to be re-copied and the
/// current architecture doesn't support partial static file copy).
pub fn determine_rebuild_scope(source: &Path, changed_paths: &[PathBuf]) -> RebuildScope {
    let mut content_paths = Vec::new();
    let mut needs_full = false;

    for path in changed_paths {
        let rel = match path.strip_prefix(source) {
            Ok(r) => r.to_string_lossy().replace('\\', "/"),
            Err(_) => {
                // If we can't make it relative, try canonical
                if let (Ok(canon_path), Ok(canon_source)) =
                    (path.canonicalize(), source.canonicalize())
                {
                    match canon_path.strip_prefix(&canon_source) {
                        Ok(r) => r.to_string_lossy().replace('\\', "/"),
                        Err(_) => continue,
                    }
                } else {
                    continue;
                }
            }
        };

        match classify_changed_file(&rel) {
            FileChangeKind::Config
            | FileChangeKind::Layout
            | FileChangeKind::Include
            | FileChangeKind::Data
            | FileChangeKind::StaticAsset => {
                needs_full = true;
            }
            FileChangeKind::Content => {
                content_paths.push(rel);
            }
        }
    }

    if needs_full || content_paths.is_empty() {
        RebuildScope::Full
    } else {
        RebuildScope::Partial(content_paths)
    }
}

/// Start a WebSocket server on the given port that sends "reload" messages
/// to all connected clients when `reload_rx` receives a signal.
///
/// This function blocks and should be run in a dedicated thread.
pub fn start_websocket_server(
    port: u16,
    reload_rx: std::sync::mpsc::Receiver<()>,
    shutdown: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    let listener = match TcpListener::bind(format!("0.0.0.0:{}", port)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to start WebSocket server on port {}: {}", port, e);
            return;
        }
    };
    listener
        .set_nonblocking(true)
        .expect("Cannot set non-blocking");

    println!(
        "Live reload WebSocket on ws://127.0.0.1:{}/__livereload",
        port
    );

    let clients: Arc<Mutex<Vec<tungstenite::WebSocket<std::net::TcpStream>>>> =
        Arc::new(Mutex::new(Vec::new()));

    let clients_for_accept = Arc::clone(&clients);

    // Spawn a thread to accept new connections
    let shutdown_accept = Arc::clone(&shutdown);
    let accept_handle = std::thread::spawn(move || {
        while !shutdown_accept.load(std::sync::atomic::Ordering::Relaxed) {
            match listener.accept() {
                Ok((stream, _addr)) => match tungstenite::accept(stream) {
                    Ok(ws) => {
                        clients_for_accept.lock().unwrap().push(ws);
                    }
                    Err(e) => {
                        eprintln!("WebSocket handshake error: {}", e);
                    }
                },
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(e) => {
                    eprintln!("WebSocket accept error: {}", e);
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        }
    });

    // Main loop: wait for reload signals and broadcast to clients
    while !shutdown.load(std::sync::atomic::Ordering::Relaxed) {
        match reload_rx.recv_timeout(Duration::from_millis(200)) {
            Ok(()) => {
                let mut locked = clients.lock().unwrap();
                locked.retain_mut(|ws| {
                    use tungstenite::Message;
                    ws.send(Message::Text("reload".to_string())).is_ok()
                });
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let _ = accept_handle.join();
}

/// Start watching the source directory for changes, triggering rebuilds
/// and sending reload signals.
///
/// This function blocks and should be run in a dedicated thread.
///
/// The `build_fn` receives a `RebuildScope` indicating whether a full or
/// partial rebuild is needed, along with the list of changed relative paths.
pub fn start_file_watcher(
    source: PathBuf,
    destination: PathBuf,
    reload_tx: std::sync::mpsc::Sender<()>,
    shutdown: std::sync::Arc<std::sync::atomic::AtomicBool>,
    build_fn: Box<dyn Fn(RebuildScope) -> Result<(), String> + Send>,
) -> Result<(), String> {
    use notify_debouncer_mini::new_debouncer;
    use std::time::Duration;

    let (tx, rx) = std::sync::mpsc::channel();

    let mut debouncer = new_debouncer(Duration::from_millis(300), tx)
        .map_err(|e| format!("Failed to create file watcher: {}", e))?;

    debouncer
        .watcher()
        .watch(&source, notify::RecursiveMode::Recursive)
        .map_err(|e| format!("Failed to watch directory: {}", e))?;

    println!("Watching {} for changes...", source.display());

    while !shutdown.load(std::sync::atomic::Ordering::Relaxed) {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(Ok(events)) => {
                // Filter events: ignore destination dir and non-content files
                let relevant_paths: Vec<PathBuf> = events
                    .iter()
                    .filter(|e| {
                        is_in_source(&e.path, &source)
                            && !is_in_destination(&e.path, &destination)
                            && should_watch_file(&e.path)
                    })
                    .map(|e| e.path.clone())
                    .collect();

                if relevant_paths.is_empty() {
                    continue;
                }

                let scope = determine_rebuild_scope(&source, &relevant_paths);

                let start = Instant::now();
                match &scope {
                    RebuildScope::Full => {
                        println!("File change detected, full rebuild...");
                    }
                    RebuildScope::Partial(paths) => {
                        println!(
                            "File change detected, incremental rebuild ({} file{})...",
                            paths.len(),
                            if paths.len() == 1 { "" } else { "s" }
                        );
                        for p in paths {
                            println!("  changed: {}", p);
                        }
                    }
                }

                match build_fn(scope) {
                    Ok(()) => {
                        let elapsed = start.elapsed();
                        println!("Rebuild complete in {:.0}ms.", elapsed.as_millis());
                        let _ = reload_tx.send(());
                    }
                    Err(e) => {
                        eprintln!("Rebuild failed: {}", e);
                        // Server continues running
                    }
                }
            }
            Ok(Err(errors)) => {
                eprintln!("Watch error: {}", errors);
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Script injection tests ---

    #[test]
    fn test_inject_script_before_body_close() {
        let html = "<html><body><p>Hello</p></body></html>";
        let result = inject_livereload_script(html, 35729);
        assert!(result.contains("ws://127.0.0.1:35729/__livereload"));
        assert!(result.contains("location.reload()"));
        // Script should be before </body>
        let script_pos = result.find("new WebSocket").unwrap();
        let body_close_pos = result.find("</body>").unwrap();
        assert!(script_pos < body_close_pos);
    }

    #[test]
    fn test_inject_script_no_body_close() {
        let html = "<html><p>Hello</p></html>";
        let result = inject_livereload_script(html, 35729);
        assert!(result.contains("ws://127.0.0.1:35729/__livereload"));
        // Script should be appended
        assert!(result.starts_with("<html>"));
        assert!(result.ends_with("</script>"));
    }

    #[test]
    fn test_inject_script_preserves_content() {
        let html = "<html><body><p>Important content</p></body></html>";
        let result = inject_livereload_script(html, 35729);
        assert!(result.contains("Important content"));
        assert!(result.contains("</body></html>"));
    }

    #[test]
    fn test_inject_script_contains_websocket_url() {
        let html = "<html><body></body></html>";
        let result = inject_livereload_script(html, 8080);
        assert!(result.contains("ws://127.0.0.1:8080/__livereload"));
    }

    #[test]
    fn test_inject_script_contains_reload_call() {
        let html = "<html><body></body></html>";
        let result = inject_livereload_script(html, 35729);
        assert!(result.contains("location.reload()"));
    }

    #[test]
    fn test_inject_script_case_insensitive_body_tag() {
        let html = "<html><BODY><p>Hi</p></BODY></html>";
        let result = inject_livereload_script(html, 35729);
        assert!(result.contains("new WebSocket"));
    }

    // --- File watching filter tests ---

    #[test]
    fn test_should_watch_markdown() {
        assert!(should_watch_file(Path::new("content/post.md")));
    }

    #[test]
    fn test_should_watch_html() {
        assert!(should_watch_file(Path::new("_layouts/default.html")));
    }

    #[test]
    fn test_should_watch_yaml() {
        assert!(should_watch_file(Path::new("_data/people.yml")));
    }

    #[test]
    fn test_should_watch_yaml_extension() {
        assert!(should_watch_file(Path::new("_config.yaml")));
    }

    #[test]
    fn test_should_not_watch_git_dir() {
        assert!(!should_watch_file(Path::new(".git/objects/abc123")));
    }

    #[test]
    fn test_should_not_watch_node_modules() {
        assert!(!should_watch_file(Path::new(
            "node_modules/package/index.js"
        )));
    }

    #[test]
    fn test_should_not_watch_swap_file() {
        assert!(!should_watch_file(Path::new("content/post.md.swp")));
    }

    #[test]
    fn test_should_not_watch_backup_file() {
        assert!(!should_watch_file(Path::new("content/post.md~")));
    }

    #[test]
    fn test_should_not_watch_ds_store() {
        assert!(!should_watch_file(Path::new("content/.DS_Store")));
    }

    #[test]
    fn test_should_watch_css() {
        assert!(should_watch_file(Path::new("assets/style.css")));
    }

    #[test]
    fn test_should_watch_js() {
        assert!(should_watch_file(Path::new("assets/script.js")));
    }

    // --- Destination filtering tests ---

    #[test]
    fn test_is_in_destination() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("_site");
        std::fs::create_dir_all(&dest).unwrap();
        let file = dest.join("index.html");
        std::fs::write(&file, "test").unwrap();
        assert!(is_in_destination(&file, &dest));
    }

    #[test]
    fn test_is_not_in_destination() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("_site");
        std::fs::create_dir_all(&dest).unwrap();
        let source_file = dir.path().join("index.md");
        std::fs::write(&source_file, "test").unwrap();
        assert!(!is_in_destination(&source_file, &dest));
    }

    // --- Source directory filtering tests ---

    #[test]
    fn test_is_in_source_file_inside() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("site");
        std::fs::create_dir_all(&source).unwrap();
        let file = source.join("post.md");
        std::fs::write(&file, "test").unwrap();
        assert!(is_in_source(&file, &source));
    }

    #[test]
    fn test_is_in_source_file_outside_project() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("site");
        std::fs::create_dir_all(&source).unwrap();
        let file = dir.path().join("Cargo.toml");
        std::fs::write(&file, "test").unwrap();
        assert!(!is_in_source(&file, &source));
    }

    #[test]
    fn test_is_in_source_file_in_other_dir() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("site");
        let other = dir.path().join("other");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&other).unwrap();
        let file = other.join("file.md");
        std::fs::write(&file, "test").unwrap();
        assert!(!is_in_source(&file, &source));
    }

    #[test]
    fn test_is_in_source_nested_file() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("site");
        let subdir = source.join("_posts");
        std::fs::create_dir_all(&subdir).unwrap();
        let file = subdir.join("2024-01-01-hello.md");
        std::fs::write(&file, "test").unwrap();
        assert!(is_in_source(&file, &source));
    }

    #[test]
    fn test_is_in_source_but_in_destination_both_checks() {
        // A file inside source's _site subdir should be in_source=true but in_destination=true
        // This tests that both checks are needed together
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("site");
        let dest = source.join("_site");
        std::fs::create_dir_all(&dest).unwrap();
        let file = dest.join("index.html");
        std::fs::write(&file, "test").unwrap();
        assert!(is_in_source(&file, &source)); // it IS in source
        assert!(is_in_destination(&file, &dest)); // it IS in destination
                                                  // The watcher should filter it out because is_in_destination is true
    }

    #[test]
    fn test_is_in_source_git_dir_inside_source() {
        // .git inside source should be in_source=true, but should_watch_file=false
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("site");
        let git_dir = source.join(".git").join("objects");
        std::fs::create_dir_all(&git_dir).unwrap();
        let file = git_dir.join("abc123");
        std::fs::write(&file, "test").unwrap();
        assert!(is_in_source(&file, &source)); // it IS in source
        assert!(!should_watch_file(&file)); // but should NOT be watched
    }

    // --- File change classification tests ---

    #[test]
    fn test_classify_config_yml() {
        assert_eq!(classify_changed_file("_config.yml"), FileChangeKind::Config);
    }

    #[test]
    fn test_classify_config_yaml() {
        assert_eq!(
            classify_changed_file("_config.yaml"),
            FileChangeKind::Config
        );
    }

    #[test]
    fn test_classify_layout_file() {
        assert_eq!(
            classify_changed_file("_layouts/default.html"),
            FileChangeKind::Layout
        );
    }

    #[test]
    fn test_classify_layout_nested() {
        assert_eq!(
            classify_changed_file("_layouts/post.html"),
            FileChangeKind::Layout
        );
    }

    #[test]
    fn test_classify_include_file() {
        assert_eq!(
            classify_changed_file("_includes/header.html"),
            FileChangeKind::Include
        );
    }

    #[test]
    fn test_classify_data_file() {
        assert_eq!(
            classify_changed_file("_data/people.yml"),
            FileChangeKind::Data
        );
    }

    #[test]
    fn test_classify_data_nested() {
        assert_eq!(
            classify_changed_file("_data/events/2024.yml"),
            FileChangeKind::Data
        );
    }

    #[test]
    fn test_classify_content_markdown() {
        assert_eq!(
            classify_changed_file("_posts/2024-01-01-hello.md"),
            FileChangeKind::Content
        );
    }

    #[test]
    fn test_classify_content_html() {
        assert_eq!(classify_changed_file("about.html"), FileChangeKind::Content);
    }

    #[test]
    fn test_classify_content_markdown_ext() {
        assert_eq!(
            classify_changed_file("_posts/test.markdown"),
            FileChangeKind::Content
        );
    }

    #[test]
    fn test_classify_static_css() {
        assert_eq!(
            classify_changed_file("assets/style.css"),
            FileChangeKind::StaticAsset
        );
    }

    #[test]
    fn test_classify_static_js() {
        assert_eq!(
            classify_changed_file("assets/script.js"),
            FileChangeKind::StaticAsset
        );
    }

    #[test]
    fn test_classify_static_image() {
        assert_eq!(
            classify_changed_file("images/photo.png"),
            FileChangeKind::StaticAsset
        );
    }

    // --- Rebuild scope determination tests ---

    #[test]
    fn test_scope_content_only_is_partial() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path();
        // Create the files so strip_prefix works
        let posts_dir = source.join("_posts");
        std::fs::create_dir_all(&posts_dir).unwrap();
        let f1 = posts_dir.join("hello.md");
        std::fs::write(&f1, "test").unwrap();

        let changed = vec![f1];
        let scope = determine_rebuild_scope(source, &changed);
        assert_eq!(
            scope,
            RebuildScope::Partial(vec!["_posts/hello.md".to_string()])
        );
    }

    #[test]
    fn test_scope_config_change_is_full() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path();
        let config = source.join("_config.yml");
        std::fs::write(&config, "title: test").unwrap();

        let changed = vec![config];
        let scope = determine_rebuild_scope(source, &changed);
        assert_eq!(scope, RebuildScope::Full);
    }

    #[test]
    fn test_scope_layout_change_is_full() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path();
        let layouts_dir = source.join("_layouts");
        std::fs::create_dir_all(&layouts_dir).unwrap();
        let layout = layouts_dir.join("default.html");
        std::fs::write(&layout, "<html></html>").unwrap();

        let changed = vec![layout];
        let scope = determine_rebuild_scope(source, &changed);
        assert_eq!(scope, RebuildScope::Full);
    }

    #[test]
    fn test_scope_include_change_is_full() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path();
        let includes_dir = source.join("_includes");
        std::fs::create_dir_all(&includes_dir).unwrap();
        let include = includes_dir.join("header.html");
        std::fs::write(&include, "<header></header>").unwrap();

        let changed = vec![include];
        let scope = determine_rebuild_scope(source, &changed);
        assert_eq!(scope, RebuildScope::Full);
    }

    #[test]
    fn test_scope_data_change_is_full() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path();
        let data_dir = source.join("_data");
        std::fs::create_dir_all(&data_dir).unwrap();
        let data_file = data_dir.join("people.yml");
        std::fs::write(&data_file, "- name: Alice").unwrap();

        let changed = vec![data_file];
        let scope = determine_rebuild_scope(source, &changed);
        assert_eq!(scope, RebuildScope::Full);
    }

    #[test]
    fn test_scope_mixed_content_and_layout_is_full() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path();
        let posts_dir = source.join("_posts");
        std::fs::create_dir_all(&posts_dir).unwrap();
        let layouts_dir = source.join("_layouts");
        std::fs::create_dir_all(&layouts_dir).unwrap();

        let post = posts_dir.join("hello.md");
        std::fs::write(&post, "test").unwrap();
        let layout = layouts_dir.join("post.html");
        std::fs::write(&layout, "<html></html>").unwrap();

        let changed = vec![post, layout];
        let scope = determine_rebuild_scope(source, &changed);
        assert_eq!(scope, RebuildScope::Full);
    }

    #[test]
    fn test_scope_multiple_content_files_is_partial() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path();
        let posts_dir = source.join("_posts");
        std::fs::create_dir_all(&posts_dir).unwrap();

        let f1 = posts_dir.join("a.md");
        let f2 = posts_dir.join("b.md");
        std::fs::write(&f1, "test a").unwrap();
        std::fs::write(&f2, "test b").unwrap();

        let changed = vec![f1, f2];
        let scope = determine_rebuild_scope(source, &changed);
        match scope {
            RebuildScope::Partial(paths) => {
                assert_eq!(paths.len(), 2);
                assert!(paths.contains(&"_posts/a.md".to_string()));
                assert!(paths.contains(&"_posts/b.md".to_string()));
            }
            other => panic!("Expected Partial, got {:?}", other),
        }
    }

    #[test]
    fn test_scope_static_asset_is_full() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path();
        let assets_dir = source.join("assets");
        std::fs::create_dir_all(&assets_dir).unwrap();
        let css = assets_dir.join("style.css");
        std::fs::write(&css, "body{}").unwrap();

        let changed = vec![css];
        let scope = determine_rebuild_scope(source, &changed);
        assert_eq!(scope, RebuildScope::Full);
    }
}
