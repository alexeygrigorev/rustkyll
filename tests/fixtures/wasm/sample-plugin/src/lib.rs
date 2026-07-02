//! rustkyll sample WASM extension plugin.
//!
//! Demonstrates the full rustkyll plugin ABI (issue #602):
//!
//! * Export `transform`: takes JSON `{ "html", "collection", "baseurl" }` and
//!   returns JSON `{ "html", "warnings" }`.
//! * Imports the host function `resolve_link(target) -> url` (empty string =
//!   miss) to query the site link index without the whole index crossing the
//!   wasm boundary.
//! * Reads per-entry config from the well-known key `config` (a JSON string).
//!
//! Behavior: scans the input HTML for `{{MARKER:SLUG}}` tokens (MARKER comes
//! from `config.marker`, default `link`). For each token it calls
//! `resolve_link(SLUG)`:
//!   * hit  -> `<a href="{baseurl}{url}">SLUG</a>`
//!   * miss -> `<span class="broken">SLUG</span>` plus a warning.
//!
//! If `config.probe == "fs"` it also attempts to read a file to demonstrate the
//! host's default-deny filesystem sandbox, recording the outcome as a warning.

use extism_pdk::*;
use serde::{Deserialize, Serialize};

#[host_fn]
extern "ExtismHost" {
    fn resolve_link(target: String) -> String;
}

#[derive(Deserialize)]
struct TransformInput {
    html: String,
    #[allow(dead_code)]
    collection: Option<String>,
    #[serde(default)]
    baseurl: String,
}

#[derive(Serialize)]
struct TransformOutput {
    html: String,
    warnings: Vec<String>,
}

#[derive(Deserialize, Default)]
struct PluginConfig {
    #[serde(default)]
    marker: Option<String>,
    #[serde(default)]
    probe: Option<String>,
}

fn load_config() -> PluginConfig {
    match config::get("config") {
        Ok(Some(raw)) if !raw.is_empty() => serde_json::from_str(&raw).unwrap_or_default(),
        _ => PluginConfig::default(),
    }
}

#[plugin_fn]
pub fn transform(input: Json<TransformInput>) -> FnResult<Json<TransformOutput>> {
    let input = input.into_inner();
    let cfg = load_config();
    let marker = cfg.marker.unwrap_or_else(|| "link".to_string());

    let open = format!("{{{{{marker}:");
    let mut warnings: Vec<String> = Vec::new();

    // Optional sandbox probe: prove default-deny filesystem access.
    if cfg.probe.as_deref() == Some("fs") {
        match std::fs::read_to_string("/etc/hostname") {
            Ok(_) => warnings.push("sandbox-fs-ALLOWED".to_string()),
            Err(_) => warnings.push("sandbox-fs-denied".to_string()),
        }
    }

    let mut out = String::with_capacity(input.html.len());
    let mut rest = input.html.as_str();
    while let Some(start) = rest.find(&open) {
        out.push_str(&rest[..start]);
        let after = &rest[start + open.len()..];
        if let Some(end) = after.find("}}") {
            let slug = &after[..end];
            let url = unsafe { resolve_link(slug.to_string())? };
            if url.is_empty() {
                out.push_str(&format!("<span class=\"broken\">{slug}</span>"));
                warnings.push(format!("unresolved link: {slug}"));
            } else {
                out.push_str(&format!(
                    "<a href=\"{}{}\">{}</a>",
                    input.baseurl, url, slug
                ));
            }
            rest = &after[end + 2..];
        } else {
            // No closing `}}`; emit the opener literally and stop scanning.
            out.push_str(&open);
            rest = after;
        }
    }
    out.push_str(rest);

    Ok(Json(TransformOutput {
        html: out,
        warnings,
    }))
}
