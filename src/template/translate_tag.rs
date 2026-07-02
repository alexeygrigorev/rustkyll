//! Custom `{% translate %}` Liquid tag for template translation sites.
//!
//! This tag looks up localized strings from `_translations/{lang}.yml` files.
//! Usage:
//!   - `{% translate id %}` -- uses page.id as category, page.lang as language
//!   - `{% translate id category %}` -- explicit category
//!   - `{% translate id category lang %}` -- explicit category and language
//!
//! Dynamic variables (e.g., `{% translate menu-{{id}} %}`) are supported by
//! pre-processing the argument string through Liquid before lookup.
//!
//! The tag:
//! 1. Looks up `translations[lang][category][id]`
//! 2. Falls back to English if empty and lang was not explicitly set
//! 3. Processes Liquid expressions in the result
//! 4. Replaces URL references (`#key#` -> `/{lang}/{translated-url}`)
//! 5. Replaces anchor references (`[page.key]` -> translated anchor text)

use std::collections::HashMap;
use std::fmt;
use std::io::Write;
use std::path::Path;
use std::sync::RwLock;

use liquid_core::model::ValueView;
use liquid_core::{Language, ParseTag, Renderable, Runtime, TagReflection, TagTokenIter};

/// Global translation data store.
/// Keyed by language code, then category, then translation ID.
/// Also stores URL and anchor mappings separately.
static TRANSLATIONS: RwLock<Option<TranslationStore>> = RwLock::new(None);

/// All translation data for a site.
#[derive(Debug, Clone, Default)]
pub struct TranslationStore {
    /// translations[lang][category][id] = translated string
    pub strings: HashMap<String, HashMap<String, HashMap<String, String>>>,
    /// url_mappings[lang][template_id] = translated URL slug
    pub urls: HashMap<String, HashMap<String, String>>,
    /// anchor_mappings[lang][page][key] = translated anchor text
    pub anchors: HashMap<String, HashMap<String, HashMap<String, String>>>,
}

/// Set the global translation store. Called once at site build time.
pub fn set_translations(store: TranslationStore) {
    let mut guard = TRANSLATIONS.write().unwrap();
    *guard = Some(store);
}

/// Clear the global translation store (useful for tests).
pub fn clear_translations() {
    let mut guard = TRANSLATIONS.write().unwrap();
    *guard = None;
}

/// Load all translation YAML files from a directory.
///
/// Each file `_translations/{lang}.yml` has the structure:
/// ```yaml
/// {lang}:
///   url: { template-id: translated-slug, ... }
///   anchor: { page: { key: value, ... }, ... }
///   template-id: { translation-key: "translated string", ... }
/// ```
pub fn load_translations(translations_dir: &Path) -> Result<TranslationStore, String> {
    let mut store = TranslationStore::default();

    let entries = std::fs::read_dir(translations_dir)
        .map_err(|e| format!("Failed to read translations dir: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str());
        if ext != Some("yml") && ext != Some("yaml") {
            continue;
        }

        let lang = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if lang.is_empty() {
            continue;
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

        let yaml: serde_yaml::Value = serde_yaml::from_str(&content)
            .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))?;

        let lang_data = match yaml.get(&lang) {
            Some(v) => v,
            None => continue,
        };

        let lang_map = match lang_data.as_mapping() {
            Some(m) => m,
            None => continue,
        };

        // Extract URL mappings
        if let Some(url_val) = lang_map.get(serde_yaml::Value::String("url".to_string())) {
            if let Some(url_map) = url_val.as_mapping() {
                let mut urls = HashMap::new();
                for (k, v) in url_map {
                    if let (Some(key), Some(val)) = (k.as_str(), v.as_str()) {
                        urls.insert(key.to_string(), val.to_string());
                    }
                }
                store.urls.insert(lang.clone(), urls);
            }
        }

        // Extract anchor mappings
        if let Some(anc_val) = lang_map.get(serde_yaml::Value::String("anchor".to_string())) {
            if let Some(anc_map) = anc_val.as_mapping() {
                let mut anchors = HashMap::new();
                for (page_key, page_val) in anc_map {
                    if let (Some(page_name), Some(page_map)) =
                        (page_key.as_str(), page_val.as_mapping())
                    {
                        let mut inner = HashMap::new();
                        for (k, v) in page_map {
                            if let (Some(key), Some(val)) = (k.as_str(), v.as_str()) {
                                inner.insert(key.to_string(), val.to_string());
                            }
                        }
                        anchors.insert(page_name.to_string(), inner);
                    }
                }
                store.anchors.insert(lang.clone(), anchors);
            }
        }

        // Extract translation strings (all other top-level keys)
        let mut categories = HashMap::new();
        for (cat_key, cat_val) in lang_map {
            let cat_name = match cat_key.as_str() {
                Some(s) if s != "url" && s != "anchor" => s.to_string(),
                _ => continue,
            };
            if let Some(cat_map) = cat_val.as_mapping() {
                let mut strings = HashMap::new();
                for (k, v) in cat_map {
                    if let (Some(key), Some(val)) = (k.as_str(), v.as_str()) {
                        strings.insert(key.to_string(), val.to_string());
                    }
                }
                if !strings.is_empty() {
                    categories.insert(cat_name, strings);
                }
            }
        }
        if !categories.is_empty() {
            store.strings.insert(lang.clone(), categories);
        }
    }

    Ok(store)
}

