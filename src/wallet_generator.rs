//! Wallet page generator.
//!
//! Emulates Jekyll's `WalletsPageGenerator` plugin (wallets.rb), which generates
//! wallet and platform pages by combining data from three sources:
//!
//! - `_wallets/*.md` (wallet definition files with YAML front matter)
//! - `_platforms/` (platform/OS definition files with YAML front matter)
//! - `_translations/*.yml` (language files with `choose-your-wallet` section)
//!
//! Detection is based on directory/file presence (not site name):
//! - `_plugins/wallets.rb` exists (and contains `WalletsPageGenerator`)
//! - `_wallets/` directory exists
//! - `_platforms/` directory exists
//! - `_translations/` directory exists

use std::fs;
use std::path::Path;

use crate::collection::Page;
use crate::frontmatter::FrontMatter;
use crate::template::translate_tag;

/// Check whether the wallet page generator should be activated.
///
/// Returns true when ALL of the following are present:
/// - `_plugins/wallets.rb` (containing `WalletsPageGenerator`)
/// - `_wallets/` directory
/// - `_platforms/` directory
/// - `_translations/` directory
pub fn should_activate(source_dir: &Path) -> bool {
    let wallets_dir = source_dir.join("_wallets");
    let platforms_dir = source_dir.join("_platforms");
    let translations_dir = source_dir.join("_translations");

    if !wallets_dir.is_dir() || !platforms_dir.is_dir() || !translations_dir.is_dir() {
        return false;
    }

    // Check for wallets.rb containing WalletsPageGenerator
    let plugins_dir = source_dir.join("_plugins");
    if !plugins_dir.is_dir() {
        return false;
    }

    // Look for wallets.rb specifically
    let wallets_rb = plugins_dir.join("wallets.rb");
    if wallets_rb.is_file() {
        if let Ok(content) = fs::read_to_string(&wallets_rb) {
            if content.contains("WalletsPageGenerator") {
                return true;
            }
        }
    }

    // Also check other .rb files in plugins dir
    if let Ok(entries) = fs::read_dir(&plugins_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("rb") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if content.contains("WalletsPageGenerator") {
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// A parsed platform definition from `_platforms/`.
#[derive(Debug, Clone)]
struct PlatformDef {
    /// Platform name (e.g., "desktop", "mobile", "hardware", "web").
    platform_name: String,
    /// OS name (e.g., "linux", "windows", or same as platform for top-level).
    os_name: String,
}

/// A parsed wallet definition from `_wallets/`.
#[derive(Debug, Clone)]
struct WalletDef {
    /// Wallet ID (e.g., "electrum").
    id: String,
    /// Wallet title (e.g., "Electrum").
    title: String,
    /// Full YAML data as serde_yaml::Value for template context.
    data: serde_yaml::Value,
    /// Platform entries from the wallet.
    platforms: Vec<WalletPlatform>,
}

/// A platform entry within a wallet definition.
#[derive(Debug, Clone)]
struct WalletPlatform {
    /// Platform name (e.g., "desktop").
    name: String,
    /// Full platform data as YAML for template context.
    data: serde_yaml::Value,
    /// OS entries within this platform.
    os_entries: Vec<WalletOs>,
}

/// An OS entry within a wallet platform.
#[derive(Debug, Clone)]
struct WalletOs {
    /// OS name (e.g., "linux", "windows").
    name: String,
    /// Full OS data as YAML for template context.
    data: serde_yaml::Value,
}

/// Generate wallet and platform pages.
///
/// Returns a vector of virtual Page structs ready for the rendering pipeline.
pub fn generate_wallet_pages(source_dir: &Path) -> Result<Vec<Page>, String> {
    let translations_dir = source_dir.join("_translations");
    let wallets_dir = source_dir.join("_wallets");
    let platforms_dir = source_dir.join("_platforms");

    // Load translations (reuse the existing translation store infrastructure)
    let store = translate_tag::load_translations(&translations_dir)?;

    // Ensure translations are set globally for {% translate %} tag
    translate_tag::set_translations(store.clone());

    // Load platform definitions
    let platform_defs = load_platform_defs(&platforms_dir)?;

    // Load wallet definitions
    let wallet_defs = load_wallet_defs(&wallets_dir)?;

    let mut pages = Vec::new();

    // For each language
    for lang in store.strings.keys() {
        let title =
            get_translation_string(&store, lang, "choose-your-wallet", "title").unwrap_or_default();

        // 1. Generate platform pages
        for pdef in &platform_defs {
            let dir = if pdef.platform_name == pdef.os_name {
                pdef.platform_name.clone()
            } else {
                format!("{}/{}", pdef.platform_name, pdef.os_name)
            };

            let platform_title = get_translation_string(
                &store,
                lang,
                "choose-your-wallet",
                &format!("walletcat{}", pdef.platform_name),
            )
            .unwrap_or_default();

            let os_title = get_translation_string(
                &store,
                lang,
                "choose-your-wallet",
                &format!("platform{}", pdef.os_name),
            );

            let full_title = match os_title {
                Some(ref ot) => format!("{} - {} - {}", platform_title, ot, title),
                None => format!("{} - {}", platform_title, title),
            };

            let page_id = format!("wallets-{}-{}", pdef.platform_name, pdef.os_name);

            let url = format!("/{}/wallets/{}/index.html", lang, dir);

            let mut fm = FrontMatter::new();
            fm.insert(
                "layout".to_string(),
                serde_yaml::Value::String("wallet-platform".to_string()),
            );
            fm.insert("lang".to_string(), serde_yaml::Value::String(lang.clone()));
            fm.insert("id".to_string(), serde_yaml::Value::String(page_id));
            fm.insert("title".to_string(), serde_yaml::Value::String(full_title));

            // platform and os as YAML mappings
            let mut platform_map = serde_yaml::Mapping::new();
            platform_map.insert(
                serde_yaml::Value::String("name".to_string()),
                serde_yaml::Value::String(pdef.platform_name.clone()),
            );
            fm.insert(
                "platform".to_string(),
                serde_yaml::Value::Mapping(platform_map),
            );

            let mut os_map = serde_yaml::Mapping::new();
            os_map.insert(
                serde_yaml::Value::String("name".to_string()),
                serde_yaml::Value::String(pdef.os_name.clone()),
            );
            fm.insert("os".to_string(), serde_yaml::Value::Mapping(os_map));

            pages.push(Page {
                slug: "index".to_string(),
                front_matter: fm,
                content: String::new(),
                html_content: String::new(),
                url,
                source_path: format!("_platforms/{}", dir),
            });
        }

        // 2. Generate wallet pages
        for wallet in &wallet_defs {
            for platform in &wallet.platforms {
                for os in &platform.os_entries {
                    if platform.name.is_empty() {
                        continue;
                    }

                    let dir = if platform.name == os.name {
                        format!("{}/{}", platform.name, wallet.id)
                    } else {
                        format!("{}/{}/{}", platform.name, os.name, wallet.id)
                    };

                    let platform_title = get_translation_string(
                        &store,
                        lang,
                        "choose-your-wallet",
                        &format!("walletcat{}", platform.name),
                    )
                    .unwrap_or_default();

                    let os_title = get_translation_string(
                        &store,
                        lang,
                        "choose-your-wallet",
                        &format!("platform{}", os.name),
                    );

                    let full_title = match os_title {
                        Some(ref ot) => {
                            format!("{} - {} - {} - {}", wallet.title, platform_title, ot, title)
                        }
                        None => format!("{} - {} - {}", wallet.title, platform_title, title),
                    };

                    let page_id = format!("wallets-{}-{}-{}", platform.name, os.name, wallet.id);

                    let url = format!("/{}/wallets/{}/index.html", lang, dir);

                    let mut fm = FrontMatter::new();
                    fm.insert(
                        "layout".to_string(),
                        serde_yaml::Value::String("wallet-container".to_string()),
                    );
                    fm.insert("lang".to_string(), serde_yaml::Value::String(lang.clone()));
                    fm.insert("id".to_string(), serde_yaml::Value::String(page_id));
                    fm.insert("title".to_string(), serde_yaml::Value::String(full_title));

                    // wallet -- full wallet data
                    fm.insert("wallet".to_string(), wallet.data.clone());

                    // platform -- current platform data
                    fm.insert("platform".to_string(), platform.data.clone());

                    // os -- current OS data
                    fm.insert("os".to_string(), os.data.clone());

                    pages.push(Page {
                        slug: wallet.id.clone(),
                        front_matter: fm,
                        content: String::new(),
                        html_content: String::new(),
                        url,
                        source_path: format!("_wallets/{}.md", wallet.id),
                    });
                }
            }
        }
    }

    Ok(pages)
}

/// Load platform definitions from the `_platforms/` directory (recursive).
fn load_platform_defs(platforms_dir: &Path) -> Result<Vec<PlatformDef>, String> {
    let mut defs = Vec::new();
    load_platform_defs_recursive(platforms_dir, &mut defs)?;
    defs.sort_by(|a, b| {
        a.platform_name
            .cmp(&b.platform_name)
            .then_with(|| a.os_name.cmp(&b.os_name))
    });
    Ok(defs)
}

fn load_platform_defs_recursive(dir: &Path, defs: &mut Vec<PlatformDef>) -> Result<(), String> {
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("Failed to read platforms dir {}: {}", dir.display(), e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            load_platform_defs_recursive(&path, defs)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("html") {
            let content = fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

            if let Some(yaml_data) = extract_yaml_front_matter(&content) {
                let platform_name = yaml_data
                    .get("platform")
                    .and_then(|v| v.as_mapping())
                    .and_then(|m| m.get(serde_yaml::Value::String("name".to_string())))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let os_name = yaml_data
                    .get("os")
                    .and_then(|v| v.as_mapping())
                    .and_then(|m| m.get(serde_yaml::Value::String("name".to_string())))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                if !platform_name.is_empty() && !os_name.is_empty() {
                    defs.push(PlatformDef {
                        platform_name,
                        os_name,
                    });
                }
            }
        }
    }

    Ok(())
}

/// Load wallet definitions from the `_wallets/` directory.
fn load_wallet_defs(wallets_dir: &Path) -> Result<Vec<WalletDef>, String> {
    let mut defs = Vec::new();

    let entries =
        fs::read_dir(wallets_dir).map_err(|e| format!("Failed to read wallets dir: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

        let yaml_data = match extract_yaml_front_matter(&content) {
            Some(data) => data,
            None => continue,
        };

        let id = yaml_data
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let title = yaml_data
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if id.is_empty() {
            continue;
        }

        let platforms = parse_wallet_platforms(&yaml_data);

        defs.push(WalletDef {
            id,
            title,
            data: serde_yaml::Value::Mapping(yaml_data.as_mapping().cloned().unwrap_or_default()),
            platforms,
        });
    }

    defs.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(defs)
}

/// Parse the `platform` array from a wallet's YAML data.
fn parse_wallet_platforms(yaml_data: &serde_yaml::Value) -> Vec<WalletPlatform> {
    let platforms_val = match yaml_data.get("platform") {
        Some(serde_yaml::Value::Sequence(seq)) => seq,
        _ => return Vec::new(),
    };

    let mut result = Vec::new();

    for platform_entry in platforms_val {
        let mapping = match platform_entry.as_mapping() {
            Some(m) => m,
            None => continue,
        };

        // The platform name is at the top level of the mapping
        let name = mapping
            .get(serde_yaml::Value::String("name".to_string()))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let os_entries = mapping
            .get(serde_yaml::Value::String("os".to_string()))
            .and_then(|v| v.as_sequence())
            .map(|os_seq| {
                os_seq
                    .iter()
                    .filter_map(|os_val| {
                        let os_map = os_val.as_mapping()?;
                        let os_name = os_map
                            .get(serde_yaml::Value::String("name".to_string()))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        if os_name.is_empty() {
                            return None;
                        }

                        Some(WalletOs {
                            name: os_name,
                            data: os_val.clone(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Build platform data for template context (the mapping itself)
        result.push(WalletPlatform {
            name,
            data: platform_entry.clone(),
            os_entries,
        });
    }

    result
}

/// Extract and parse YAML front matter from a file content string.
fn extract_yaml_front_matter(content: &str) -> Option<serde_yaml::Value> {
    if !content.starts_with("---") {
        return None;
    }

    let rest = &content[3..];
    let end = rest.find("\n---")?;
    let yaml_str = &rest[..end];

    serde_yaml::from_str(yaml_str).ok()
}

/// Get a translation string from the store.
fn get_translation_string(
    store: &translate_tag::TranslationStore,
    lang: &str,
    category: &str,
    key: &str,
) -> Option<String> {
    store.strings.get(lang)?.get(category)?.get(key).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("rustkyll_wallet_gen_{}", name));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn setup_full_fixture(dir: &Path) {
        // Create directories
        fs::create_dir_all(dir.join("_wallets")).unwrap();
        fs::create_dir_all(dir.join("_platforms/desktop")).unwrap();
        fs::create_dir_all(dir.join("_platforms/mobile")).unwrap();
        fs::create_dir_all(dir.join("_translations")).unwrap();
        fs::create_dir_all(dir.join("_plugins")).unwrap();

        // Write wallets.rb plugin
        fs::write(
            dir.join("_plugins/wallets.rb"),
            "class WalletsPageGenerator < Jekyll::Generator\nend\n",
        )
        .unwrap();

        // Write a wallet file
        fs::write(
            dir.join("_wallets/testwallet.md"),
            r#"---
id: testwallet
title: "Test Wallet"
titleshort: "TestW"
platform:
  - desktop:
    name: desktop
    os:
      - name: windows
        text: "wallettestwallet"
        link: "https://example.com"
        features: "bech32 segwit"
        check:
          control: "checkgoodcontrolfull"
      - name: linux
        text: "wallettestwallet"
        link: "https://example.com"
        features: "bech32 segwit"
        check:
          control: "checkgoodcontrolfull"
  - hardware:
    name: hardware
    os:
      - name: hardware
        text: "wallettestwallet"
        link: "https://example.com"
        features: "hardware_wallet"
        check:
          control: "checkgoodcontrolfull"
---
"#,
        )
        .unwrap();

        // Write platform files
        fs::write(
            dir.join("_platforms/desktop.html"),
            "---\nplatform:\n  name: desktop\nos:\n  name: desktop\n---\n",
        )
        .unwrap();
        fs::write(
            dir.join("_platforms/desktop/windows.html"),
            "---\nplatform:\n  name: desktop\nos:\n  name: windows\n---\n",
        )
        .unwrap();
        fs::write(
            dir.join("_platforms/desktop/linux.html"),
            "---\nplatform:\n  name: desktop\nos:\n  name: linux\n---\n",
        )
        .unwrap();
        fs::write(
            dir.join("_platforms/hardware.html"),
            "---\nplatform:\n  name: hardware\nos:\n  name: hardware\n---\n",
        )
        .unwrap();

        // Write translation files
        fs::write(
            dir.join("_translations/en.yml"),
            r#"en:
  url:
    index: ""
  choose-your-wallet:
    title: "Choose your wallet - Bitcoin"
    walletcatdesktop: "Desktop"
    walletcathardware: "Hardware"
    platformwindows: "Windows"
    platformlinux: "Linux"
"#,
        )
        .unwrap();
    }

    // --- Detection tests ---

    #[test]
    fn test_should_activate_with_all_present() {
        let dir = create_test_dir("activate_all");
        setup_full_fixture(&dir);
        assert!(
            should_activate(&dir),
            "Wallet generator should activate with all dirs present"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_should_not_activate_without_wallets_rb() {
        let dir = create_test_dir("no_wallets_rb");
        setup_full_fixture(&dir);
        fs::remove_file(dir.join("_plugins/wallets.rb")).unwrap();
        assert!(
            !should_activate(&dir),
            "Wallet generator should NOT activate without wallets.rb"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_should_not_activate_without_wallets_dir() {
        let dir = create_test_dir("no_wallets_dir");
        setup_full_fixture(&dir);
        fs::remove_dir_all(dir.join("_wallets")).unwrap();
        assert!(
            !should_activate(&dir),
            "Wallet generator should NOT activate without _wallets/"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_should_not_activate_without_platforms_dir() {
        let dir = create_test_dir("no_platforms_dir");
        setup_full_fixture(&dir);
        fs::remove_dir_all(dir.join("_platforms")).unwrap();
        assert!(
            !should_activate(&dir),
            "Wallet generator should NOT activate without _platforms/"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_should_not_activate_without_translations_dir() {
        let dir = create_test_dir("no_translations_dir");
        setup_full_fixture(&dir);
        fs::remove_dir_all(dir.join("_translations")).unwrap();
        assert!(
            !should_activate(&dir),
            "Wallet generator should NOT activate without _translations/"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    // --- Platform page generation ---

    #[test]
    fn test_platform_pages_generated() {
        let dir = create_test_dir("platform_pages");
        setup_full_fixture(&dir);

        let pages = generate_wallet_pages(&dir).unwrap();

        // 4 platforms x 1 language = 4 platform pages
        let platform_pages: Vec<_> = pages
            .iter()
            .filter(|p| {
                p.front_matter.get("layout").and_then(|v| v.as_str()) == Some("wallet-platform")
            })
            .collect();

        assert_eq!(
            platform_pages.len(),
            4,
            "Expected 4 platform pages, got {}",
            platform_pages.len()
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_platform_page_url_same_name() {
        let dir = create_test_dir("platform_url_same");
        setup_full_fixture(&dir);

        let pages = generate_wallet_pages(&dir).unwrap();

        // desktop.html has platform.name == os.name == "desktop"
        let desktop_page = pages
            .iter()
            .find(|p| p.url == "/en/wallets/desktop/index.html");
        assert!(
            desktop_page.is_some(),
            "Missing /en/wallets/desktop/index.html platform page"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_platform_page_url_different_name() {
        let dir = create_test_dir("platform_url_diff");
        setup_full_fixture(&dir);

        let pages = generate_wallet_pages(&dir).unwrap();

        // desktop/linux has platform.name != os.name
        let linux_page = pages
            .iter()
            .find(|p| p.url == "/en/wallets/desktop/linux/index.html");
        assert!(
            linux_page.is_some(),
            "Missing /en/wallets/desktop/linux/index.html platform page"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_platform_page_layout() {
        let dir = create_test_dir("platform_layout");
        setup_full_fixture(&dir);

        let pages = generate_wallet_pages(&dir).unwrap();
        let platform_page = pages
            .iter()
            .find(|p| p.url == "/en/wallets/desktop/index.html")
            .expect("Missing desktop platform page");

        assert_eq!(
            platform_page
                .front_matter
                .get("layout")
                .unwrap()
                .as_str()
                .unwrap(),
            "wallet-platform"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_platform_page_id_format() {
        let dir = create_test_dir("platform_id");
        setup_full_fixture(&dir);

        let pages = generate_wallet_pages(&dir).unwrap();
        let linux_page = pages
            .iter()
            .find(|p| p.url == "/en/wallets/desktop/linux/index.html")
            .expect("Missing desktop/linux platform page");

        assert_eq!(
            linux_page.front_matter.get("id").unwrap().as_str().unwrap(),
            "wallets-desktop-linux"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_platform_page_title_composed() {
        let dir = create_test_dir("platform_title");
        setup_full_fixture(&dir);

        let pages = generate_wallet_pages(&dir).unwrap();
        let linux_page = pages
            .iter()
            .find(|p| p.url == "/en/wallets/desktop/linux/index.html")
            .expect("Missing desktop/linux platform page");

        let title = linux_page
            .front_matter
            .get("title")
            .unwrap()
            .as_str()
            .unwrap();
        assert!(
            title.contains("Desktop"),
            "Title should contain platform category: {}",
            title
        );
        assert!(
            title.contains("Linux"),
            "Title should contain OS name: {}",
            title
        );
        assert!(
            title.contains("Choose your wallet"),
            "Title should contain main title: {}",
            title
        );

        let _ = fs::remove_dir_all(&dir);
    }

    // --- Wallet page generation ---

    #[test]
    fn test_wallet_pages_generated() {
        let dir = create_test_dir("wallet_pages");
        setup_full_fixture(&dir);

        let pages = generate_wallet_pages(&dir).unwrap();

        // testwallet has 3 OS entries (windows, linux, hardware) x 1 language = 3 wallet pages
        let wallet_pages: Vec<_> = pages
            .iter()
            .filter(|p| {
                p.front_matter.get("layout").and_then(|v| v.as_str()) == Some("wallet-container")
            })
            .collect();

        assert_eq!(
            wallet_pages.len(),
            3,
            "Expected 3 wallet pages, got {}",
            wallet_pages.len()
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_wallet_page_url_platform_ne_os() {
        let dir = create_test_dir("wallet_url_diff");
        setup_full_fixture(&dir);

        let pages = generate_wallet_pages(&dir).unwrap();

        // desktop/linux/testwallet
        let page = pages
            .iter()
            .find(|p| p.url == "/en/wallets/desktop/linux/testwallet/index.html");
        assert!(
            page.is_some(),
            "Missing /en/wallets/desktop/linux/testwallet/index.html"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_wallet_page_url_platform_eq_os() {
        let dir = create_test_dir("wallet_url_same");
        setup_full_fixture(&dir);

        let pages = generate_wallet_pages(&dir).unwrap();

        // hardware/testwallet (platform == os == hardware)
        let page = pages
            .iter()
            .find(|p| p.url == "/en/wallets/hardware/testwallet/index.html");
        assert!(
            page.is_some(),
            "Missing /en/wallets/hardware/testwallet/index.html"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_wallet_page_layout() {
        let dir = create_test_dir("wallet_layout");
        setup_full_fixture(&dir);

        let pages = generate_wallet_pages(&dir).unwrap();
        let wallet_page = pages
            .iter()
            .find(|p| p.url == "/en/wallets/desktop/linux/testwallet/index.html")
            .expect("Missing wallet page");

        assert_eq!(
            wallet_page
                .front_matter
                .get("layout")
                .unwrap()
                .as_str()
                .unwrap(),
            "wallet-container"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_wallet_page_has_wallet_data() {
        let dir = create_test_dir("wallet_data");
        setup_full_fixture(&dir);

        let pages = generate_wallet_pages(&dir).unwrap();
        let wallet_page = pages
            .iter()
            .find(|p| p.url == "/en/wallets/desktop/linux/testwallet/index.html")
            .expect("Missing wallet page");

        // page.wallet should contain full wallet data
        let wallet_data = wallet_page
            .front_matter
            .get("wallet")
            .expect("Missing wallet data in front matter");

        assert_eq!(
            wallet_data.get("id").unwrap().as_str().unwrap(),
            "testwallet"
        );
        assert_eq!(
            wallet_data.get("title").unwrap().as_str().unwrap(),
            "Test Wallet"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_wallet_page_has_platform_and_os() {
        let dir = create_test_dir("wallet_plat_os");
        setup_full_fixture(&dir);

        let pages = generate_wallet_pages(&dir).unwrap();
        let wallet_page = pages
            .iter()
            .find(|p| p.url == "/en/wallets/desktop/linux/testwallet/index.html")
            .expect("Missing wallet page");

        // page.platform should have name
        let platform = wallet_page
            .front_matter
            .get("platform")
            .expect("Missing platform");
        let plat_mapping = platform.as_mapping().expect("platform should be a mapping");
        let name_val = plat_mapping
            .get(serde_yaml::Value::String("name".to_string()))
            .expect("platform missing name");
        assert_eq!(name_val.as_str().unwrap(), "desktop");

        // page.os should have name and check data
        let os = wallet_page.front_matter.get("os").expect("Missing os");
        let os_mapping = os.as_mapping().expect("os should be a mapping");
        let os_name = os_mapping
            .get(serde_yaml::Value::String("name".to_string()))
            .expect("os missing name");
        assert_eq!(os_name.as_str().unwrap(), "linux");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_wallet_page_id_format() {
        let dir = create_test_dir("wallet_id");
        setup_full_fixture(&dir);

        let pages = generate_wallet_pages(&dir).unwrap();
        let wallet_page = pages
            .iter()
            .find(|p| p.url == "/en/wallets/desktop/linux/testwallet/index.html")
            .expect("Missing wallet page");

        assert_eq!(
            wallet_page
                .front_matter
                .get("id")
                .unwrap()
                .as_str()
                .unwrap(),
            "wallets-desktop-linux-testwallet"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_wallet_page_lang_set() {
        let dir = create_test_dir("wallet_lang");
        setup_full_fixture(&dir);

        let pages = generate_wallet_pages(&dir).unwrap();
        let wallet_page = pages
            .iter()
            .find(|p| p.url == "/en/wallets/desktop/linux/testwallet/index.html")
            .expect("Missing wallet page");

        assert_eq!(
            wallet_page
                .front_matter
                .get("lang")
                .unwrap()
                .as_str()
                .unwrap(),
            "en"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    // --- Multi-language ---

    #[test]
    fn test_multi_language_generation() {
        let dir = create_test_dir("multi_lang");
        setup_full_fixture(&dir);

        // Add French translations
        fs::write(
            dir.join("_translations/fr.yml"),
            r#"fr:
  url:
    index: ""
  choose-your-wallet:
    title: "Choisissez votre portefeuille - Bitcoin"
    walletcatdesktop: "Bureau"
    walletcathardware: "Mat\u00e9riel"
    platformwindows: "Windows"
    platformlinux: "Linux"
"#,
        )
        .unwrap();

        let pages = generate_wallet_pages(&dir).unwrap();

        // Should have pages for both languages
        let en_page = pages
            .iter()
            .find(|p| p.url == "/en/wallets/desktop/linux/testwallet/index.html");
        assert!(en_page.is_some(), "Missing English wallet page");

        let fr_page = pages
            .iter()
            .find(|p| p.url == "/fr/wallets/desktop/linux/testwallet/index.html");
        assert!(fr_page.is_some(), "Missing French wallet page");

        // Check French page has correct lang
        assert_eq!(
            fr_page
                .unwrap()
                .front_matter
                .get("lang")
                .unwrap()
                .as_str()
                .unwrap(),
            "fr"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    // --- Edge cases ---

    #[test]
    fn test_wallet_with_no_valid_platform_name() {
        let dir = create_test_dir("no_plat_name");
        setup_full_fixture(&dir);

        // Write a wallet with empty platform name
        fs::write(
            dir.join("_wallets/broken.md"),
            r#"---
id: broken
title: "Broken Wallet"
platform:
  - name: ""
    os:
      - name: linux
        text: "test"
---
"#,
        )
        .unwrap();

        // Should not crash
        let pages = generate_wallet_pages(&dir);
        assert!(
            pages.is_ok(),
            "Should handle empty platform name gracefully"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_unicode_wallet_title() {
        let dir = create_test_dir("unicode_wallet");
        setup_full_fixture(&dir);

        fs::write(
            dir.join("_wallets/unicode.md"),
            "---\nid: unicode\ntitle: \"\u{00C9}lectron Portefeuille \u{6BD4}\u{7279}\u{5E01}\"\nplatform:\n  - desktop:\n    name: desktop\n    os:\n      - name: linux\n        text: \"walletunicode\"\n---\n",
        )
        .unwrap();

        let pages = generate_wallet_pages(&dir).unwrap();
        let unicode_page = pages.iter().find(|p| p.url.contains("/unicode/"));
        assert!(unicode_page.is_some(), "Should handle unicode wallet");

        let wallet_data = unicode_page
            .unwrap()
            .front_matter
            .get("wallet")
            .expect("Missing wallet data");
        assert!(
            wallet_data
                .get("title")
                .unwrap()
                .as_str()
                .unwrap()
                .contains('\u{00C9}'),
            "Unicode title should be preserved"
        );

        let _ = fs::remove_dir_all(&dir);
    }
}
