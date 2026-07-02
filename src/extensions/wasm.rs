//! WASM external-extension adapter (issue #602).
//!
//! A [`WasmExtension`] loads a third-party `.wasm` module and drives it through
//! the exact same [`HtmlTransform`] trait as the compiled-in extensions, so the
//! render pipeline ([`crate::extensions::Registry::apply`]) calls native and
//! wasm transforms identically. This is the escape hatch that lets third
//! parties extend rustkyll **without source access and without recompiling**.
//!
//! The whole module is gated behind the default-on `wasm` cargo feature so that
//! `--no-default-features` builds skip the [`extism`]/wasmtime engine entirely.
//!
//! # Plugin ABI
//!
//! - Export `transform`: JSON `{ "html", "collection", "baseurl" }` in ->
//!   JSON `{ "html", "warnings" }` out. Non-JSON / schema-invalid output is an
//!   ABI mismatch surfaced as a loud warning (never a silent pass-through).
//! - Host function `resolve_link(target) -> url` where an **empty string** is
//!   the miss sentinel. Only the queried string and the resolved URL cross the
//!   wasm boundary — the whole [`SiteIndex`] never does.
//! - Optional per-entry config is serialized to JSON and passed under the
//!   well-known extism config key `config`.
//!
//! # Sandboxing
//!
//! Third-party wasm is untrusted, so plugins are instantiated with **no**
//! `allowed_paths` and **no** `allowed_hosts` (extism default-deny). A plugin
//! attempting filesystem or network access is denied at runtime.

use super::{ExtensionError, HtmlTransform, SiteIndex, TransformContext, TransformResult};
use extism::{host_fn, Function, Manifest, Plugin, UserData, Wasm, PTR};
use serde::Deserialize;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// Slot holding the address of the current [`SiteIndex`] for the duration of a
/// single `transform` call. `0` means "no index bound" (all lookups miss).
///
/// The pointer is only ever written while the owning [`WasmExtension`] holds its
/// plugin mutex, and only read from `resolve_link` which runs synchronously
/// inside `plugin.call`, so the borrow is always live while it is used.
type IndexSlot = usize;

host_fn!(resolve_link(user_data: IndexSlot; target: String) -> String {
    let slot = user_data.get()?;
    let ptr = *slot.lock().unwrap_or_else(|e| e.into_inner());
    if ptr == 0 {
        return Ok(String::new());
    }
    // SAFETY: `WasmExtension::transform` stores the address of a live
    // `&SiteIndex` here while holding the plugin mutex, and clears it before
    // releasing the mutex. `resolve_link` only runs synchronously from inside
    // `plugin.call` (which requires that mutex), so the reference is valid for
    // the whole duration of this call.
    let index: &SiteIndex = unsafe { &*(ptr as *const SiteIndex) };
    Ok(index.resolve(&target).unwrap_or("").to_string())
});

/// Shape of the JSON a plugin returns from its `transform` export.
#[derive(Deserialize)]
struct WasmOutput {
    html: String,
    #[serde(default)]
    warnings: Vec<String>,
}

/// An extension backed by a loaded `.wasm` module.
///
/// The `.wasm` is instantiated exactly once (in [`WasmExtension::load`]) and the
/// single [`Plugin`] is reused for every document. Because an extism `Plugin` is
/// `Send` but not `Sync` and `call` takes `&mut self`, it is wrapped in a
/// [`Mutex`] so `WasmExtension` is `Sync` and wasm calls serialize through the
/// lock.
pub struct WasmExtension {
    name: String,
    plugin: Mutex<Plugin>,
    index_slot: UserData<IndexSlot>,
    calls: AtomicUsize,
}

impl WasmExtension {
    /// Load and instantiate a `.wasm` plugin.
    ///
    /// `path` is resolved relative to `source_root`. `config_json`, when
    /// present, is passed to the plugin under the well-known config key
    /// `config`. Fails loudly ([`ExtensionError::WasmLoad`]) on a missing file,
    /// corrupt bytes, or a module missing the `transform` export.
    pub fn load(
        path: &str,
        source_root: &Path,
        config_json: Option<String>,
    ) -> Result<Self, ExtensionError> {
        let wasm_path = source_root.join(path);
        let display = wasm_path.display().to_string();

        if !wasm_path.exists() {
            return Err(ExtensionError::WasmLoad {
                path: display,
                message: "file not found".to_string(),
            });
        }

        let bytes = std::fs::read(&wasm_path).map_err(|e| ExtensionError::WasmLoad {
            path: display.clone(),
            message: format!("could not read file: {e}"),
        })?;

        let mut manifest = Manifest::new([Wasm::data(bytes)]);
        if let Some(cfg) = config_json {
            manifest = manifest.with_config_key("config", cfg);
        }
        // Default-deny sandbox: no allowed_paths / allowed_hosts are set, so the
        // plugin gets no filesystem and no network access.

        let index_slot: UserData<IndexSlot> = UserData::new(0);
        let resolve = Function::new(
            "resolve_link",
            [PTR],
            [PTR],
            index_slot.clone(),
            resolve_link,
        );

        // WASI is enabled so plugins built for wasm32-wasip1 link, but with no
        // preopened dirs (empty allowed_paths) filesystem access stays denied.
        let plugin =
            Plugin::new(&manifest, [resolve], true).map_err(|e| ExtensionError::WasmLoad {
                path: display.clone(),
                message: format!("failed to instantiate: {e}"),
            })?;

        if !plugin.function_exists("transform") {
            return Err(ExtensionError::WasmLoad {
                path: display,
                message: "module does not export the required `transform` function".to_string(),
            });
        }

        let name = format!("wasm:{path}");
        Ok(WasmExtension {
            name,
            plugin: Mutex::new(plugin),
            index_slot,
            calls: AtomicUsize::new(0),
        })
    }