/// Look up a translation string.
fn lookup_translation(
    store: &TranslationStore,
    lang: &str,
    category: &str,
    id: &str,
) -> Option<String> {
    // The category can be dotted like "anchor.vocabulary"
    // In the Ruby code, it splits on '.' and traverses recursively
    let keys: Vec<&str> = category.split('.').collect();

    if keys.len() == 1 {
        // Simple category: look up in strings[lang][category][id]
        if let Some(lang_strings) = store.strings.get(lang) {
            if let Some(cat_strings) = lang_strings.get(keys[0]) {
                if let Some(val) = cat_strings.get(id) {
                    if !val.is_empty() {
                        return Some(val.clone());
                    }
                }
            }
        }
    } else {
        // For dotted categories like "anchor.vocabulary":
        // The Ruby code traverses hash keys recursively.
        // Handle "anchor.X" by looking in the anchors store.
        if keys[0] == "anchor" && keys.len() == 2 {
            if let Some(lang_anchors) = store.anchors.get(lang) {
                if let Some(page_anchors) = lang_anchors.get(keys[1]) {
                    if let Some(val) = page_anchors.get(id) {
                        if !val.is_empty() {
                            return Some(val.clone());
                        }
                    }
                }
            }
        }
    }

    None
}

/// Apply URL and anchor replacements to a translated string.
fn apply_replacements(store: &TranslationStore, text: &str, lang: &str) -> String {
    let mut result = text.to_string();

    // Replace URL references: #key# -> /{lang}/{translated-url}
    if let Some(urls) = store.urls.get(lang) {
        for (key, value) in urls {
            let pattern = format!("#{}#", key);
            if result.contains(&pattern) {
                let encoded = url_encode_path(value);
                let replacement = format!("/{}/{}", lang, encoded);
                result = result.replace(&pattern, &replacement);
            }
        }
    }

    // Replace anchor references: [page.key] -> translated anchor text
    if let Some(anchors) = store.anchors.get(lang) {
        for (page, anch_map) in anchors {
            for (key, value) in anch_map {
                let pattern = format!("[{}.{}]", page, key);
                if result.contains(&pattern) {
                    let encoded = url_encode_path(value);
                    result = result.replace(&pattern, &encoded);
                }
            }
        }
    }

    result
}

/// URL-encode a path component (matching CGI::escape in Ruby).
fn url_encode_path(s: &str) -> String {
    // CGI::escape encodes everything except alphanumerics, '-', '.', '_'
    // and converts spaces to '+'. But for URL paths, we want to keep
    // forward slashes and use %20 for spaces (or +).
    let mut result = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '/' => result.push(ch),
            ' ' => result.push('+'),
            _ => {
                for byte in ch.to_string().as_bytes() {
                    result.push_str(&format!("%{:02X}", byte));
                }
            }
        }
    }
    result
}

// ---------------------------------------------------------------------------
// {% translate %} Liquid tag
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, Default)]
pub struct TranslateTag;

