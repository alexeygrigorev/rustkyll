//! End-to-end integration test for the WASM external-extension adapter
//! (issue #602).
//!
//! Builds a minimal site whose `_config.yml` declares the committed sample wasm
//! extension and whose page contains `{{link:...}}` tokens, then asserts the
//! generated HTML reflects the wasm transform: a resolved token becomes an
//! anchor and an unresolved token becomes a `<span class="broken">` element.
//!
//! This proves rustkyll actually LOADS and EXECUTES the real `.wasm` end to end
//! (per "Done Means DONE"), exercising the `resolve_link` host callback for both
//! a hit and a miss.
//!
//! Run with: cargo test -p integration-tests --test integration_wasm_extension

use std::path::PathBuf;
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

fn sample_wasm() -> PathBuf {
    project_root().join("tests/fixtures/wasm/sample.wasm")
}

#[test]
fn test_end_to_end_wasm_extension_build() {
    let binary = rustkyll_binary();
    assert!(
        binary.exists(),
        "rustkyll binary not found at {:?}. Run `cargo build --release` first.",
        binary
    );
    let wasm = sample_wasm();
    assert!(
        wasm.exists(),
        "sample.wasm fixture missing at {:?} — build via tests/fixtures/wasm/sample-plugin/README.md",
        wasm
    );

    // Build a throwaway source site in a temp dir.
    let src = tempfile::TempDir::new().unwrap();
    let dst = tempfile::TempDir::new().unwrap();
    let root = src.path();

    std::fs::create_dir_all(root.join("_extensions")).unwrap();
    std::fs::copy(&wasm, root.join("_extensions/sample.wasm")).unwrap();

    std::fs::write(
        root.join("_config.yml"),
        "title: Wasm Fixture\nbaseurl: \"\"\nextensions:\n  - wasm: _extensions/sample.wasm\n",
    )
    .unwrap();

    // A resolvable target page (slug `target-page`).
    std::fs::write(
        root.join("target-page.md"),
        "---\ntitle: Target Page\n---\nHello target.\n",
    )
    .unwrap();

    // Index page with a resolvable token and an unresolvable one. `{% raw %}`
    // keeps the `{{...}}` literal through Liquid so the post-render wasm
    // transform sees it.
    std::fs::write(
        root.join("index.md"),
        "---\ntitle: Home\n---\n{% raw %}{{link:target-page}}{% endraw %} and {% raw %}{{link:no-such-slug}}{% endraw %}\n",
    )
    .unwrap();

    let output = Command::new(&binary)
        .args(["build", "--source"])
        .arg(root)
        .arg("--destination")
        .arg(dst.path())
        .output()
        .expect("failed to run rustkyll");
    assert!(
        output.status.success(),
        "wasm fixture build failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let index_html = std::fs::read_to_string(dst.path().join("index.html"))
        .expect("index.html should be generated");

    // Resolved token -> anchor whose href points at the target page's URL.
    assert!(
        index_html.contains(">target-page</a>"),
        "expected resolved wasm anchor for target-page, got:\n{index_html}"
    );
    assert!(
        index_html.contains("<a href=\"/target-page"),
        "resolved anchor href must be the SiteIndex URL for target-page, got:\n{index_html}"
    );
    // Unresolved token -> broken span (resolve_link miss round-trip).
    assert!(
        index_html.contains("<span class=\"broken\">no-such-slug</span>"),
        "expected broken span for unresolved slug, got:\n{index_html}"
    );
    // The literal token must NOT survive.
    assert!(
        !index_html.contains("{{link:target-page}}"),
        "raw token must have been transformed by the wasm plugin, got:\n{index_html}"
    );
}