    /// Number of times [`HtmlTransform::transform`] has run on this instance.
    ///
    /// Used by tests to observe that the single instantiated plugin is reused
    /// across documents rather than reinstantiated per document.
    pub fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl std::fmt::Debug for WasmExtension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmExtension")
            .field("name", &self.name)
            .field("calls", &self.call_count())
            .finish()
    }
}

impl HtmlTransform for WasmExtension {
    fn name(&self) -> &str {
        &self.name
    }

    fn transform(&self, html: &str, index: &SiteIndex, ctx: &TransformContext) -> TransformResult {
        self.calls.fetch_add(1, Ordering::SeqCst);

        let input = serde_json::json!({
            "html": html,
            "collection": ctx.collection,
            "baseurl": ctx.baseurl,
        });
        let input_bytes = match serde_json::to_vec(&input) {
            Ok(b) => b,
            Err(e) => {
                return TransformResult {
                    html: html.to_string(),
                    warnings: vec![format!(
                        "wasm extension `{}`: could not serialize input: {e}",
                        self.name
                    )],
                }
            }
        };

        let index_ptr = index as *const SiteIndex as usize;
        let mut plugin = self.plugin.lock().unwrap_or_else(|e| e.into_inner());

        // Bind the index pointer only while holding the plugin lock so that
        // concurrent `transform` calls (which serialize through this lock) never
        // observe another call's index.
        if let Ok(slot) = self.index_slot.get() {
            *slot.lock().unwrap_or_else(|e| e.into_inner()) = index_ptr;
        }

        let result = plugin.call::<&[u8], &[u8]>("transform", &input_bytes);

        // Parse before clearing so the borrow of `plugin` (which the output
        // bytes reference) ends first.
        let parsed = match result {
            Ok(out_bytes) => match serde_json::from_slice::<WasmOutput>(out_bytes) {
                Ok(out) => Ok(TransformResult {
                    html: out.html,
                    warnings: out.warnings,
                }),
                Err(e) => Err(format!(
                    "wasm extension `{}` returned invalid output: {e}",
                    self.name
                )),
            },
            Err(e) => Err(format!("wasm extension `{}` failed: {e}", self.name)),
        };

        if let Ok(slot) = self.index_slot.get() {
            *slot.lock().unwrap_or_else(|e| e.into_inner()) = 0;
        }
        drop(plugin);

        match parsed {
            Ok(res) => res,
            // ABI mismatch / call failure is surfaced loudly as a warning (the
            // pipeline prints warnings to stderr) with the HTML left unchanged.
            Err(message) => TransformResult {
                html: html.to_string(),
                warnings: vec![message],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn sample_wasm_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/wasm/sample.wasm")
    }

    fn load_sample(config_json: Option<String>) -> WasmExtension {
        let path = sample_wasm_path();
        assert!(
            path.exists(),
            "sample.wasm fixture missing at {} — build it via tests/fixtures/wasm/sample-plugin/README.md",
            path.display()
        );
        // Load with the fixtures dir as source root and the bare filename.
        let root = path.parent().expect("fixtures dir");
        WasmExtension::load("sample.wasm", root, config_json).expect("load sample.wasm")
    }

    fn index_with(slug: &str, url: &str) -> SiteIndex {
        let mut idx = SiteIndex::new();
        idx.insert(slug, None, url);
        idx
    }

    #[test]
    fn test_sample_wasm_transforms_and_resolves_hit() {
        let ext = load_sample(None);
        let idx = index_with("event-tracking", "/wiki/event-tracking/");
        let ctx = TransformContext {
            baseurl: "",
            collection: Some("wiki"),
        };
        let res = ext.transform("<p>{{link:event-tracking}}</p>", &idx, &ctx);
        assert_eq!(
            res.html,
            "<p><a href=\"/wiki/event-tracking/\">event-tracking</a></p>"
        );
        assert!(
            res.warnings.is_empty(),
            "hit must not warn: {:?}",
            res.warnings
        );
    }

    #[test]
    fn test_sample_wasm_resolve_link_miss_renders_broken_and_warns() {
        let ext = load_sample(None);
        let idx = index_with("event-tracking", "/wiki/event-tracking/");
        let ctx = TransformContext {
            baseurl: "",
            collection: None,
        };
        let res = ext.transform("<p>{{link:missing-slug}}</p>", &idx, &ctx);
        assert_eq!(
            res.html,
            "<p><span class=\"broken\">missing-slug</span></p>"
        );
        assert!(
            res.warnings.iter().any(|w| w.contains("missing-slug")),
            "miss must produce a warning, got {:?}",
            res.warnings
        );
    }

    #[test]
    fn test_sample_wasm_baseurl_applied() {
        let ext = load_sample(None);
        let idx = index_with("foo", "/wiki/foo/");
        let ctx = TransformContext {
            baseurl: "/blog",
            collection: None,
        };
        let res = ext.transform("{{link:foo}}", &idx, &ctx);
        assert_eq!(res.html, "<a href=\"/blog/wiki/foo/\">foo</a>");
    }

    #[test]
    fn test_sample_wasm_config_changes_marker() {
        // config: { marker: "ref" } -> the plugin now transforms {{ref:...}}.
        let ext = load_sample(Some(r#"{"marker":"ref"}"#.to_string()));
        let idx = index_with("foo", "/wiki/foo/");
        let ctx = TransformContext {
            baseurl: "",
            collection: None,
        };
        // {{link:...}} is no longer recognized, {{ref:...}} is.
        let res = ext.transform("{{link:foo}} {{ref:foo}}", &idx, &ctx);
        assert_eq!(
            res.html, "{{link:foo}} <a href=\"/wiki/foo/\">foo</a>",
            "config.marker must switch the recognized token"
        );
    }

    #[test]
    fn test_instantiated_once_reused_across_documents() {
        let ext = load_sample(None);
        let idx = index_with("foo", "/wiki/foo/");
        let ctx = TransformContext {
            baseurl: "",
            collection: None,
        };
        assert_eq!(ext.call_count(), 0);
        for _ in 0..5 {
            let _ = ext.transform("{{link:foo}}", &idx, &ctx);
        }
        // A single instance serviced 5 documents => reused, not reinstantiated.
        assert_eq!(ext.call_count(), 5);
    }

    #[test]
    fn test_sandbox_denies_filesystem_access() {
        // With probe=fs the plugin tries to read a file; default-deny sandbox
        // (no allowed_paths) must cause it to fail, recorded as a warning.
        let ext = load_sample(Some(r#"{"probe":"fs"}"#.to_string()));
        let idx = SiteIndex::new();
        let ctx = TransformContext {
            baseurl: "",
            collection: None,
        };
        let res = ext.transform("<p>hi</p>", &idx, &ctx);
        assert!(
            res.warnings.iter().any(|w| w == "sandbox-fs-denied"),
            "filesystem access must be denied, warnings: {:?}",
            res.warnings
        );
        assert!(
            !res.warnings.iter().any(|w| w == "sandbox-fs-ALLOWED"),
            "filesystem access must NOT succeed, warnings: {:?}",
            res.warnings
        );
    }

    #[test]
    fn test_missing_wasm_file_errors() {
        let err = WasmExtension::load("does-not-exist.wasm", Path::new("/tmp"), None)
            .expect_err("missing file must error");
        match err {
            ExtensionError::WasmLoad { message, .. } => {
                assert!(message.contains("not found"), "got: {message}")
            }
            other => panic!("expected WasmLoad, got {other:?}"),
        }
    }

    #[test]
    fn test_wasm_missing_transform_export_errors() {
        // A minimal but valid wasm module (bare header) instantiates yet lacks
        // the required `transform` export -> ABI-mismatch WasmLoad, not a panic.
        let dir = tempfile::tempdir().expect("tempdir");
        let empty = dir.path().join("empty.wasm");
        // 8-byte module: "\0asm" magic + version 1.
        std::fs::write(&empty, [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00])
            .expect("write empty wasm");
        let err = WasmExtension::load("empty.wasm", dir.path(), None)
            .expect_err("module without transform export must error");
        match err {
            ExtensionError::WasmLoad { message, .. } => assert!(
                message.contains("transform") || message.contains("instantiate"),
                "got: {message}"
            ),
            other => panic!("expected WasmLoad, got {other:?}"),
        }
    }

    #[test]
    fn test_corrupt_wasm_errors() {
        let dir = tempfile::tempdir().expect("tempdir");
        let bad = dir.path().join("bad.wasm");
        std::fs::write(&bad, b"this is not wasm").expect("write bad wasm");
        let err =
            WasmExtension::load("bad.wasm", dir.path(), None).expect_err("corrupt wasm must error");
        assert!(
            matches!(err, ExtensionError::WasmLoad { .. }),
            "expected WasmLoad, got {err:?}"
        );
    }
}