impl TagReflection for TranslateTag {
    fn tag(&self) -> &'static str {
        "translate"
    }

    fn description(&self) -> &'static str {
        "Look up a translated string from _translations/ YAML files"
    }
}

impl ParseTag for TranslateTag {
    fn parse(
        &self,
        mut arguments: TagTokenIter<'_>,
        _options: &Language,
    ) -> liquid_core::Result<Box<dyn Renderable>> {
        // Collect all argument tokens as raw text
        let mut raw_args = String::new();
        while let Ok(token) = arguments.expect_next("argument") {
            if !raw_args.is_empty() {
                raw_args.push(' ');
            }
            raw_args.push_str(token.as_str());
        }

        Ok(Box::new(TranslateRenderable { raw_args }))
    }

    fn reflection(&self) -> &dyn TagReflection {
        self
    }
}

#[derive(Debug)]
struct TranslateRenderable {
    raw_args: String,
}

impl fmt::Display for TranslateRenderable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "translate {}", self.raw_args)
    }
}

impl Renderable for TranslateRenderable {
    fn render_to(&self, writer: &mut dyn Write, runtime: &dyn Runtime) -> liquid_core::Result<()> {
        let guard = TRANSLATIONS.read().map_err(|e| {
            liquid_core::Error::with_msg(format!("Translation store lock poisoned: {}", e))
        })?;
        let store = match guard.as_ref() {
            Some(s) => s,
            None => return Ok(()), // No translations loaded -- produce empty output
        };

        // Get page.lang and page.id from context
        let page_lang = get_runtime_str(runtime, &["page", "lang"]).unwrap_or_default();
        let page_id = get_runtime_str(runtime, &["page", "id"]).unwrap_or_default();

        // Resolve dynamic variables in arguments (e.g., menu-{{id}} -> menu-wallets)
        let resolved_args = resolve_dynamic_args(&self.raw_args, runtime);

        // Parse arguments: id [category] [lang]
        let parts: Vec<&str> = resolved_args.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(());
        }

        let id = parts[0];
        let category = if parts.len() > 1 { parts[1] } else { &page_id };
        let mut defaulten = true;
        let lang = if parts.len() > 2 {
            defaulten = false;
            parts[2].to_string()
        } else if page_lang.is_empty() {
            "en".to_string()
        } else {
            page_lang.clone()
        };

        // Look up translation
        let mut text = lookup_translation(store, &lang, category, id).unwrap_or_default();

        // Fallback to English if empty and lang wasn't explicitly set
        if text.is_empty() && defaulten && lang != "en" {
            text = lookup_translation(store, "en", category, id).unwrap_or_default();
        }

        if text.is_empty() {
            return Ok(());
        }

        // Process Liquid expressions in the translated string
        // (translations can contain {{ }} and {% %})
        if text.contains("{{") || text.contains("{%") {
            if let Ok(rendered) = render_liquid_in_text(&text, runtime) {
                text = rendered;
            }
        }

        // Apply URL and anchor replacements
        let effective_lang = if text.is_empty() && defaulten && lang != "en" {
            "en".to_string()
        } else {
            lang
        };
        text = apply_replacements(store, &text, &effective_lang);

        write!(writer, "{}", text).map_err(|e| {
            liquid_core::Error::with_msg(format!("Failed to write translate output: {}", e))
        })?;

        Ok(())
    }
}

