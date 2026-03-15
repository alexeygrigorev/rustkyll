//! Live reload support: script injection, WebSocket server, and file watching.

use std::path::{Path, PathBuf};

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
pub fn start_file_watcher(
    source: PathBuf,
    destination: PathBuf,
    reload_tx: std::sync::mpsc::Sender<()>,
    shutdown: std::sync::Arc<std::sync::atomic::AtomicBool>,
    build_fn: Box<dyn Fn() -> Result<(), String> + Send>,
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
                let relevant: Vec<_> = events
                    .iter()
                    .filter(|e| {
                        is_in_source(&e.path, &source)
                            && !is_in_destination(&e.path, &destination)
                            && should_watch_file(&e.path)
                    })
                    .collect();

                if relevant.is_empty() {
                    continue;
                }

                println!("File change detected, rebuilding...");
                match build_fn() {
                    Ok(()) => {
                        println!("Rebuild complete.");
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
}