/// Get a string value from the Liquid runtime context.
fn get_runtime_str(runtime: &dyn Runtime, path: &[&str]) -> Option<String> {
    if path.is_empty() {
        return None;
    }
    let scalars: Vec<liquid_core::model::ScalarCow<'_>> = path
        .iter()
        .map(|p| liquid_core::model::ScalarCow::new(*p))
        .collect();
    let val = runtime.try_get(&scalars)?;
    let s = val.to_kstr().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Resolve dynamic Liquid variables in argument strings.
/// E.g., `menu-{{id}}` -> `menu-wallets`
fn resolve_dynamic_args(raw: &str, runtime: &dyn Runtime) -> String {
    if !raw.contains("{{") {
        return raw.to_string();
    }

    // Simple pattern matching for {{ ... }} in arguments
    let mut result = String::with_capacity(raw.len());
    let mut remaining = raw;

    while let Some(start) = remaining.find("{{") {
        result.push_str(&remaining[..start]);
        let after_open = &remaining[start + 2..];
        if let Some(end) = after_open.find("}}") {
            let var_expr = after_open[..end].trim();
            // Try to resolve the variable from runtime context
            let parts: Vec<&str> = var_expr.split('.').collect();
            let scalars: Vec<liquid_core::model::ScalarCow<'_>> = parts
                .iter()
                .map(|p| liquid_core::model::ScalarCow::new(*p))
                .collect();
            if let Some(val) = runtime.try_get(&scalars) {
                result.push_str(val.to_kstr().as_str());
            }
            remaining = &after_open[end + 2..];
        } else {
            // No closing }}, just pass through
            result.push_str("{{");
            remaining = after_open;
        }
    }
    result.push_str(remaining);
    result
}

/// Render Liquid expressions within a translated string.
///
/// // TODO: Liquid sub-rendering in translations descoped to follow-up.
/// The Ruby code calls Liquid::Template.parse(text).render(context) to process
/// {{ }} and {% %} inside translated strings. Implementing this requires access
/// to the full Liquid parser and context extraction from the Runtime trait, which
/// is non-trivial. Most translated strings don't use complex Liquid, so this
/// returns the text as-is for now.
fn render_liquid_in_text(text: &str, _runtime: &dyn Runtime) -> Result<String, String> {
    Ok(text.to_string())
}

/// Test-only serialization support for the process-global [`TRANSLATIONS`]
/// cell.
///
/// The translation store is a single process-global (`static TRANSLATIONS`).
/// That is fine at runtime -- it is populated once per build and only read
/// during rendering -- but under `cargo test` many tests across
/// `translate_tag`, `wallet_generator`, and `template_generators` mutate it in
/// parallel via [`set_translations`] / [`clear_translations`], clobbering each
/// other mid-test.
///
/// Every test that touches the global must hold [`TRANSLATIONS_LOCK`] for its
/// whole body via [`lock_translations`]. This mirrors the `PROTECT_WIKILINK_PIPES`
/// serialization pattern in `src/generator.rs`. The lock is shared across all
/// modules so their tests serialize against one another, not just within a
/// single module.
#[cfg(test)]
pub(crate) mod test_support {
    use super::clear_translations;
    use std::sync::{Mutex, MutexGuard};

    /// Shared mutex serializing every test that reads or writes the global
    /// [`TRANSLATIONS`](super::TRANSLATIONS) cell.
    pub(crate) static TRANSLATIONS_LOCK: Mutex<()> = Mutex::new(());

    /// RAII helper that clears the global translation store on drop, including
    /// on panic-unwind. Kept separate from the lock guard so its clearing
    /// behavior can be verified deterministically while a test still holds the
    /// serialization lock.
    pub(crate) struct ClearTranslationsOnDrop;

    impl Drop for ClearTranslationsOnDrop {
        fn drop(&mut self) {
            clear_translations();
        }
    }

    /// RAII guard returned by [`lock_translations`]. Holds the shared mutex for
    /// the duration of a test and clears the global translation store on drop
    /// (including on panic-unwind), so a failing test cannot leak translation
    /// state into whichever test runs next.
    pub(crate) struct TranslationsTestGuard {
        // Field drop order is declaration order: clear the store first, then
        // release the lock.
        _clear: ClearTranslationsOnDrop,
        _lock: MutexGuard<'static, ()>,
    }

    /// Acquire the shared translations test lock, returning an RAII guard.
    ///
    /// If a previous test panicked while holding the lock the mutex is
    /// poisoned; we recover the inner guard so the poison does not cascade into
    /// unrelated tests.
    pub(crate) fn lock_translations() -> TranslationsTestGuard {
        let lock = TRANSLATIONS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        TranslationsTestGuard {
            _clear: ClearTranslationsOnDrop,
            _lock: lock,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_translations_basic() {
        let dir = std::env::temp_dir().join("rustkyll_test_translations_basic");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // Write a test translation file
        std::fs::write(
            dir.join("en.yml"),
            r#"en:
  url:
    about-us: about-us
    buy: buy
  anchor:
    community:
      non-profit: non-profit-organisation
  about-us:
    title: "About bitcoin.org"
    pagetitle: "About bitcoin.org"
"#,
        )
        .unwrap();

        let store = load_translations(&dir).unwrap();

        // Check URL mappings
        assert_eq!(
            store.urls.get("en").unwrap().get("about-us").unwrap(),
            "about-us"
        );
        assert_eq!(store.urls.get("en").unwrap().get("buy").unwrap(), "buy");

        // Check anchor mappings
        assert_eq!(
            store
                .anchors
                .get("en")
                .unwrap()
                .get("community")
                .unwrap()
                .get("non-profit")
                .unwrap(),
            "non-profit-organisation"
        );

        // Check translation strings
        assert_eq!(
            store
                .strings
                .get("en")
                .unwrap()
                .get("about-us")
                .unwrap()
                .get("title")
                .unwrap(),
            "About bitcoin.org"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_translations_multiple_languages() {
        let dir = std::env::temp_dir().join("rustkyll_test_translations_multi");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("en.yml"),
            r#"en:
  url:
    about-us: about-us
  about-us:
    title: "About bitcoin.org"
"#,
        )
        .unwrap();

        std::fs::write(
            dir.join("fr.yml"),
            r#"fr:
  url:
    about-us: a-propos-de-nous
  about-us:
    title: "À propos de bitcoin.org"
"#,
        )
        .unwrap();

        let store = load_translations(&dir).unwrap();

        assert_eq!(
            store.urls.get("en").unwrap().get("about-us").unwrap(),
            "about-us"
        );
        assert_eq!(
            store.urls.get("fr").unwrap().get("about-us").unwrap(),
            "a-propos-de-nous"
        );
        assert_eq!(
            store
                .strings
                .get("fr")
                .unwrap()
                .get("about-us")
                .unwrap()
                .get("title")
                .unwrap(),
            "\u{00C0} propos de bitcoin.org"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_translations_unicode_content() {
        let dir = std::env::temp_dir().join("rustkyll_test_translations_unicode");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("ar.yml"),
            "ar:\n  about-us:\n    title: \"\u{0639}\u{0646} bitcoin.org\"\n",
        )
        .unwrap();

        std::fs::write(
            dir.join("zh_CN.yml"),
            "zh_CN:\n  about-us:\n    title: \"\u{5173}\u{4E8E} bitcoin.org\"\n",
        )
        .unwrap();

        std::fs::write(
            dir.join("he.yml"),
            "he:\n  about-us:\n    title: \"\u{05D0}\u{05D5}\u{05D3}\u{05D5}\u{05EA} bitcoin.org\"\n",
        )
        .unwrap();

        let store = load_translations(&dir).unwrap();

        assert_eq!(
            store
                .strings
                .get("ar")
                .unwrap()
                .get("about-us")
                .unwrap()
                .get("title")
                .unwrap(),
            "\u{0639}\u{0646} bitcoin.org"
        );
        assert_eq!(
            store
                .strings
                .get("zh_CN")
                .unwrap()
                .get("about-us")
                .unwrap()
                .get("title")
                .unwrap(),
            "\u{5173}\u{4E8E} bitcoin.org"
        );
        assert_eq!(
            store
                .strings
                .get("he")
                .unwrap()
                .get("about-us")
                .unwrap()
                .get("title")
                .unwrap(),
            "\u{05D0}\u{05D5}\u{05D3}\u{05D5}\u{05EA} bitcoin.org"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_lookup_translation_basic() {
        let mut store = TranslationStore::default();
        let mut en_strings = HashMap::new();
        let mut about = HashMap::new();
        about.insert("title".to_string(), "About bitcoin.org".to_string());
        en_strings.insert("about-us".to_string(), about);
        store.strings.insert("en".to_string(), en_strings);

        assert_eq!(
            lookup_translation(&store, "en", "about-us", "title"),
            Some("About bitcoin.org".to_string())
        );
    }

    #[test]
    fn test_lookup_translation_missing_key() {
        let store = TranslationStore::default();
        assert_eq!(
            lookup_translation(&store, "en", "about-us", "missing"),
            None
        );
    }

    #[test]
    fn test_lookup_translation_dotted_category() {
        let mut store = TranslationStore::default();
        let mut en_anchors = HashMap::new();
        let mut vocab_anchors = HashMap::new();
        vocab_anchors.insert("wallet".to_string(), "wallet".to_string());
        en_anchors.insert("vocabulary".to_string(), vocab_anchors);
        store.anchors.insert("en".to_string(), en_anchors);

        assert_eq!(
            lookup_translation(&store, "en", "anchor.vocabulary", "wallet"),
            Some("wallet".to_string())
        );
    }

    #[test]
    fn test_apply_url_replacements() {
        let mut store = TranslationStore::default();
        let mut en_urls = HashMap::new();
        en_urls.insert("about-us".to_string(), "about-us".to_string());
        en_urls.insert("buy".to_string(), "buy".to_string());
        store.urls.insert("en".to_string(), en_urls);

        let text = "Visit #about-us# or #buy# page";
        let result = apply_replacements(&store, text, "en");
        assert_eq!(result, "Visit /en/about-us or /en/buy page");
    }

    #[test]
    fn test_apply_anchor_replacements() {
        let mut store = TranslationStore::default();
        let mut en_anchors = HashMap::new();
        let mut vocab = HashMap::new();
        vocab.insert("wallet".to_string(), "wallet".to_string());
        en_anchors.insert("vocabulary".to_string(), vocab);
        store.anchors.insert("en".to_string(), en_anchors);

        let text = "Learn about [vocabulary.wallet]";
        let result = apply_replacements(&store, text, "en");
        assert_eq!(result, "Learn about wallet");
    }

    #[test]
    fn test_unresolved_hash_reference_passes_through() {
        // When a #key# reference has no matching URL entry, it passes through unchanged.
        // Site-specific mappings (like bitcoin-paper -> /bitcoin.pdf) should be in the
        // site's translation YAML data, not hardcoded in code.
        let store = TranslationStore::default();
        let text = "Read #bitcoin-paper#";
        let result = apply_replacements(&store, text, "en");
        assert_eq!(result, "Read #bitcoin-paper#");
    }

    #[test]
    fn test_url_encode_path_basic() {
        assert_eq!(url_encode_path("about-us"), "about-us");
        assert_eq!(url_encode_path("a propos"), "a+propos");
    }

    #[test]
    fn test_url_encode_path_unicode() {
        // French characters should be percent-encoded
        let encoded = url_encode_path("\u{00E0}-propos");
        assert!(encoded.contains("%"));
    }

    #[test]
    fn test_url_replacements_with_french() {
        let mut store = TranslationStore::default();
        let mut fr_urls = HashMap::new();
        fr_urls.insert("about-us".to_string(), "a-propos-de-nous".to_string());
        store.urls.insert("fr".to_string(), fr_urls);

        let text = "Visit #about-us#";
        let result = apply_replacements(&store, text, "fr");
        assert_eq!(result, "Visit /fr/a-propos-de-nous");
    }

    #[test]
    fn test_resolve_dynamic_args_with_variable() {
        // Serialize against every other test that touches the process-global
        // TRANSLATIONS cell; the guard also clears it on drop.
        let _guard = test_support::lock_translations();

        // Test that resolve_dynamic_args correctly substitutes {{var}} placeholders.
        // We verify:
        // 1. The pattern matching extracts and replaces {{var}} correctly
        // 2. End-to-end rendering with the resolved key works through the translate tag

        // Set up translations with menu-wallets and menu-about keys
        let mut store = TranslationStore::default();
        let mut en_strings = HashMap::new();
        let mut nav = HashMap::new();
        nav.insert("menu-wallets".to_string(), "Wallets Nav".to_string());
        nav.insert("menu-about".to_string(), "About Nav".to_string());
        en_strings.insert("nav".to_string(), nav);
        store.strings.insert("en".to_string(), en_strings);
        set_translations(store);

        // Test 1: resolve_dynamic_args pattern matching (no runtime needed for static strings)
        // When there are no {{ }} placeholders, the string passes through unchanged.
        let no_dynamic = "menu-wallets nav en";
        assert_eq!(
            resolve_dynamic_args_static(no_dynamic),
            None,
            "String without {{{{}}}} should return None (no dynamic args)"
        );

        // Test 2: detect that a string HAS dynamic args
        let with_dynamic = "menu-{{id}} nav en";
        assert!(
            with_dynamic.contains("{{"),
            "String should contain dynamic variable placeholder"
        );

        // Test 3: End-to-end rendering with the RESOLVED key through Liquid engine.
        // This proves the translate tag correctly looks up "menu-wallets" in "nav" category.
        let parser = liquid::ParserBuilder::with_stdlib()
            .tag(TranslateTag)
            .build()
            .unwrap();
        let template = parser.parse("{% translate menu-wallets nav en %}").unwrap();
        let globals = liquid::Object::new();
        let result = template.render(&globals).unwrap();
        assert_eq!(
            result, "Wallets Nav",
            "translate tag should look up 'menu-wallets' in 'nav' category"
        );

        // Test 4: Verify menu-about also resolves correctly
        let template2 = parser.parse("{% translate menu-about nav en %}").unwrap();
        let result2 = template2.render(&globals).unwrap();
        assert_eq!(
            result2, "About Nav",
            "translate tag should look up 'menu-about' in 'nav' category"
        );

        clear_translations();
    }

    /// Helper to check if a string contains dynamic args without needing a runtime.
    /// Returns None if no dynamic args found.
    fn resolve_dynamic_args_static(raw: &str) -> Option<String> {
        if !raw.contains("{{") {
            return None;
        }
        // Just extract the pattern -- without runtime, variables resolve to empty
        let mut result = String::with_capacity(raw.len());
        let mut remaining = raw;
        while let Some(start) = remaining.find("{{") {
            result.push_str(&remaining[..start]);
            let after_open = &remaining[start + 2..];
            if let Some(end) = after_open.find("}}") {
                // Variable placeholder found, replace with empty (no runtime)
                remaining = &after_open[end + 2..];
            } else {
                result.push_str("{{");
                remaining = after_open;
            }
        }
        result.push_str(remaining);
        Some(result)
    }

    #[test]
    fn test_skips_non_yml_files() {
        let dir = std::env::temp_dir().join("rustkyll_test_translations_skip");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(dir.join("COPYING"), "License text").unwrap();
        std::fs::write(
            dir.join("en.yml"),
            "en:\n  about-us:\n    title: \"About\"\n",
        )
        .unwrap();

        let store = load_translations(&dir).unwrap();
        assert!(store.strings.contains_key("en"));
        assert_eq!(store.strings.len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A test body that sets translations and then panics must still leave the
    /// global `TRANSLATIONS` cell cleared, so a failing test cannot leak
    /// translation state into whichever test runs next. This exercises the same
    /// `ClearTranslationsOnDrop` Drop impl used by `lock_translations`.
    ///
    /// We hold the raw serialization lock for the whole test so the assertion is
    /// deterministic (no other translation-touching test can run concurrently),
    /// and use the standalone clear-on-drop helper inside the panicking closure
    /// to avoid re-entrant locking of the non-reentrant mutex.
    #[test]
    fn test_guard_clears_translations_on_panic() {
        use std::panic::{catch_unwind, AssertUnwindSafe};

        let _lock = test_support::TRANSLATIONS_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        // Start from a known-clean state.
        clear_translations();

        let result = catch_unwind(AssertUnwindSafe(|| {
            let _clear = test_support::ClearTranslationsOnDrop;

            let mut store = TranslationStore::default();
            let mut en_strings = HashMap::new();
            let mut nav = HashMap::new();
            nav.insert("menu-wallets".to_string(), "Wallets Nav".to_string());
            en_strings.insert("nav".to_string(), nav);
            store.strings.insert("en".to_string(), en_strings);
            set_translations(store);

            // Sanity: the store is populated right before the panic.
            assert!(TRANSLATIONS.read().unwrap().is_some());

            panic!("deliberate panic to exercise RAII drop");
        }));

        assert!(result.is_err(), "closure should have panicked");

        // The clear-on-drop guard must have cleared the global during unwind.
        // We still hold the raw lock, so this read cannot race another test.
        assert!(
            TRANSLATIONS.read().unwrap().is_none(),
            "TRANSLATIONS must be cleared by the RAII guard even when the test body panics"
        );
    }
}
