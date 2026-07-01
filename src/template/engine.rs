use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::{OnceLock, RwLock};

use liquid::model::{ArrayView, DisplayCow, KStringCow, ObjectView, State, Value, ValueView};
use liquid::partials::{EagerCompiler, InMemorySource};
use liquid::Object;

use super::error::TemplateError;
use super::filters;

/// A parsed Liquid template ready for rendering.
pub struct Template {
    inner: liquid::Template,
}

/// A Liquid `Value` wrapped to return `Nil` for missing keys at any nesting
/// level. This matches Jekyll's lenient behavior where undefined variables
/// silently render as empty strings.
///
/// `LenientValue` wraps both scalar values (passing through unchanged) and
/// object values (intercepting `get()` to return `Nil` for missing keys).
/// Nested objects are recursively wrapped.
///
/// For performance, `LenientValue` can be built once for large shared contexts
/// (like `site`) and reused across many renders, avoiding O(n^2) deep-cloning.
pub struct LenientValue {
    /// The original value.
    inner: Value,
    /// Lazily wrapped child objects (for returning references from `get()`).
    /// Built on first access to avoid upfront cost for objects whose children
    /// are never queried (e.g., page front matter fields that aren't accessed).
    children: OnceLock<std::collections::HashMap<String, LenientValue>>,
    /// Lazily wrapped array elements (for returning references from array iteration).
    /// Built on first `values()` or `get()` call to avoid wrapping large arrays
    /// (like podcast transcripts with 200+ items) when they're never iterated.
    array_children: OnceLock<Vec<LenientValue>>,
    /// Lazily computed positional children for integer-indexed access on objects.
    /// Each entry is a two-element array `[key_string, value]` matching Jekyll's
    /// `hash[0]` behavior. Only populated on first integer-index access.
    positional_children: OnceLock<Vec<LenientValue>>,
    /// A Nil value we can hand out references to for missing keys.
    nil: Value,
}

// LenientValue is Sync because all fields are Sync:
// - inner (Value), nil (Value) are Sync
// - children, array_children, positional_children use OnceLock for safe lazy initialization

impl LenientValue {
    /// Build a `LenientValue` node from a `Value`.
    ///
    /// Children (object keys, array elements, positional pairs) are all lazily
    /// initialized on first access. This makes construction O(1) regardless of
    /// the depth or breadth of the value tree. For shared contexts (like `site`),
    /// the lazy children are built once and reused across all renders. For
    /// per-render contexts (like `page`), unused children are never built at all.
    pub fn from_value(value: Value) -> Self {
        Self {
            inner: value,
            children: OnceLock::new(),
            array_children: OnceLock::new(),
            positional_children: OnceLock::new(),
            nil: Value::Nil,
        }
    }

    /// Get or lazily initialize object children.
    fn get_children(&self) -> &std::collections::HashMap<String, LenientValue> {
        self.children.get_or_init(|| {
            if let Value::Object(ref obj) = self.inner {
                let mut map = std::collections::HashMap::with_capacity(obj.size() as usize);
                for (key, val) in obj.iter() {
                    map.insert(key.to_string(), LenientValue::from_value(val.to_value()));
                }
                map
            } else {
                std::collections::HashMap::new()
            }
        })
    }

    /// Get or lazily initialize array children.
    fn get_array_children(&self) -> &[LenientValue] {
        self.array_children.get_or_init(|| {
            if let Value::Array(ref arr) = self.inner {
                arr.iter()
                    .map(|v| LenientValue::from_value(v.to_value()))
                    .collect()
            } else {
                Vec::new()
            }
        })
    }

    /// Get or lazily initialize positional children for integer-indexed access.
    ///
    /// Only builds the positional children on first call, avoiding the cost
    /// for objects that are never integer-indexed (which is the common case).
    fn get_positional_children(&self) -> &[LenientValue] {
        self.positional_children.get_or_init(|| {
            if let Value::Object(ref obj) = self.inner {
                obj.iter()
                    .map(|(key, val)| {
                        let pair =
                            Value::Array(vec![Value::scalar(key.to_string()), val.to_value()]);
                        LenientValue::from_value(pair)
                    })
                    .collect()
            } else {
                Vec::new()
            }
        })
    }
}

impl std::fmt::Debug for LenientValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl ValueView for LenientValue {
    fn as_debug(&self) -> &dyn std::fmt::Debug {
        self
    }

    fn render(&self) -> DisplayCow<'_> {
        self.inner.render()
    }

    fn source(&self) -> DisplayCow<'_> {
        self.inner.source()
    }

    fn type_name(&self) -> &'static str {
        self.inner.type_name()
    }

    fn query_state(&self, state: State) -> bool {
        self.inner.query_state(state)
    }

    /// Issue 579: Delegate is_nil() to the wrapped value.
    ///
    /// The default `ValueView::is_nil()` always returns `false`. Without
    /// this override, a `LenientValue` wrapping `Value::Nil` (e.g.,
    /// `site.url` when `url:` is absent from `_config.yml`) would report
    /// `is_nil() == false`, preventing the SEO tag from detecting the
    /// absent URL and suppressing canonical/og:url.
    fn is_nil(&self) -> bool {
        self.inner.is_nil()
    }

    fn to_kstr(&self) -> KStringCow<'_> {
        self.inner.to_kstr()
    }

    fn to_value(&self) -> Value {
        self.inner.to_value()
    }

    fn as_scalar(&self) -> Option<liquid::model::ScalarCow<'_>> {
        self.inner.as_scalar()
    }

    fn as_object(&self) -> Option<&dyn ObjectView> {
        if self.inner.is_object() {
            Some(self)
        } else {
            None
        }
    }

    fn as_array(&self) -> Option<&dyn ArrayView> {
        if self.inner.is_array() {
            Some(self)
        } else {
            None
        }
    }
}

impl ArrayView for LenientValue {
    fn as_value(&self) -> &dyn ValueView {
        self
    }

    fn size(&self) -> i64 {
        let children = self.get_array_children();
        children.len() as i64
    }

    fn values<'k>(&'k self) -> Box<dyn Iterator<Item = &'k dyn ValueView> + 'k> {
        let children = self.get_array_children();
        Box::new(children.iter().map(|v| v as &dyn ValueView))
    }

    fn contains_key(&self, index: i64) -> bool {
        let children = self.get_array_children();
        let len = children.len() as i64;
        let idx = if index >= 0 { index } else { len + index };
        idx >= 0 && idx < len
    }

    fn get(&self, index: i64) -> Option<&dyn ValueView> {
        let children = self.get_array_children();
        let len = children.len() as i64;
        let idx = if index >= 0 { index } else { len + index };
        if idx >= 0 && (idx as usize) < children.len() {
            Some(&children[idx as usize] as &dyn ValueView)
        } else {
            None
        }
    }
}

impl ObjectView for LenientValue {
    fn as_value(&self) -> &dyn ValueView {
        self
    }

    fn size(&self) -> i64 {
        if let Value::Object(ref obj) = self.inner {
            let raw = obj.size();
            // Exclude the __key_order metadata key from the count so that
            // templates like `site.tags.size` return the number of real keys,
            // matching Jekyll behavior.
            if obj.contains_key("__key_order") {
                raw - 1
            } else {
                raw
            }
        } else {
            0
        }
    }

    fn keys<'k>(&'k self) -> Box<dyn Iterator<Item = KStringCow<'k>> + 'k> {
        if let Value::Object(ref obj) = self.inner {
            ObjectView::keys(obj)
        } else {
            Box::new(std::iter::empty())
        }
    }

    fn values<'k>(&'k self) -> Box<dyn Iterator<Item = &'k dyn ValueView> + 'k> {
        let children = self.get_children();
        Box::new(children.values().map(|v| v as &dyn ValueView))
    }

    fn iter<'k>(&'k self) -> Box<dyn Iterator<Item = (KStringCow<'k>, &'k dyn ValueView)> + 'k> {
        let children = self.get_children();
        Box::new(
            children
                .iter()
                .map(|(k, v)| (KStringCow::from_ref(k), v as &dyn ValueView)),
        )
    }

    fn contains_key(&self, _index: &str) -> bool {
        true
    }

    fn get<'s>(&'s self, index: &str) -> Option<&'s dyn ValueView> {
        // First try normal string-key lookup.
        let children = self.get_children();
        if let Some(child) = children.get(index) {
            return Some(child as &dyn ValueView);
        }
        // For "size", "first", and "last" on objects/arrays: return None when
        // there is no actual key with that name, so that augmented_get in
        // liquid-core's find.rs can compute the correct built-in value
        // (e.g., obj.size() returns key count). Without this, the lenient nil
        // fallback would shadow the built-in .size property.
        if matches!(index, "size" | "first" | "last") {
            return None;
        }
        // Fall back to positional (integer) indexing on objects.
        // Jekyll allows `hash[0]` to return the first [key, value] pair.
        // Positional children are lazily built only when integer indexing is used.
        if self.inner.is_object() {
            if let Ok(i) = index.parse::<i64>() {
                let positional = self.get_positional_children();
                if i >= 0 && (i as usize) < positional.len() {
                    return Some(&positional[i as usize] as &dyn ValueView);
                }
                // Negative or out-of-bounds integer index returns nil.
                return Some(&self.nil as &dyn ValueView);
            }
        }
        // Missing key returns nil (lenient behavior).
        Some(&self.nil as &dyn ValueView)
    }
}

/// Wrapper around `Object` that returns `Nil` for missing keys instead of
/// causing "Unknown variable" errors. This matches Jekyll's lenient behavior
/// where undefined variables silently render as empty strings.
///
/// Nested `Object` values are recursively wrapped in `LenientValue` so that
/// accessing missing keys at any depth (e.g. `page.missing_field`) returns
/// `Nil` instead of erroring.
struct LenientObject<'a> {
    inner: &'a Object,
    /// Pre-wrapped page object (small, needs lenient key access).
    page: Option<LenientValue>,
    /// Pre-wrapped include object (small, needs lenient key access).
    include: Option<LenientValue>,
    /// Pre-wrapped site object -- either owned (built fresh) or borrowed from cache.
    /// Using an enum avoids rebuilding the expensive site LenientValue tree on every render.
    site: CachedOrOwned<'a>,
    /// Optional per-render site key overrides (e.g., per-post related_posts).
    site_with_overrides: Option<SiteWithOverrides<'a>>,
    /// A Nil value we can hand out references to for missing keys.
    nil: Value,
    /// Optional borrowed reference to a pre-built page LenientValue.
    /// When set, this takes priority over the `page` field, avoiding the
    /// expensive `to_value()` clone in `with_cached_site()`.
    page_lenient_ref: Option<&'a LenientValue>,
}

/// Wrapper that overrides specific keys in a cached site LenientValue.
pub(crate) struct SiteWithOverrides<'a> {
    base: &'a LenientValue,
    overrides: &'a HashMap<String, LenientValue>,
}

impl std::fmt::Debug for SiteWithOverrides<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.base.fmt(f)
    }
}

impl ValueView for SiteWithOverrides<'_> {
    fn as_debug(&self) -> &dyn std::fmt::Debug {
        self
    }
    fn render(&self) -> DisplayCow<'_> {
        self.base.render()
    }
    fn source(&self) -> DisplayCow<'_> {
        self.base.source()
    }
    fn type_name(&self) -> &'static str {
        self.base.type_name()
    }
    fn query_state(&self, state: State) -> bool {
        self.base.query_state(state)
    }
    fn to_kstr(&self) -> KStringCow<'_> {
        self.base.to_kstr()
    }
    fn to_value(&self) -> Value {
        self.base.to_value()
    }
    fn as_scalar(&self) -> Option<liquid::model::ScalarCow<'_>> {
        self.base.as_scalar()
    }
    fn as_object(&self) -> Option<&dyn ObjectView> {
        Some(self)
    }
    fn as_array(&self) -> Option<&dyn ArrayView> {
        self.base.as_array()
    }
}

impl ObjectView for SiteWithOverrides<'_> {
    fn as_value(&self) -> &dyn ValueView {
        self
    }
    fn size(&self) -> i64 {
        ObjectView::size(self.base)
    }
    fn keys<'k>(&'k self) -> Box<dyn Iterator<Item = KStringCow<'k>> + 'k> {
        ObjectView::keys(self.base)
    }
    fn values<'k>(&'k self) -> Box<dyn Iterator<Item = &'k dyn ValueView> + 'k> {
        ObjectView::values(self.base)
    }
    fn iter<'k>(&'k self) -> Box<dyn Iterator<Item = (KStringCow<'k>, &'k dyn ValueView)> + 'k> {
        ObjectView::iter(self.base)
    }
    fn contains_key(&self, _index: &str) -> bool {
        true
    }
    fn get<'s>(&'s self, index: &str) -> Option<&'s dyn ValueView> {
        if let Some(overridden) = self.overrides.get(index) {
            return Some(overridden as &dyn ValueView);
        }
        ObjectView::get(self.base, index)
    }
}

/// Either a borrowed reference to a pre-built `LenientValue` (for cached site context)
/// or an owned one built on the fly.
enum CachedOrOwned<'a> {
    Cached(&'a LenientValue),
    Owned(Box<LenientValue>),
    None,
}

impl<'a> CachedOrOwned<'a> {
    fn as_ref(&self) -> Option<&LenientValue> {
        match self {
            CachedOrOwned::Cached(v) => Some(v),
            CachedOrOwned::Owned(v) => Some(v),
            CachedOrOwned::None => None,
        }
    }
}

impl<'a> LenientObject<'a> {
    fn new(inner: &'a Object) -> Self {
        let page = inner
            .get("page")
            .map(|v| LenientValue::from_value(v.to_value()));
        let include = inner
            .get("include")
            .map(|v| LenientValue::from_value(v.to_value()));
        let site = match inner.get("site") {
            Some(v) => CachedOrOwned::Owned(Box::new(LenientValue::from_value(v.to_value()))),
            None => CachedOrOwned::None,
        };
        Self {
            inner,
            page,
            include,
            site,
            site_with_overrides: None,
            nil: Value::Nil,
            page_lenient_ref: None,
        }
    }

    /// Create a `LenientObject` using a pre-built `LenientValue` for the site context.
    ///
    /// This avoids the expensive `LenientValue::from_value()` call on the site object,
    /// which is the main O(n^2) bottleneck for large sites. The site `LenientValue`
    /// is built once and shared across all page renders.
    fn with_cached_site(inner: &'a Object, cached_site: &'a LenientValue) -> Self {
        let page = inner
            .get("page")
            .map(|v| LenientValue::from_value(v.to_value()));
        let include = inner
            .get("include")
            .map(|v| LenientValue::from_value(v.to_value()));
        Self {
            inner,
            page,
            include,
            site: CachedOrOwned::Cached(cached_site),
            site_with_overrides: None,
            nil: Value::Nil,
            page_lenient_ref: None,
        }
    }

    /// Create with cached site and per-render key overrides.
    fn with_cached_site_overrides(
        inner: &'a Object,
        cached_site: &'a LenientValue,
        site_overrides: &'a HashMap<String, LenientValue>,
    ) -> Self {
        let page = inner
            .get("page")
            .map(|v| LenientValue::from_value(v.to_value()));
        let include = inner
            .get("include")
            .map(|v| LenientValue::from_value(v.to_value()));
        let site_with_overrides = if site_overrides.is_empty() {
            None
        } else {
            Some(SiteWithOverrides {
                base: cached_site,
                overrides: site_overrides,
            })
        };
        Self {
            inner,
            page,
            include,
            site: CachedOrOwned::Cached(cached_site),
            site_with_overrides,
            nil: Value::Nil,
            page_lenient_ref: None,
        }
    }

    /// Create with a pre-built page `LenientValue` and cached site context.
    ///
    /// This avoids the expensive `to_value()` clone on the page object that
    /// happens in `with_cached_site()`. The caller pre-builds the page
    /// `LenientValue` once and passes it by reference for each render.
    fn with_prebuilt_page(
        inner: &'a Object,
        page_lenient: &'a LenientValue,
        cached_site: &'a LenientValue,
    ) -> Self {
        let include = inner
            .get("include")
            .map(|v| LenientValue::from_value(v.to_value()));
        Self {
            inner,
            page: None, // Not used -- page_lenient_ref replaces it
            include,
            site: CachedOrOwned::Cached(cached_site),
            site_with_overrides: None,
            nil: Value::Nil,
            page_lenient_ref: Some(page_lenient),
        }
    }

    /// Create with a pre-built page `LenientValue`, cached site, and overrides.
    fn with_prebuilt_page_overrides(
        inner: &'a Object,
        page_lenient: &'a LenientValue,
        cached_site: &'a LenientValue,
        site_overrides: &'a HashMap<String, LenientValue>,
    ) -> Self {
        let include = inner
            .get("include")
            .map(|v| LenientValue::from_value(v.to_value()));
        let site_with_overrides = if site_overrides.is_empty() {
            None
        } else {
            Some(SiteWithOverrides {
                base: cached_site,
                overrides: site_overrides,
            })
        };
        Self {
            inner,
            page: None,
            include,
            site: CachedOrOwned::Cached(cached_site),
            site_with_overrides,
            nil: Value::Nil,
            page_lenient_ref: Some(page_lenient),
        }
    }
}

impl std::fmt::Debug for LenientObject<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl ValueView for LenientObject<'_> {
    fn as_debug(&self) -> &dyn std::fmt::Debug {
        self
    }

    fn render(&self) -> DisplayCow<'_> {
        self.inner.render()
    }

    fn source(&self) -> DisplayCow<'_> {
        self.inner.source()
    }

    fn type_name(&self) -> &'static str {
        self.inner.type_name()
    }

    fn query_state(&self, state: State) -> bool {
        self.inner.query_state(state)
    }

    fn to_kstr(&self) -> KStringCow<'_> {
        self.inner.to_kstr()
    }

    fn to_value(&self) -> Value {
        self.inner.to_value()
    }

    fn as_object(&self) -> Option<&dyn ObjectView> {
        Some(self)
    }
}

impl ObjectView for LenientObject<'_> {
    fn as_value(&self) -> &dyn ValueView {
        self
    }

    fn size(&self) -> i64 {
        let raw = self.inner.size();
        if self.inner.contains_key("__key_order") {
            raw - 1
        } else {
            raw
        }
    }

    fn keys<'k>(&'k self) -> Box<dyn Iterator<Item = KStringCow<'k>> + 'k> {
        ObjectView::keys(&self.inner)
    }

    fn values<'k>(&'k self) -> Box<dyn Iterator<Item = &'k dyn ValueView> + 'k> {
        ObjectView::values(&self.inner)
    }

    fn iter<'k>(&'k self) -> Box<dyn Iterator<Item = (KStringCow<'k>, &'k dyn ValueView)> + 'k> {
        ObjectView::iter(&self.inner)
    }

    fn contains_key(&self, _index: &str) -> bool {
        // Always claim to contain the key so the runtime never falls through
        // to the parent RuntimeCore which would error with "Unknown variable".
        true
    }

    fn get<'s>(&'s self, index: &str) -> Option<&'s dyn ValueView> {
        // Use pre-wrapped lenient versions for page, include, and site.
        // This enables lenient key access and hash integer indexing.
        match index {
            "page" => {
                // Prefer pre-built page reference (avoids to_value() clone)
                if let Some(plr) = self.page_lenient_ref {
                    return Some(plr as &dyn ValueView);
                }
                self.page.as_ref().map(|v| v as &dyn ValueView)
            }
            "include" => self.include.as_ref().map(|v| v as &dyn ValueView),
            "site" => {
                if let Some(ref overrides) = self.site_with_overrides {
                    Some(overrides as &dyn ValueView)
                } else {
                    self.site.as_ref().map(|v| v as &dyn ValueView)
                }
            }
            _ => self.inner.get(index).map(|v| v as &dyn ValueView),
        }
        .or(Some(&self.nil as &dyn ValueView))
    }
}

/// Pre-built, cached site context for efficient rendering.
///
/// On large sites (787+ pages), building a `LenientValue` tree for the site
/// context on every render is O(n^2) -- each of the N pages triggers a full
/// recursive walk of all N post objects. By building the `LenientValue` once
/// and sharing it, rendering becomes O(n).
pub struct CachedSiteContext {
    site_lenient: LenientValue,
}

impl CachedSiteContext {
    /// Build a cached site context from a site `Object`.
    ///
    /// This is the expensive operation -- it recursively wraps all values in
    /// the site Object into `LenientValue` nodes. Call this ONCE, then pass
    /// the result to every `render_with_cached_site` call.
    pub fn new(site_obj: &Object) -> Self {
        let site_value = Value::Object(site_obj.clone());
        Self {
            site_lenient: LenientValue::from_value(site_value),
        }
    }

    /// Build a cached site context by taking ownership of the site `Object`.
    ///
    /// This avoids cloning the entire site Object (which can be expensive
    /// for large sites with hundreds of collection items). Use this when
    /// the caller no longer needs the Object after building the cache.
    pub fn from_object(site_obj: Object) -> Self {
        let site_value = Value::Object(site_obj);
        Self {
            site_lenient: LenientValue::from_value(site_value),
        }
    }
}

/// The core template engine wrapping the `liquid` crate parser.
///
/// Provides methods to parse and render Liquid templates with Jekyll-compatible
/// behavior (e.g., undefined variables produce empty strings rather than errors).
///
/// Designed for extensibility: use `TemplateEngine::builder()` to get a
/// `liquid::ParserBuilder` that can be customized with additional filters
/// before building.
pub struct TemplateEngine {
    parser: RwLock<liquid::Parser>,
    /// Includes map for rebuilding the parser when unknown filters are encountered.
    includes: Option<HashMap<String, String>>,
    /// Whether to register include tag when rebuilding.
    has_include_tag: bool,
    /// Set of passthrough filter names registered for unknown filters.
    passthrough_filters: RwLock<HashSet<String>>,
}

impl TemplateEngine {
    /// Check if any include source references `page.previous` or `page.next`.
    pub fn uses_prev_next(&self) -> bool {
        if let Some(ref includes) = self.includes {
            for source in includes.values() {
                if source.contains("page.previous") || source.contains("page.next") {
                    return true;
                }
            }
        }
        false
    }

    /// Check if any layout or include references `site.related_posts`.
    ///
    /// Used to skip the expensive per-post related_posts computation when
    /// templates don't use it (common for sites like large-blog-3000).
    pub fn uses_related_posts(&self) -> bool {
        if let Some(ref includes) = self.includes {
            for source in includes.values() {
                if source.contains("related_posts") {
                    return true;
                }
            }
        }
        false
    }

    /// Create a new `TemplateEngine` with stdlib + Jekyll filters but no includes.
    ///
    /// # Errors
    ///
    /// Returns `TemplateError::ParseError` if the parser fails to build.
    pub fn new() -> Result<Self, TemplateError> {
        let parser = Self::builder()
            .tag(super::seo_tag::SeoTag)
            .tag(super::avatar_tag::AvatarTag)
            .block(super::highlight_tag::HighlightBlock)
            .tag(super::feed_meta_tag::FeedMetaTag)
            .tag(super::noop_tags::GithubEditLinkTag)
            .tag(super::noop_tags::CiteTag)
            .tag(super::noop_tags::ReferenceTag)
            .tag(super::noop_tags::BibliographyTag)
            .tag(super::noop_tags::JupyterNotebookTag)
            .tag(super::noop_tags::SocialLinksTag)
            .tag(super::noop_tags::TwitterTag)
            .block(super::noop_tags::TabsBlock)
            .block(super::noop_tags::TabBlock)
            .block(super::noop_tags::QuoteBlock)
            .block(super::details_tag::DetailsBlock)
            .tag(super::file_exists_tag::FileExistsTag)
            .tag(super::translate_tag::TranslateTag)
            .build()
            .map_err(|e| TemplateError::ParseError(e.to_string()))?;
        Ok(Self {
            parser: RwLock::new(parser),
            includes: None,
            has_include_tag: false,
            passthrough_filters: RwLock::new(HashSet::new()),
        })
    }

    /// Create a `TemplateEngine` with includes loaded from a directory.
    ///
    /// All `.html` files in `includes_dir` (and subdirectories) are registered
    /// as partials. The Jekyll-compatible include tag is used, supporting
    /// unquoted filenames (`{% include head.html %}`), parameters with `=`,
    /// and variable parameters.
    ///
    /// # Errors
    ///
    /// Returns `TemplateError::IoError` if the directory cannot be read, or
    /// `TemplateError::ParseError` if the parser fails to build.
    pub fn with_includes(includes_dir: &Path) -> Result<Self, TemplateError> {
        let partials_map = load_includes(includes_dir)?;
        // Pre-discover unknown filters in includes so they are registered
        // as passthrough stubs BEFORE building the parser with partials.
        // The EagerCompiler defers partial parse errors to render time, so
        // without this pre-scan, unknown filters in includes silently break
        // every layout that chains through those includes.
        let passthrough_set = Self::discover_unknown_filters_in_includes(&partials_map)?;
        let partials = build_partials(&partials_map);
        let mut builder = Self::builder()
            .tag(super::include_tag::LenientIncludeTag)
            .tag(super::include_tag::LenientIncludeCachedTag)
            .tag(super::seo_tag::SeoTag)
            .tag(super::avatar_tag::AvatarTag)
            .block(super::highlight_tag::HighlightBlock)
            .tag(super::feed_meta_tag::FeedMetaTag)
            .tag(super::noop_tags::GithubEditLinkTag)
            .tag(super::noop_tags::CiteTag)
            .tag(super::noop_tags::ReferenceTag)
            .tag(super::noop_tags::BibliographyTag)
            .tag(super::noop_tags::JupyterNotebookTag)
            .tag(super::noop_tags::SocialLinksTag)
            .tag(super::noop_tags::TwitterTag)
            .block(super::noop_tags::TabsBlock)
            .block(super::noop_tags::TabBlock)
            .block(super::noop_tags::QuoteBlock)
            .block(super::details_tag::DetailsBlock)
            .tag(super::file_exists_tag::FileExistsTag)
            .tag(super::translate_tag::TranslateTag);
        for name in &passthrough_set {
            builder = builder.filter(filters::passthrough::PassthroughFilter::new(name.clone()));
        }
        let parser = builder
            .partials(partials)
            .build()
            .map_err(|e| TemplateError::ParseError(e.to_string()))?;
        Ok(Self {
            parser: RwLock::new(parser),
            includes: Some(partials_map),
            has_include_tag: true,
            passthrough_filters: RwLock::new(passthrough_set),
        })
    }

    /// Create a `TemplateEngine` with includes from a pre-built map.
    ///
    /// Useful for testing or when includes are loaded from a non-filesystem source.
    ///
    /// # Errors
    ///
    /// Returns `TemplateError::ParseError` if the parser fails to build.
    pub fn with_includes_map(includes: &HashMap<String, String>) -> Result<Self, TemplateError> {
        Self::with_includes_and_extra_sources(includes, &HashMap::new())
    }

    /// Create a `TemplateEngine` with includes and additional template sources
    /// for unknown filter discovery (e.g., layout sources).
    ///
    /// Like `with_includes_map`, but also scans `extra_sources` for unknown
    /// filters and registers them as passthrough stubs. This prevents parse
    /// failures when layouts contain filters not present in includes.
    ///
    /// # Errors
    ///
    /// Returns `TemplateError::ParseError` if the parser fails to build.
    pub fn with_includes_and_extra_sources(
        includes: &HashMap<String, String>,
        extra_sources: &HashMap<String, String>,
    ) -> Result<Self, TemplateError> {
        let mut passthrough_set = Self::discover_unknown_filters_in_includes(includes)?;
        if !extra_sources.is_empty() {
            let extra = Self::discover_unknown_filters_in_includes(extra_sources)?;
            passthrough_set.extend(extra);
        }
        let partials = build_partials(includes);
        let mut builder = Self::builder()
            .tag(super::include_tag::LenientIncludeTag)
            .tag(super::include_tag::LenientIncludeCachedTag)
            .tag(super::seo_tag::SeoTag)
            .tag(super::avatar_tag::AvatarTag)
            .block(super::highlight_tag::HighlightBlock)
            .tag(super::feed_meta_tag::FeedMetaTag)
            .tag(super::noop_tags::GithubEditLinkTag)
            .tag(super::noop_tags::CiteTag)
            .tag(super::noop_tags::ReferenceTag)
            .tag(super::noop_tags::BibliographyTag)
            .tag(super::noop_tags::JupyterNotebookTag)
            .tag(super::noop_tags::SocialLinksTag)
            .tag(super::noop_tags::TwitterTag)
            .block(super::noop_tags::TabsBlock)
            .block(super::noop_tags::TabBlock)
            .block(super::noop_tags::QuoteBlock)
            .block(super::details_tag::DetailsBlock)
            .tag(super::file_exists_tag::FileExistsTag)
            .tag(super::translate_tag::TranslateTag);
        for name in &passthrough_set {
            builder = builder.filter(filters::passthrough::PassthroughFilter::new(name.clone()));
        }
        let parser = builder
            .partials(partials)
            .build()
            .map_err(|e| TemplateError::ParseError(e.to_string()))?;
        Ok(Self {
            parser: RwLock::new(parser),
            includes: Some(includes.clone()),
            has_include_tag: true,
            passthrough_filters: RwLock::new(passthrough_set),
        })
    }

    /// Return a `ParserBuilder` pre-configured with stdlib + Jekyll filters.
    ///
    /// Use this when you need to register additional custom filters before
    /// building the parser (e.g., for Issue 07 custom filters).
    pub fn builder() -> liquid::ParserBuilder {
        liquid::ParserBuilder::with_stdlib()
            .filter(liquid_lib::jekyll::Slugify)
            .filter(liquid_lib::jekyll::Push)
            .filter(liquid_lib::jekyll::Pop)
            .filter(liquid_lib::jekyll::Unshift)
            .filter(liquid_lib::jekyll::Shift)
            .filter(filters::ArrayToSentenceString)
            // Custom filters (Issue 07)
            .filter(filters::WhereExp)
            .filter(filters::Where)
            .filter(filters::Find)
            .filter(filters::Jsonify)
            .filter(filters::DateToString)
            .filter(filters::DateToLongString)
            .filter(filters::DateToRfc822)
            .filter(filters::DateToXmlschema)
            .filter(filters::Markdownify)
            .filter(filters::NewlineToBr)
            .filter(filters::RelativeUrl)
            .filter(filters::AbsoluteUrl)
            // Missing filters (Issue 30)
            .filter(filters::NumberOfWords)
            .filter(filters::GroupBy)
            .filter(filters::GroupByExp)
            .filter(filters::XmlEscape)
            .filter(filters::Truncatewords)
            // Jekyll number_with_delimiter (ActiveSupport): thousands grouping
            // for star/fork counts etc. (Issue GH#6)
            .filter(filters::NumberWithDelimiter)
            // Missing filters (Issue 37)
            .filter(filters::NormalizeWhitespace)
            // Custom date filter that handles YYYY-MM-DD strings (Issue 72)
            .filter(filters::Date)
            // Sort with stable tie-breaking by slug (Issue 112)
            .filter(filters::Sort)
            // Jekyll-compatible URL encoding: spaces as + (Issue 178)
            // Must come after with_stdlib() to override the default url_encode (%20)
            .filter(filters::UrlEncode)
            .filter(filters::CgiEscape)
            .filter(filters::UriEscape)
            .filter(filters::UrlEscape)
            .filter(filters::Camelcase)
            // Lenient math filters: non-numeric strings coerce to 0 (Issue 196)
            // Must come after with_stdlib() to override strict versions
            .filter(filters::math::Times)
            .filter(filters::math::Plus)
            .filter(filters::math::Minus)
            .filter(filters::math::Modulo)
            // Jekyll-compatible map filter that preserves nested arrays (Issue 209/233)
            // Must come after with_stdlib() to override the default map filter
            .filter(filters::Map)
            // Ruby Liquid InputIterator compat: uniq/compact/sort flatten nested arrays
            // one level before processing, matching `map: "tags" | uniq | sort` pattern
            .filter(filters::Uniq)
            .filter(filters::Compact)
            // Ruby Liquid sample filter: random sampling from arrays (Issue 214)
            .filter(filters::Sample)
            // Jekyll-compatible strip_html: simple tag removal matching gsub(/<.*?>/m, '')
            // Must come after with_stdlib() to override the default strip_html
            .filter(filters::StripHtml)
            // Jekyll URL filter: strip trailing /index.html from URLs
            .filter(filters::StripIndex)
            // Lenient join: strings pass through unchanged (Issue 328)
            // Must come after with_stdlib() to override the strict join
            .filter(filters::Join)
            // Render YAML mapping values like Jekyll/Ruby hash strings (Issue 348)
            .filter(filters::RenderMapping)
            // Character-count size filter (Issue 517): Ruby/Jekyll `size` returns
            // character count, not byte count. The stdlib version uses str::len()
            // which is bytes. This matters for `size` + `slice` on non-ASCII content.
            .filter(filters::Size)
            // Character-based slice filter (Issue 517): stdlib `slice` uses byte count
            // for bounds-checking but chars for iteration, causing incorrect slicing
            // on non-ASCII content. This uses character count consistently.
            .filter(filters::Slice)
    }

    /// Pre-scan include sources to discover unknown Liquid filter names.
    ///
    /// Extracts all `| filter_name` patterns from include sources using regex,
    /// then tests each candidate with a temporary parser. Any that fail to
    /// parse (unknown to the liquid engine) are returned for passthrough
    /// registration.
    ///
    /// Returns the set of passthrough filter names that were discovered.
    fn discover_unknown_filters_in_includes(
        includes: &HashMap<String, String>,
    ) -> Result<HashSet<String>, TemplateError> {
        // Step 1: Extract all candidate filter names from include sources.
        // Pattern: `| word` where word is [a-z_][a-z0-9_]*
        let mut candidates: HashSet<String> = HashSet::new();
        for source in includes.values() {
            // Find all `| filter_name` patterns (inside {{ }} or {% %} tags)
            let bytes = source.as_bytes();
            let len = bytes.len();
            let mut i = 0;
            while i < len {
                if bytes[i] == b'|' {
                    // Skip whitespace after pipe
                    let mut j = i + 1;
                    while j < len && bytes[j] == b' ' {
                        j += 1;
                    }
                    // Extract word: [a-z_][a-z0-9_]*
                    if j < len && (bytes[j].is_ascii_lowercase() || bytes[j] == b'_') {
                        let start = j;
                        while j < len
                            && (bytes[j].is_ascii_lowercase()
                                || bytes[j].is_ascii_digit()
                                || bytes[j] == b'_')
                        {
                            j += 1;
                        }
                        let name = &source[start..j];
                        candidates.insert(name.to_string());
                    }
                }
                i += 1;
            }
        }

        // Also extract from layout sources that get preprocessed later
        // (not needed here since layouts are compiled separately).

        // Step 2: Test each candidate with a temporary parser.
        let parser = Self::builder()
            .tag(super::include_tag::LenientIncludeTag)
            .tag(super::include_tag::LenientIncludeCachedTag)
            .tag(super::seo_tag::SeoTag)
            .tag(super::avatar_tag::AvatarTag)
            .block(super::highlight_tag::HighlightBlock)
            .tag(super::feed_meta_tag::FeedMetaTag)
            .tag(super::noop_tags::GithubEditLinkTag)
            .tag(super::noop_tags::CiteTag)
            .tag(super::noop_tags::ReferenceTag)
            .tag(super::noop_tags::BibliographyTag)
            .tag(super::noop_tags::JupyterNotebookTag)
            .tag(super::noop_tags::SocialLinksTag)
            .tag(super::noop_tags::TwitterTag)
            .block(super::noop_tags::TabsBlock)
            .block(super::noop_tags::TabBlock)
            .block(super::noop_tags::QuoteBlock)
            .block(super::details_tag::DetailsBlock)
            .tag(super::file_exists_tag::FileExistsTag)
            .tag(super::translate_tag::TranslateTag)
            .build()
            .map_err(|e| TemplateError::ParseError(e.to_string()))?;

        let mut passthrough_names: HashSet<String> = HashSet::new();
        for name in &candidates {
            // Try parsing a simple template using this filter
            let test_template = format!("{{{{ x | {} }}}}", name);
            if let Err(e) = parser.parse(&test_template) {
                let err_str = e.to_string();
                if err_str.contains("Unknown filter") || err_str.contains("FilterChain") {
                    eprintln!(
                        "Warning: unknown filter '{}' in include, registering passthrough",
                        name
                    );
                    passthrough_names.insert(name.clone());
                }
            }
        }

        Ok(passthrough_names)
    }

    /// Create a `TemplateEngine` from a pre-built `liquid::Parser`.
    ///
    /// Useful when you've customized the parser builder with additional filters.
    pub fn from_parser(parser: liquid::Parser) -> Self {
        Self {
            parser: RwLock::new(parser),
            includes: None,
            has_include_tag: false,
            passthrough_filters: RwLock::new(HashSet::new()),
        }
    }

    /// Parse a template string into a `Template`.
    ///
    /// If parsing fails due to an unknown filter, the engine automatically
    /// registers a passthrough filter for the unknown name, rebuilds the parser,
    /// and retries. This allows templates with unrecognized filters (e.g. typos
    /// or site-specific filters) to still render, with the unknown filter
    /// passing through the input value unchanged.
    ///
    /// # Errors
    ///
    /// Returns `TemplateError::ParseError` if the template contains syntax errors
    /// that are not related to unknown filters.
    pub fn parse(&self, template_str: &str) -> Result<Template, TemplateError> {
        let preprocessed = preprocess_all(template_str);
        let template_str = &preprocessed;
        loop {
            let parser_guard = self
                .parser
                .read()
                .map_err(|e| TemplateError::ParseError(format!("lock poisoned: {}", e)))?;
            let result = parser_guard.parse(template_str);
            drop(parser_guard);
            match result {
                Ok(inner) => return Ok(Template { inner }),
                Err(e) => {
                    let err_str = e.to_string();
                    if let Some(filter_name) = extract_unknown_filter_name(&err_str) {
                        eprintln!(
                            "Warning: unknown filter '{}' encountered, registering passthrough",
                            filter_name
                        );
                        self.register_passthrough_filter(&filter_name)?;
                    } else {
                        return Err(TemplateError::ParseError(err_str));
                    }
                }
            }
        }
    }

    /// Register a passthrough filter for an unknown filter name and rebuild
    /// the parser.
    fn register_passthrough_filter(&self, name: &str) -> Result<(), TemplateError> {
        {
            let mut filters = self
                .passthrough_filters
                .write()
                .map_err(|e| TemplateError::ParseError(format!("lock poisoned: {}", e)))?;
            filters.insert(name.to_string());
        }
        self.rebuild_parser()
    }

    /// Rebuild the parser with all currently registered passthrough filters.
    fn rebuild_parser(&self) -> Result<(), TemplateError> {
        let mut builder = Self::builder();

        // Register all passthrough filters
        let filters_guard = self
            .passthrough_filters
            .read()
            .map_err(|e| TemplateError::ParseError(format!("lock poisoned: {}", e)))?;
        for name in filters_guard.iter() {
            builder = builder.filter(filters::passthrough::PassthroughFilter::new(name.clone()));
        }
        drop(filters_guard);

        // Always register seo tag, avatar tag, highlight block, and no-op plugin tags;
        // include tag only when includes are present
        builder = builder.tag(super::seo_tag::SeoTag);
        builder = builder.tag(super::avatar_tag::AvatarTag);
        builder = builder.block(super::highlight_tag::HighlightBlock);
        builder = builder.tag(super::feed_meta_tag::FeedMetaTag);
        builder = builder
            .tag(super::noop_tags::GithubEditLinkTag)
            .tag(super::noop_tags::CiteTag)
            .tag(super::noop_tags::ReferenceTag)
            .tag(super::noop_tags::BibliographyTag)
            .tag(super::noop_tags::JupyterNotebookTag)
            .tag(super::noop_tags::SocialLinksTag)
            .tag(super::noop_tags::TwitterTag)
            .block(super::noop_tags::TabsBlock)
            .block(super::noop_tags::TabBlock)
            .block(super::noop_tags::QuoteBlock)
            .block(super::details_tag::DetailsBlock)
            .tag(super::file_exists_tag::FileExistsTag)
            .tag(super::translate_tag::TranslateTag);
        if self.has_include_tag {
            builder = builder.tag(super::include_tag::LenientIncludeTag);
            builder = builder.tag(super::include_tag::LenientIncludeCachedTag);
        }
        if let Some(ref includes) = self.includes {
            builder = builder.partials(build_partials(includes));
        }

        let parser = builder
            .build()
            .map_err(|e| TemplateError::ParseError(e.to_string()))?;
        let mut parser_guard = self
            .parser
            .write()
            .map_err(|e| TemplateError::ParseError(format!("lock poisoned: {}", e)))?;
        *parser_guard = parser;
        Ok(())
    }

    /// Render a parsed template with the given context.
    ///
    /// Undefined variables produce empty strings (matching Jekyll behavior)
    /// rather than errors.
    ///
    /// # Errors
    ///
    /// Returns `TemplateError::RenderError` for render failures other than
    /// undefined variables.
    pub fn render(&self, template: &Template, context: &Object) -> Result<String, TemplateError> {
        let lenient = LenientObject::new(context);
        template
            .inner
            .render(&lenient)
            .map_err(|e| TemplateError::RenderError(e.to_string()))
    }

    /// Parse and render a template string in one step.
    ///
    /// Convenience method combining `parse()` and `render()`.
    ///
    /// # Errors
    ///
    /// Returns `TemplateError::ParseError` or `TemplateError::RenderError`.
    pub fn parse_and_render(
        &self,
        template_str: &str,
        context: &Object,
    ) -> Result<String, TemplateError> {
        let template = self.parse(template_str)?;
        self.render(&template, context)
    }

    /// Render a parsed template using a pre-built cached site context.
    ///
    /// This avoids the O(n^2) cost of rebuilding the `LenientValue` tree for
    /// the site object on every render. The `context` Object should contain
    /// `page` and `content` but NOT `site` -- the site is provided via the
    /// cached context.
    ///
    /// # Errors
    ///
    /// Returns `TemplateError::RenderError` for render failures.
    pub fn render_with_cached_site(
        &self,
        template: &Template,
        context: &Object,
        cached_site: &CachedSiteContext,
    ) -> Result<String, TemplateError> {
        let lenient = LenientObject::with_cached_site(context, &cached_site.site_lenient);
        template
            .inner
            .render(&lenient)
            .map_err(|e| TemplateError::RenderError(e.to_string()))
    }

    /// Parse and render a template string using a pre-built cached site context.
    ///
    /// Combines `parse()` and `render_with_cached_site()`.
    ///
    /// # Errors
    ///
    /// Returns `TemplateError::ParseError` or `TemplateError::RenderError`.
    pub fn parse_and_render_with_cached_site(
        &self,
        template_str: &str,
        context: &Object,
        cached_site: &CachedSiteContext,
    ) -> Result<String, TemplateError> {
        let template = self.parse(template_str)?;
        self.render_with_cached_site(&template, context, cached_site)
    }

    /// Render with cached site and per-render overrides.
    pub(crate) fn render_with_site_overrides(
        &self,
        template: &Template,
        context: &Object,
        cached_site: &CachedSiteContext,
        site_overrides: &HashMap<String, LenientValue>,
    ) -> Result<String, TemplateError> {
        let lenient = LenientObject::with_cached_site_overrides(
            context,
            &cached_site.site_lenient,
            site_overrides,
        );
        template
            .inner
            .render(&lenient)
            .map_err(|e| TemplateError::RenderError(e.to_string()))
    }

    /// Render with a pre-built page LenientValue and cached site context.
    ///
    /// This avoids the expensive `to_value()` clone on the page object that
    /// happens in `render_with_cached_site()`. The caller pre-builds the page
    /// `LenientValue` once and reuses it for layout rendering.
    pub fn render_with_prebuilt_page_lenient(
        &self,
        template: &Template,
        context: &Object,
        cached_site: &CachedSiteContext,
        page_lenient: &LenientValue,
    ) -> Result<String, TemplateError> {
        let lenient =
            LenientObject::with_prebuilt_page(context, page_lenient, &cached_site.site_lenient);
        template
            .inner
            .render(&lenient)
            .map_err(|e| TemplateError::RenderError(e.to_string()))
    }

    /// Render with a pre-built page LenientValue, cached site, and overrides.
    pub(crate) fn render_with_prebuilt_page_overrides(
        &self,
        template: &Template,
        context: &Object,
        cached_site: &CachedSiteContext,
        page_lenient: &LenientValue,
        site_overrides: &HashMap<String, LenientValue>,
    ) -> Result<String, TemplateError> {
        let lenient = LenientObject::with_prebuilt_page_overrides(
            context,
            page_lenient,
            &cached_site.site_lenient,
            site_overrides,
        );
        template
            .inner
            .render(&lenient)
            .map_err(|e| TemplateError::RenderError(e.to_string()))
    }

    /// Parse and render with cached site and per-render overrides.
    pub(crate) fn parse_and_render_with_site_overrides(
        &self,
        template_str: &str,
        context: &Object,
        cached_site: &CachedSiteContext,
        site_overrides: &HashMap<String, LenientValue>,
    ) -> Result<String, TemplateError> {
        let template = self.parse(template_str)?;
        self.render_with_site_overrides(&template, context, cached_site, site_overrides)
    }
}

/// Run all Liquid preprocessing passes on a template string.
///
/// This consolidates the 13 preprocessing passes into a single function with
/// fast-path short-circuits. When the content has no Liquid markers (`{{` or
/// `{%`), all preprocessing is skipped entirely since none of the passes
/// would modify the content.
fn preprocess_all(template: &str) -> String {
    let has_tag = template.contains("{%");
    let has_output = template.contains("{{");

    // Fast path: no Liquid markers at all -- skip all preprocessing.
    if !has_tag && !has_output {
        return template.to_string();
    }

    // Run tag-related preprocessors only when {%  is present.
    let preprocessed = if has_tag {
        let preprocessed = super::include_tag::preprocess_include_paths(template);
        let preprocessed = preprocess_capture_tags(&preprocessed);
        let preprocessed = preprocess_jekyll_tags(&preprocessed);
        let preprocessed = super::octicon_tag::preprocess_octicon_tags(&preprocessed);
        let preprocessed = preprocess_nil_contains(&preprocessed);
        let preprocessed = preprocess_nil_eq_false(&preprocessed);
        let preprocessed = preprocess_if_condition_filters(&preprocessed);
        let preprocessed = preprocess_nested_braces(&preprocessed);
        let preprocessed = preprocess_for_loop_filters(&preprocessed);
        let preprocessed = preprocess_parenthesized_assign(&preprocessed);
        preprocess_stray_brace_in_tags(&preprocessed)
    } else {
        template.to_string()
    };

    // Run output-related preprocessors only when {{ is present.
    if has_output {
        let preprocessed = preprocess_output_or(&preprocessed);
        preprocess_bare_output_render_mapping(&preprocessed)
    } else {
        preprocessed
    }
}

/// Pre-process capture tags to strip extra tokens after the variable name.
///
/// Jekyll's Liquid parser silently ignores extra tokens after the variable name
/// in a capture tag (e.g., `{% capture myvar do %}` is treated as
/// `{% capture myvar %}`). The liquid crate's parser is strict and rejects them.
///
/// This function finds `{% capture <var> <extra_tokens> %}` patterns and
/// rewrites them to `{% capture <var> %}`, preserving whitespace-control
/// markers (`-`).
fn preprocess_capture_tags(template: &str) -> String {
    let mut result = String::with_capacity(template.len());
    let mut remaining = template;

    while let Some(start) = remaining.find("{%") {
        // Copy everything up to this tag
        result.push_str(&remaining[..start]);

        let after_open = &remaining[start + 2..];
        if let Some(end_offset) = after_open.find("%}") {
            let tag_inner = &after_open[..end_offset];
            let tag_end = start + 2 + end_offset + 2;

            // Parse tag inner, handling whitespace-control dashes
            let trimmed = tag_inner.trim();
            let has_leading_dash = trimmed.starts_with('-');
            let trimmed = if has_leading_dash {
                trimmed[1..].trim_start()
            } else {
                trimmed
            };
            let has_trailing_dash = trimmed.ends_with('-');
            let trimmed = if has_trailing_dash {
                trimmed[..trimmed.len() - 1].trim_end()
            } else {
                trimmed
            };

            // Check if this is a capture tag with extra tokens
            if let Some(after_capture) = trimmed
                .strip_prefix("capture")
                .filter(|rest| rest.starts_with(char::is_whitespace))
            {
                let args = after_capture.trim();
                // The variable name is the first word; extra tokens follow
                let var_name = args.split_whitespace().next().unwrap_or(args);
                let has_extra = args.len() > var_name.len();

                if has_extra {
                    // Rewrite: keep only the variable name
                    result.push_str("{%");
                    if has_leading_dash {
                        result.push('-');
                    }
                    result.push_str(" capture ");
                    result.push_str(var_name);
                    result.push(' ');
                    if has_trailing_dash {
                        result.push('-');
                    }
                    result.push_str("%}");
                } else {
                    // No extra tokens, keep original
                    result.push_str(&remaining[start..tag_end]);
                }
            } else {
                // Not a capture tag, keep original
                result.push_str(&remaining[start..tag_end]);
            }

            remaining = &remaining[tag_end..];
        } else {
            // No closing %}, copy rest as-is
            result.push_str(&remaining[start..]);
            remaining = "";
        }
    }

    // Copy any remaining text
    result.push_str(remaining);
    result
}

/// Pre-process Jekyll-specific tags that the liquid crate does not support.
///
/// Handles:
/// - `{% link _pages/file.md %}` -> approximate URL (e.g., `/pages/file.html`)
/// - `{% post_url 2022-01-01-title %}` -> approximate URL (e.g., `/posts/title`)
///
/// These are best-effort transformations. The exact URL depends on the site's
/// permalink configuration, but the approximation is sufficient for most cases.
fn preprocess_jekyll_tags(template: &str) -> String {
    let config = crate::collection::LinkTagConfig::from_globals();
    preprocess_jekyll_tags_with_config(template, &config)
}

/// Inner implementation of `preprocess_jekyll_tags` that accepts an explicit
/// `LinkTagConfig` instead of reading from global state. This allows tests
/// to pass permalink styles directly, avoiding races on global mutexes.
fn preprocess_jekyll_tags_with_config(
    template: &str,
    link_config: &crate::collection::LinkTagConfig,
) -> String {
    let mut result = String::with_capacity(template.len());
    let mut remaining = template;

    while let Some(start) = remaining.find("{%") {
        result.push_str(&remaining[..start]);

        let after_open = &remaining[start + 2..];
        if let Some(end_offset) = after_open.find("%}") {
            let tag_inner = &after_open[..end_offset];
            let tag_end = start + 2 + end_offset + 2;

            let trimmed = tag_inner.trim();
            let trimmed = trimmed.strip_prefix('-').unwrap_or(trimmed).trim();
            let trimmed = trimmed.strip_suffix('-').unwrap_or(trimmed).trim();

            if let Some(path) = trimmed
                .strip_prefix("link")
                .filter(|rest| rest.starts_with(char::is_whitespace))
            {
                // {% link _pages/file.md %} -> /pages/file (collection) or /file.html (root)
                let path = path.trim().trim_matches('"').trim_matches('\'');

                // Special handling for _posts/: resolve using the permalink pattern
                if let Some(post_path) = path.strip_prefix("_posts/") {
                    let url_path = resolve_link_post_url(post_path);
                    result.push_str(&url_path);
                } else {
                    // Check if the path starts with _ (collection document)
                    let is_collection = path.starts_with('_');
                    // Strip leading underscore-prefixed directory (e.g., _pages/ -> pages/)
                    let url_path = if let Some(stripped) = path.strip_prefix('_') {
                        stripped
                    } else {
                        path
                    };

                    // Extract collection name for looking up collection-specific suffix.
                    // E.g., from "docs/variables" extract "docs".
                    let collection_suffix: Option<&str> = if is_collection {
                        let coll_name = url_path.split('/').next().unwrap_or("");
                        link_config.collection_link_suffix(coll_name)
                    } else {
                        None
                    };

                    // For collection docs: strip .md or .html extension, then apply
                    // collection-specific suffix (trailing slash) if configured.
                    // For root pages: use permalink-style suffix (.html or /)
                    let link_suffix = link_config.link_tag_suffix();
                    let url_path = if let Some(stem) = url_path.strip_suffix(".md") {
                        if is_collection {
                            // Check for index files: _docs/index.md -> /docs/
                            let basename = stem.rsplit('/').next().unwrap_or(stem);
                            if basename == "index" {
                                let dir = &stem[..stem.len() - "index".len()];
                                format!("/{}", dir)
                            } else {
                                let suffix = collection_suffix.unwrap_or("");
                                format!("/{}{}", stem, suffix)
                            }
                        } else {
                            format!("/{}{}", stem, link_suffix)
                        }
                    } else if let Some(stem) = url_path.strip_suffix(".html") {
                        if is_collection {
                            let basename = stem.rsplit('/').next().unwrap_or(stem);
                            if basename == "index" {
                                let dir = &stem[..stem.len() - "index".len()];
                                format!("/{}", dir)
                            } else {
                                let suffix = collection_suffix.unwrap_or("");
                                format!("/{}{}", stem, suffix)
                            }
                        } else if link_suffix == "/" {
                            format!("/{}/", stem)
                        } else {
                            format!("/{}", url_path)
                        }
                    } else {
                        format!("/{}", url_path)
                    };
                    result.push_str(&url_path);
                }
            } else if let Some(slug) = trimmed
                .strip_prefix("post_url")
                .filter(|rest| rest.starts_with(char::is_whitespace))
            {
                // {% post_url 2022-01-01-title %} -> /2022/01/01/title/
                let slug = slug.trim();
                // Parse YYYY-MM-DD-title format
                if slug.len() > 10
                    && slug.chars().nth(4) == Some('-')
                    && slug.chars().nth(7) == Some('-')
                {
                    let year = &slug[0..4];
                    let month = &slug[5..7];
                    let day = &slug[8..10];
                    let title = &slug[11..];
                    result.push_str(&format!("/{}/{}/{}/{}/", year, month, day, title));
                } else {
                    // Fallback: just use slug as path
                    result.push_str(&format!("/{}", slug));
                }
            } else if let Some(args) = trimmed
                .strip_prefix("gist")
                .filter(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
            {
                // {% gist 5555251 gist.md %} -> <noscript>...<script>...
                result.push_str(&super::gist_tag::render_gist(args));
            } else {
                // Not a link/post_url/gist tag, keep original
                result.push_str(&remaining[start..tag_end]);
            }

            remaining = &remaining[tag_end..];
        } else {
            result.push_str(&remaining[start..]);
            remaining = "";
        }
    }

    result.push_str(remaining);
    result
}

/// Resolve a `{% link _posts/YYYY-MM-DD-title.md %}` to its URL using the
/// global post permalink pattern. Extracts date and title from the filename
/// and substitutes them into the pattern.
fn resolve_link_post_url(post_filename: &str) -> String {
    // Strip .md or .html extension
    let stem = post_filename
        .strip_suffix(".md")
        .or_else(|| post_filename.strip_suffix(".html"))
        .unwrap_or(post_filename);

    // Parse YYYY-MM-DD-title format
    if stem.len() > 10
        && stem.as_bytes().get(4) == Some(&b'-')
        && stem.as_bytes().get(7) == Some(&b'-')
    {
        let year = &stem[0..4];
        let month = &stem[5..7];
        let day = &stem[8..10];
        let title = &stem[11..];

        let pattern = crate::frontmatter::get_post_permalink_pattern();
        // Apply the permalink pattern
        let url = pattern
            .replace(":year", year)
            .replace(":month", month)
            .replace(":day", day)
            .replace(":title", title)
            .replace(":categories", ""); // no category info available here

        // Clean up double slashes from empty :categories
        let mut cleaned = url.replace("//", "/");
        // Collapse remaining double slashes
        while cleaned.contains("//") {
            cleaned = cleaned.replace("//", "/");
        }
        // Issue 557: Jekyll does NOT append trailing slash to permalink patterns
        // without an extension. url_to_output_path handles adding .html.
        cleaned
    } else {
        // Fallback: strip extension and use as-is
        format!("/posts/{}", stem)
    }
}

/// Pre-process Liquid templates to add nil guards around `contains` operators.
///
/// Jekyll treats `nil contains "x"` as `false`, but the `liquid` crate raises
/// a runtime error ("Expected string | array | object, found `nil`"). This
/// function rewrites conditions like:
///
///   `{% if EXPR contains "STR" %}`
///
/// to:
///
///   `{% if EXPR and EXPR contains "STR" %}`
///
/// The `and` operator short-circuits: if EXPR is nil (falsy), the `contains`
/// is never evaluated, matching Jekyll's behavior. (Issue 171)
fn preprocess_nil_contains(template: &str) -> String {
    let mut result = String::with_capacity(template.len());
    let mut remaining = template;

    while let Some(start) = remaining.find("{%") {
        result.push_str(&remaining[..start]);

        let after_open = &remaining[start + 2..];
        if let Some(end_offset) = after_open.find("%}") {
            let tag_inner = &after_open[..end_offset];
            let tag_end = start + 2 + end_offset + 2;

            let trimmed = tag_inner.trim();
            let has_leading_dash = trimmed.starts_with('-');
            let trimmed_content = if has_leading_dash {
                trimmed[1..].trim_start()
            } else {
                trimmed
            };
            let has_trailing_dash = trimmed_content.ends_with('-');
            let trimmed_content = if has_trailing_dash {
                trimmed_content[..trimmed_content.len() - 1].trim_end()
            } else {
                trimmed_content
            };

            let is_if_tag =
                trimmed_content.starts_with("if ") || trimmed_content.starts_with("elsif ");
            if is_if_tag {
                if let Some(rewritten) = rewrite_contains_with_nil_guard(trimmed_content) {
                    result.push_str("{%");
                    if has_leading_dash {
                        result.push('-');
                    }
                    result.push(' ');
                    result.push_str(&rewritten);
                    result.push(' ');
                    if has_trailing_dash {
                        result.push('-');
                    }
                    result.push_str("%}");
                } else {
                    result.push_str(&remaining[start..tag_end]);
                }
            } else {
                result.push_str(&remaining[start..tag_end]);
            }

            remaining = &remaining[tag_end..];
        } else {
            result.push_str(&remaining[start..]);
            remaining = "";
        }
    }

    result.push_str(remaining);
    result
}

/// Pre-process `== false` comparisons to work around the Liquid crate
/// treating `nil == false` as true (Ruby Liquid returns false).
///
/// Rewrites `VAR == false` to `VAR == false and VAR != nil` so that undefined
/// variables (nil) don't match the literal `false`.
///
/// Note: The vendored liquid-core (issue #397) already fixes `nil == false` to
/// return false, so the `== false` guard is belt-and-suspenders. The previous
/// `!= false` -> `!= false or VAR == nil` rewrite was REMOVED (issue #504)
/// because it introduces an `or` operator that changes Liquid's and-before-or
/// precedence and causes incorrect evaluation of compound conditions like
/// `site.x != false and layout.y == nil and page.z == nil`.
fn preprocess_nil_eq_false(template: &str) -> String {
    use regex::Regex;
    use std::sync::LazyLock;

    // Match patterns like: VARIABLE == false
    // VARIABLE is a dotted name like page.toc, site.show_edit, etc.
    static EQ_FALSE_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\b([\w][\w.]*)\s*==\s*false\b").unwrap());

    let result = EQ_FALSE_RE.replace_all(template, "$1 == false and $1 != nil");
    result.into_owned()
}

/// Pre-process filter chains in `{% if/elsif/unless %}` conditions.
///
/// Jekyll (Ruby Liquid) allows filter chains in conditional expressions:
///   `{% if site.x and page.y | default: false %}`
/// The Rust liquid crate does not support filters in conditions. We rewrite
/// these by extracting the filter chain into a preceding `{% assign %}`:
///   `{% assign __if_filter_N = page.y | default: false %}{% if site.x and __if_filter_N %}`
///
/// Only conditions containing a `|` pipe are rewritten.
fn preprocess_if_condition_filters(template: &str) -> String {
    // Fast path: if no `if ` or `unless `, nothing to do.
    if !template.contains("if ") && !template.contains("unless ") {
        return template.to_string();
    }

    let mut result = String::with_capacity(template.len());
    let mut remaining = template;
    let mut counter = 0u32;

    while let Some(start) = remaining.find("{%") {
        result.push_str(&remaining[..start]);

        let after_open = &remaining[start + 2..];
        if let Some(end_offset) = after_open.find("%}") {
            let tag_inner = &after_open[..end_offset];
            let tag_end = start + 2 + end_offset + 2;

            let trimmed = tag_inner.trim();
            // Detect leading/trailing whitespace-control dashes
            let has_leading_dash = trimmed.starts_with('-');
            let content = if has_leading_dash {
                trimmed[1..].trim_start()
            } else {
                trimmed
            };
            let has_trailing_dash = content.ends_with('-');
            let content = if has_trailing_dash {
                content[..content.len() - 1].trim_end()
            } else {
                content
            };

            let is_conditional = content.starts_with("if ")
                || content.starts_with("elsif ")
                || content.starts_with("unless ");

            if is_conditional && content.contains('|') {
                // Check if the pipe is inside a string literal (skip those)
                if let Some(rewritten) = rewrite_condition_filter_chain(content, &mut counter) {
                    let ld = if has_leading_dash { "-" } else { "" };
                    let td = if has_trailing_dash { "-" } else { "" };
                    result.push_str(&rewritten.prefix_assigns);
                    result.push_str(&format!("{{% {}{}{} %}}", ld, rewritten.condition, td));
                    remaining = &remaining[tag_end..];
                    continue;
                }
            }

            // No rewrite needed -- emit original tag
            result.push_str(&remaining[start..tag_end]);
            remaining = &remaining[tag_end..];
        } else {
            // Unclosed tag -- emit rest as-is
            result.push_str(&remaining[start..]);
            remaining = "";
        }
    }
    result.push_str(remaining);
    result
}

struct ConditionRewrite {
    prefix_assigns: String,
    condition: String,
}

/// Try to rewrite a conditional expression that contains filter chains.
///
/// Splits on `and`/`or` keywords and checks each operand for `|` pipes.
/// Filter-chain operands are replaced with temporary variables, and
/// `{% assign %}` tags are prepended.
fn rewrite_condition_filter_chain(content: &str, counter: &mut u32) -> Option<ConditionRewrite> {
    // Find the keyword (if/elsif/unless) and the condition body
    let (keyword, body) = if let Some(rest) = content.strip_prefix("if ") {
        ("if", rest.trim())
    } else if let Some(rest) = content.strip_prefix("elsif ") {
        ("elsif", rest.trim())
    } else if let Some(rest) = content.strip_prefix("unless ") {
        ("unless", rest.trim())
    } else {
        return None;
    };

    // Check if there's actually a pipe in the body (outside of string literals)
    if !has_pipe_outside_strings(body) {
        return None;
    }

    // Split the body into tokens by `and`/`or` while preserving them.
    // We need to find filter chains (operands with `|`) and extract them.
    let mut prefix_assigns = String::new();
    let mut new_body = String::new();

    // Simple tokenization: split on ` and ` and ` or ` boundaries
    let mut remaining = body;
    let mut first = true;

    loop {
        let (operand, connector, rest) = split_next_logical_op(remaining);
        let operand_trimmed = operand.trim();

        if !first {
            new_body.push(' ');
        }
        first = false;

        if has_pipe_outside_strings(operand_trimmed) {
            // This operand has a filter chain -- extract it
            *counter += 1;
            let var_name = format!("__if_filter_{}", counter);
            prefix_assigns.push_str(&format!(
                "{{% assign {} = {} %}}",
                var_name, operand_trimmed
            ));
            new_body.push_str(&var_name);
        } else {
            new_body.push_str(operand_trimmed);
        }

        if let Some(conn) = connector {
            new_body.push_str(&format!(" {}", conn));
            remaining = rest;
        } else {
            break;
        }
    }

    Some(ConditionRewrite {
        prefix_assigns,
        condition: format!("{} {}", keyword, new_body),
    })
}

/// Check if a string contains `|` outside of quoted strings.
fn has_pipe_outside_strings(s: &str) -> bool {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    for ch in s.chars() {
        match ch {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote => in_double_quote = !in_double_quote,
            '|' if !in_single_quote && !in_double_quote => return true,
            _ => {}
        }
    }
    false
}

/// Split a condition string on the next ` and ` or ` or ` boundary.
///
/// Returns (operand, connector, rest). If no connector is found,
/// returns (full_string, None, "").
fn split_next_logical_op(s: &str) -> (&str, Option<&str>, &str) {
    // Find the first ` and ` or ` or ` that's not inside quotes
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut i = 0;

    while i < len {
        match bytes[i] {
            b'\'' if !in_double_quote => in_single_quote = !in_single_quote,
            b'"' if !in_single_quote => in_double_quote = !in_double_quote,
            b' ' if !in_single_quote && !in_double_quote => {
                // Check for ` and ` or ` or `
                if i + 5 <= len && &s[i..i + 5] == " and " {
                    return (&s[..i], Some("and"), &s[i + 5..]);
                }
                if i + 4 <= len && &s[i..i + 4] == " or " {
                    return (&s[..i], Some("or"), &s[i + 4..]);
                }
            }
            _ => {}
        }
        i += 1;
    }
    (s, None, "")
}

/// Pre-process `{% for var in expr | filter %}` to extract the filter chain.
///
/// Jekyll (Ruby Liquid) supports filter chains in `for` loop iterables, e.g.:
///   `{% for name in names | sort %}`
/// The `liquid` crate (Rust) does not. We rewrite these to:
///   `{% assign __for_name = names | sort %}{% for name in __for_name %}`
///
/// Only for-loops with a pipe `|` between `in` and `%}` are rewritten.
/// For-loops with `limit:`, `offset:`, or `reversed` but no pipe are unchanged.
fn preprocess_for_loop_filters(template: &str) -> String {
    // Fast path: if there's no "for " at all, nothing to do.
    if !template.contains("for ") {
        return template.to_string();
    }

    let mut result = String::with_capacity(template.len());
    let mut remaining = template;

    while let Some(start) = remaining.find("{%") {
        result.push_str(&remaining[..start]);

        let after_open = &remaining[start + 2..];
        if let Some(end_offset) = after_open.find("%}") {
            let tag_inner = &after_open[..end_offset];
            let tag_end = start + 2 + end_offset + 2;

            // Check if this is a for tag
            let trimmed = tag_inner.trim();
            let trimmed = trimmed.strip_prefix('-').unwrap_or(trimmed).trim();
            let trimmed_end = trimmed.strip_suffix('-').unwrap_or(trimmed).trim();

            if let Some(rest) = trimmed_end.strip_prefix("for ") {
                let rest = rest.trim();
                // Parse: VAR in EXPR | FILTER
                if let Some(in_pos) = rest.find(" in ") {
                    let var_name = rest[..in_pos].trim();
                    let after_in = rest[in_pos + 4..].trim();

                    // Check if there's a pipe that's NOT inside a string literal
                    // and not part of limit:/offset:/reversed
                    if let Some(pipe_pos) = find_top_level_pipe(after_in) {
                        let expr = after_in[..pipe_pos].trim();
                        let filter_chain = after_in[pipe_pos..].trim(); // includes the |

                        // Preserve whitespace-control markers
                        let orig_tag = &remaining[start..tag_end];
                        let open_marker = if orig_tag.starts_with("{%-") {
                            "{%-"
                        } else {
                            "{%"
                        };
                        let close_marker = if orig_tag.ends_with("-%}") {
                            "-%}"
                        } else {
                            "%}"
                        };

                        let temp_var = format!("__for_{}", var_name);
                        // Emit: {% assign __for_VAR = EXPR | FILTER %}{% for VAR in __for_VAR %}
                        result.push_str("{%");
                        result.push_str(&format!(
                            " assign {} = {} {} ",
                            temp_var, expr, filter_chain
                        ));
                        result.push_str("%}");
                        result.push_str(open_marker);
                        result.push_str(&format!(" for {} in {} ", var_name, temp_var));
                        result.push_str(close_marker);
                        remaining = &remaining[tag_end..];
                        continue;
                    }
                }
            }

            // Not a for-with-filter -- copy as-is
            result.push_str(&remaining[start..tag_end]);
            remaining = &remaining[tag_end..];
        } else {
            result.push_str(&remaining[start..]);
            remaining = "";
        }
    }

    result.push_str(remaining);
    result
}

/// Find the position of the first `|` that is NOT inside a string literal.
/// Returns `None` if there's no top-level pipe.
fn find_top_level_pipe(s: &str) -> Option<usize> {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    for (i, ch) in s.char_indices() {
        match ch {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote => in_double_quote = !in_double_quote,
            '|' if !in_single_quote && !in_double_quote => return Some(i),
            _ => {}
        }
    }
    None
}

/// Pre-process `{% assign var = (expr | filter) %}` to strip parentheses.
///
/// Jekyll (Ruby Liquid) allows parenthesized expressions in assign:
///   `{% assign x = (arr | split: ',' | sort) %}`
/// The `liquid` crate rejects the parentheses. We strip them:
///   `{% assign x = arr | split: ',' | sort %}`
fn preprocess_parenthesized_assign(template: &str) -> String {
    // Fast path
    if !template.contains("assign ") || !template.contains('(') {
        return template.to_string();
    }

    let mut result = String::with_capacity(template.len());
    let mut remaining = template;

    while let Some(start) = remaining.find("{%") {
        result.push_str(&remaining[..start]);

        let after_open = &remaining[start + 2..];
        if let Some(end_offset) = after_open.find("%}") {
            let tag_inner = &after_open[..end_offset];
            let tag_end = start + 2 + end_offset + 2;

            let trimmed = tag_inner.trim();
            let trimmed = trimmed.strip_prefix('-').unwrap_or(trimmed).trim();
            let trimmed_end = trimmed.strip_suffix('-').unwrap_or(trimmed).trim();

            if let Some(rest) = trimmed_end.strip_prefix("assign ") {
                // Check for pattern: VAR = (EXPR)
                if let Some(eq_pos) = rest.find('=') {
                    let after_eq = rest[eq_pos + 1..].trim();
                    if after_eq.starts_with('(') && after_eq.ends_with(')') {
                        let inner = &after_eq[1..after_eq.len() - 1];
                        let var_name = rest[..eq_pos].trim();

                        // Preserve whitespace-control markers
                        let orig_tag = &remaining[start..tag_end];
                        let open_marker = if orig_tag.starts_with("{%-") {
                            "{%-"
                        } else {
                            "{%"
                        };
                        let close_marker = if orig_tag.ends_with("-%}") {
                            "-%}"
                        } else {
                            "%}"
                        };

                        result.push_str(open_marker);
                        result.push_str(&format!(" assign {} = {} ", var_name, inner));
                        result.push_str(close_marker);
                        remaining = &remaining[tag_end..];
                        continue;
                    }
                }
            }

            result.push_str(&remaining[start..tag_end]);
            remaining = &remaining[tag_end..];
        } else {
            result.push_str(&remaining[start..]);
            remaining = "";
        }
    }

    result.push_str(remaining);
    result
}

/// Pre-process `{{var}}` inside `{% %}` tags to strip the inner braces.
///
/// Some Jekyll themes (e.g., documentation-theme-jekyll) use the non-standard
/// pattern `{% if {{include.url}} %}` where `{{}}` appears inside a `{% %}`
/// tag. Standard Liquid rejects this. We rewrite it to `{% if include.url %}`
/// so the parser can handle it.
///
/// Only `{{` and `}}` that appear between `{%` and `%}` are affected;
/// `{{}}` in the body/output sections are left untouched.
fn preprocess_nested_braces(template: &str) -> String {
    // Fast path: if there's no `{{` at all, nothing to do.
    if !template.contains("{{") {
        return template.to_string();
    }

    let mut result = String::with_capacity(template.len());
    let mut remaining = template;

    while !remaining.is_empty() {
        // Find the next {% tag
        if let Some(tag_start) = remaining.find("{%") {
            // Find the matching %}
            let after_open = &remaining[tag_start + 2..];
            if let Some(tag_end_rel) = after_open.find("%}") {
                // We have a {% ... %} tag
                let tag_content = &after_open[..tag_end_rel];

                // Copy everything before the tag
                result.push_str(&remaining[..tag_start]);

                // Strip {{ and }} from inside the tag content
                let cleaned = tag_content.replace("{{", "").replace("}}", "");
                result.push_str("{%");
                result.push_str(&cleaned);
                result.push_str("%}");

                remaining = &after_open[tag_end_rel + 2..];
            } else {
                // No matching %}, copy everything
                result.push_str(remaining);
                break;
            }
        } else {
            result.push_str(remaining);
            break;
        }
    }

    result
}

/// Rewrite a condition to add nil guards around `contains`.
fn rewrite_contains_with_nil_guard(condition: &str) -> Option<String> {
    if !condition.contains(" contains ") {
        return None;
    }

    let (keyword, expr) = if let Some(rest) = condition.strip_prefix("elsif ") {
        ("elsif", rest.trim())
    } else if let Some(rest) = condition.strip_prefix("if ") {
        ("if", rest.trim())
    } else {
        return None;
    };

    let rewritten = rewrite_contains_in_expr(expr)?;
    Some(format!("{} {}", keyword, rewritten))
}

/// Rewrite `contains` in an expression to add nil guards.
fn rewrite_contains_in_expr(expr: &str) -> Option<String> {
    let contains_keyword = " contains ";
    if !expr.contains(contains_keyword) {
        return None;
    }

    let mut result = String::with_capacity(expr.len() * 2);
    let mut remaining = expr;
    let mut changed = false;

    while let Some(pos) = remaining.find(contains_keyword) {
        let before = &remaining[..pos];
        let lhs = extract_last_operand(before);

        if !lhs.is_empty() {
            result.push_str(before);
            result.push_str(" and ");
            result.push_str(lhs);
            result.push_str(contains_keyword);
            changed = true;
        } else {
            result.push_str(before);
            result.push_str(contains_keyword);
        }

        remaining = &remaining[pos + contains_keyword.len()..];
    }

    result.push_str(remaining);

    if changed {
        Some(result)
    } else {
        None
    }
}

/// Extract the last operand from an expression fragment.
fn extract_last_operand(expr: &str) -> &str {
    let trimmed = expr.trim_end();
    let last_and = trimmed.rfind(" and ");
    let last_or = trimmed.rfind(" or ");

    let boundary = match (last_and, last_or) {
        (Some(a), Some(o)) => Some(a.max(o)),
        (Some(a), None) => Some(a),
        (None, Some(o)) => Some(o),
        (None, None) => None,
    };

    match boundary {
        Some(pos) => {
            let after = &trimmed[pos..];
            let skip = if after.starts_with(" and ") { 5 } else { 4 };
            trimmed[pos + skip..].trim()
        }
        None => trimmed.trim(),
    }
}

/// Extract the unknown filter name from a liquid parse error message.
///
/// The liquid crate formats "Unknown filter" errors with the filter name
/// in a "requested filter" context line. This function parses that out.
fn extract_unknown_filter_name(err_str: &str) -> Option<String> {
    // The error message format is:
    //   liquid: Unknown filter
    //     from: ...
    //     requested filter: <name>
    //     available filters: ...
    if !err_str.contains("Unknown filter") {
        return None;
    }
    for line in err_str.lines() {
        let trimmed = line.trim();
        // The liquid crate formats context as "requested filter=name"
        if let Some(name) = trimmed.strip_prefix("requested filter=") {
            let name = name.trim();
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }
    None
}

/// Load all include files from a directory into a map of name -> content.
///
/// Files in subdirectories are registered with path separators, e.g.,
/// `"course-structured-data/file.html"`.
///
/// # Errors
///
/// Returns `TemplateError::IoError` if the directory or any file cannot be read.
pub fn load_includes(includes_dir: &Path) -> Result<HashMap<String, String>, TemplateError> {
    let mut map = HashMap::new();
    load_includes_recursive(includes_dir, includes_dir, &mut map)?;
    Ok(map)
}

fn load_includes_recursive(
    base_dir: &Path,
    current_dir: &Path,
    map: &mut HashMap<String, String>,
) -> Result<(), TemplateError> {
    if !current_dir.exists() {
        return Ok(());
    }

    let entries = fs::read_dir(current_dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            load_includes_recursive(base_dir, &path, map)?;
        } else if path.is_file() {
            // Register all files (not just .html) -- Jekyll includes can have any extension
            let relative = path
                .strip_prefix(base_dir)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            // Normalize path separators to forward slashes
            let key = relative.replace('\\', "/");
            let content = fs::read_to_string(&path)?;
            // Issue 250: Do NOT normalize void elements in include files.
            // Include files contain raw HTML that should pass through as-is.
            // Jekyll does not normalize void elements in includes.
            // The final normalize_html_output() after layout rendering will
            // handle markdown-rendered <br> and <hr> elements correctly.
            map.insert(key, content);
        }
    }
    Ok(())
}

/// Load includes from a default directory and a custom directory, merging them.
///
/// Files from the custom directory override files from the default directory
/// with the same relative path. This matches Jekyll's behavior where a
/// site-level include overrides a theme-level include.
///
/// When both paths are the same directory, the includes are loaded once (no duplication).
pub fn load_includes_merged(
    default_dir: &Path,
    custom_dir: &Path,
) -> Result<HashMap<String, String>, TemplateError> {
    let mut map = HashMap::new();
    // Load default includes first
    load_includes_recursive(default_dir, default_dir, &mut map)?;
    // If custom dir is different, overlay its entries (overriding defaults)
    if default_dir != custom_dir {
        load_includes_recursive(custom_dir, custom_dir, &mut map)?;
    }
    Ok(map)
}

/// Strip stray `}` inside `{% %}` tags.
///
/// Some themes have typos like `{% assign x = y | filter } %}` where
/// a stray `}` appears before the closing `%}`. Jekyll's parser ignores
/// the extra brace, but the Rust `liquid` crate rejects it. This function
/// removes `}` that appears at the end of a tag body (before optional
/// whitespace-control `-` and the closing `%}`).
fn preprocess_stray_brace_in_tags(template: &str) -> String {
    if !template.contains("%}") {
        return template.to_string();
    }

    let mut result = String::with_capacity(template.len());
    let mut remaining = template;

    while let Some(start) = remaining.find("{%") {
        result.push_str(&remaining[..start]);

        let after_open = &remaining[start + 2..];
        if let Some(end_offset) = after_open.find("%}") {
            let tag_inner = &after_open[..end_offset];
            let tag_end = start + 2 + end_offset + 2;

            // Check if tag inner has a stray } at the end (before optional dash).
            // We look for the content part (strip leading/trailing dash + whitespace)
            // and check if it ends with ` }`.
            let content = tag_inner.trim();
            let content = content.strip_prefix('-').unwrap_or(content).trim();
            let content = content.strip_suffix('-').unwrap_or(content).trim();
            if (content.ends_with(" }") || content.ends_with("\t}")) && !content.trim().is_empty() {
                // Find the position of the stray `}` in the original tag_inner.
                // It's the last `}` before the trailing dash (if any).
                let inner_trimmed = tag_inner.trim_end();
                let inner_no_dash = inner_trimmed
                    .strip_suffix('-')
                    .unwrap_or(inner_trimmed)
                    .trim_end();
                if let Some(brace_rel) = inner_no_dash.rfind('}') {
                    // Rebuild: tag_inner with the stray } removed
                    result.push_str("{%");
                    result.push_str(&tag_inner[..brace_rel]);
                    result.push_str(&tag_inner[brace_rel + 1..]);
                    result.push_str("%}");
                    remaining = &remaining[tag_end..];
                    continue;
                }
            }

            // No stray brace, copy as-is
            result.push_str(&remaining[start..tag_end]);
            remaining = &remaining[tag_end..];
        } else {
            result.push_str(&remaining[start..]);
            remaining = "";
        }
    }

    result.push_str(remaining);
    result
}

/// Pre-process `{{ a or b }}` output tags to `{{ a | default: b }}`.
///
/// Jekyll/Ruby Liquid supports the `or` operator inside output tags as a
/// fallback mechanism: if `a` is nil/false, `b` is rendered instead. The
/// Rust `liquid` crate does not support `or` in output expressions, so we
/// rewrite it to use the `default` filter which achieves the same result.
///
/// Only rewrites `or` inside `{{ }}` output tags, not inside `{% %}` control
/// tags (where `or` is a valid boolean operator handled by the parser).
fn preprocess_output_or(template: &str) -> String {
    // Fast path: skip if no output tags at all
    if !template.contains("{{") {
        return template.to_string();
    }

    let mut result = String::with_capacity(template.len());
    let mut remaining = template;

    while !remaining.is_empty() {
        if let Some(start) = remaining.find("{{") {
            result.push_str(&remaining[..start]);

            let after_open = &remaining[start + 2..];
            if let Some(end_rel) = after_open.find("}}") {
                let inner = &after_open[..end_rel];
                let tag_end = start + 2 + end_rel + 2;

                // Check for ` or ` surrounded by whitespace (word boundary).
                // Only rewrite if there's no existing `|` filter (to avoid
                // interfering with filter chains that happen to contain "or").
                if let Some(or_pos) = find_word_or(inner) {
                    let lhs = inner[..or_pos].trim();
                    let rhs = inner[or_pos + 3..].trim(); // skip " or"
                    result.push_str("{{ ");
                    result.push_str(lhs);
                    result.push_str(" | default: ");
                    result.push_str(rhs);
                    result.push_str(" }}");
                } else {
                    result.push_str(&remaining[start..tag_end]);
                }

                remaining = &remaining[tag_end..];
            } else {
                result.push_str(&remaining[start..]);
                remaining = "";
            }
        } else {
            result.push_str(remaining);
            remaining = "";
        }
    }

    result
}

/// Find ` or ` as a word boundary inside an output tag expression.
/// Returns the byte offset of the space before `or`, or None.
/// Only matches when `or` is surrounded by whitespace (not part of a word).
fn find_word_or(inner: &str) -> Option<usize> {
    let bytes = inner.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i + 3 < len {
        if bytes[i] == b' ' && bytes[i + 1] == b'o' && bytes[i + 2] == b'r' && bytes[i + 3] == b' '
        {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Pre-process bare `{{ expr }}` output tags to apply `render_mapping`.
///
/// In Jekyll/Ruby Liquid, rendering a Hash (YAML mapping) via `{{ hash }}`
/// produces a Ruby-style hash string `{"key"=>"value"}`. The Rust `liquid`
/// crate instead concatenates all values. To match Jekyll, we append
/// `| render_mapping` to bare output tags that have no filters. The
/// `render_mapping` filter is a no-op for non-object values.
///
/// Only output tags (`{{ ... }}`) without any existing `|` filter pipe are
/// rewritten. Tags inside `{% %}` blocks are not affected.
fn preprocess_bare_output_render_mapping(template: &str) -> String {
    // Fast path: if there's no `{{` at all, nothing to do.
    if !template.contains("{{") {
        return template.to_string();
    }

    // Split the template into protected (code/raw) and unprotected segments.
    // Only unprotected segments get `| render_mapping` injected.
    let segments = split_protected_segments(template);
    let mut result = String::with_capacity(template.len() + 64);

    for (text, protected) in &segments {
        if *protected {
            result.push_str(text);
        } else {
            result.push_str(&inject_render_mapping(text));
        }
    }

    result
}

/// Split a template into segments, marking fenced code blocks (`\`\`\`` / `~~~`),
/// `{% raw %}...{% endraw %}` blocks, and `{% highlight %}...{% endhighlight %}`
/// blocks as protected. Returns `(text, is_protected)` pairs.
fn split_protected_segments(template: &str) -> Vec<(&str, bool)> {
    let mut segments: Vec<(&str, bool)> = Vec::new();
    let bytes = template.as_bytes();
    let len = bytes.len();
    let mut pos = 0;
    let mut unprotected_start = 0;

    while pos < len {
        // Check for {% raw %} blocks
        if bytes[pos] == b'{' && template[pos..].starts_with("{%") {
            if let Some(tag_end) = template[pos..].find("%}") {
                let tag_inner = template[pos + 2..pos + tag_end].trim();
                if tag_inner == "raw" {
                    // Found {% raw %}, look for {% endraw %}
                    let block_start = pos;
                    let after_tag = pos + tag_end + 2;
                    if let Some(end_offset) = template[after_tag..].find("{% endraw %}") {
                        let block_end = after_tag + end_offset + "{% endraw %}".len();
                        if unprotected_start < block_start {
                            segments.push((&template[unprotected_start..block_start], false));
                        }
                        segments.push((&template[block_start..block_end], true));
                        pos = block_end;
                        unprotected_start = pos;
                        continue;
                    }
                } else if tag_inner.starts_with("highlight") {
                    // Found {% highlight ... %}, look for {% endhighlight %}
                    let block_start = pos;
                    let after_tag = pos + tag_end + 2;
                    if let Some(end_offset) = template[after_tag..].find("{% endhighlight %}") {
                        let block_end = after_tag + end_offset + "{% endhighlight %}".len();
                        if unprotected_start < block_start {
                            segments.push((&template[unprotected_start..block_start], false));
                        }
                        segments.push((&template[block_start..block_end], true));
                        pos = block_end;
                        unprotected_start = pos;
                        continue;
                    }
                }
            }
        }

        // Check for fenced code blocks (``` or ~~~) at start of line
        if (pos == 0 || bytes[pos - 1] == b'\n')
            && pos + 3 <= len
            && (template[pos..].starts_with("```") || template[pos..].starts_with("~~~"))
        {
            let fence_char = bytes[pos];
            // Count fence characters
            let mut fence_len = 0;
            while pos + fence_len < len && bytes[pos + fence_len] == fence_char {
                fence_len += 1;
            }
            if fence_len >= 3 {
                let block_start = pos;
                // Skip to end of opening fence line
                let line_end = template[pos + fence_len..]
                    .find('\n')
                    .map(|i| pos + fence_len + i + 1)
                    .unwrap_or(len);
                // Look for matching closing fence
                let mut search_pos = line_end;
                let mut found_end = None;
                while search_pos < len {
                    if bytes[search_pos] == fence_char {
                        let mut close_len = 0;
                        while search_pos + close_len < len
                            && bytes[search_pos + close_len] == fence_char
                        {
                            close_len += 1;
                        }
                        if close_len >= fence_len {
                            // Found closing fence; include to end of line
                            let close_line_end = template[search_pos + close_len..]
                                .find('\n')
                                .map(|i| search_pos + close_len + i + 1)
                                .unwrap_or(search_pos + close_len);
                            found_end = Some(close_line_end);
                            break;
                        }
                    }
                    // Advance to next line
                    if let Some(nl) = template[search_pos..].find('\n') {
                        search_pos += nl + 1;
                    } else {
                        break;
                    }
                }
                if let Some(block_end) = found_end {
                    if unprotected_start < block_start {
                        segments.push((&template[unprotected_start..block_start], false));
                    }
                    segments.push((&template[block_start..block_end], true));
                    pos = block_end;
                    unprotected_start = pos;
                    continue;
                }
            }
        }

        pos += 1;
    }

    if unprotected_start < len {
        segments.push((&template[unprotected_start..], false));
    }

    segments
}

/// Apply `| render_mapping` to bare `{{ expr }}` output tags in a text segment.
fn inject_render_mapping(text: &str) -> String {
    let mut result = String::with_capacity(text.len() + 32);
    let mut remaining = text;

    while !remaining.is_empty() {
        if let Some(start) = remaining.find("{{") {
            // Copy everything before the tag
            result.push_str(&remaining[..start]);

            let after_open = &remaining[start + 2..];
            if let Some(end_rel) = after_open.find("}}") {
                let inner = &after_open[..end_rel];

                // Strip whitespace-control dashes and whitespace to get the
                // bare expression for checking whether a filter pipe exists.
                let expr = inner
                    .trim()
                    .trim_start_matches('-')
                    .trim_end_matches('-')
                    .trim();

                // Only add render_mapping if there's no existing filter pipe
                // and the expression is non-empty
                if !expr.is_empty() && !expr.contains('|') {
                    // Insert `| render_mapping` before the closing braces,
                    // preserving any whitespace-control dashes.
                    let trimmed_end = inner.trim_end();
                    if let Some(without_dash) = trimmed_end.strip_suffix('-') {
                        result.push_str("{{");
                        result.push_str(without_dash);
                        result.push_str(" | render_mapping -}}");
                    } else {
                        result.push_str("{{");
                        result.push_str(inner);
                        result.push_str(" | render_mapping }}");
                    }
                } else {
                    // Has a filter or is empty -- keep as-is
                    result.push_str("{{");
                    result.push_str(inner);
                    result.push_str("}}");
                }

                remaining = &after_open[end_rel + 2..];
            } else {
                // No matching `}}`, copy the rest as-is
                result.push_str(&remaining[start..]);
                break;
            }
        } else {
            // No more `{{`, copy the rest
            result.push_str(remaining);
            break;
        }
    }

    result
}

/// Build an `EagerCompiler<InMemorySource>` from a map of partial names to content.
///
/// Each partial's content is pre-processed with the same pipeline as templates:
/// include path quoting, for-loop filter extraction, parenthesized assign stripping,
/// etc. This ensures include files with Jekyll-specific Liquid syntax are handled
/// correctly (Issue 328).
fn build_partials(includes: &HashMap<String, String>) -> EagerCompiler<InMemorySource> {
    let mut partials = EagerCompiler::<InMemorySource>::empty();
    for (name, content) in includes {
        let preprocessed = preprocess_all(content);
        partials.add(name.clone(), preprocessed);
    }
    partials
}

#[cfg(test)]
mod tests {
    use super::*;
    use liquid::model::Value as LiquidValue;

    fn engine() -> TemplateEngine {
        TemplateEngine::new().unwrap()
    }

    // ========================================================================
    // Template parsing tests
    // ========================================================================

    #[test]
    fn test_parse_valid_template() {
        let eng = engine();
        assert!(eng.parse("Hello {{ variable }}!").is_ok());
    }

    #[test]
    fn test_parse_invalid_template() {
        let eng = engine();
        let result = eng.parse("{% if %}");
        assert!(result.is_err());
        if let Err(TemplateError::ParseError(_)) = result {
            // correct error variant
        } else {
            panic!("Expected ParseError");
        }
    }

    #[test]
    fn test_parse_and_render_simple() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("name".into(), LiquidValue::scalar("World"));
        let output = eng.parse_and_render("Hello {{ name }}!", &ctx).unwrap();
        assert_eq!(output, "Hello World!");
    }

    // ========================================================================
    // Variable output with dot notation
    // ========================================================================

    #[test]
    fn test_simple_variable() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("name".into(), LiquidValue::scalar("Alice"));
        let output = eng.parse_and_render("{{ name }}", &ctx).unwrap();
        assert_eq!(output, "Alice");
    }

    #[test]
    fn test_dot_notation_2_levels() {
        let eng = engine();
        let mut page = Object::new();
        page.insert("title".into(), LiquidValue::scalar("Hello"));
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page));
        let output = eng.parse_and_render("{{ page.title }}", &ctx).unwrap();
        assert_eq!(output, "Hello");
    }

    #[test]
    fn test_dot_notation_3_levels() {
        let eng = engine();
        let mut links = Object::new();
        links.insert(
            "youtube".into(),
            LiquidValue::scalar("https://youtube.com/123"),
        );
        let mut page = Object::new();
        page.insert("links".into(), LiquidValue::Object(links));
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page));
        let output = eng
            .parse_and_render("{{ page.links.youtube }}", &ctx)
            .unwrap();
        assert_eq!(output, "https://youtube.com/123");
    }

    #[test]
    fn test_undefined_variable_renders_empty() {
        let eng = engine();
        let ctx = Object::new();
        // A template with only an undefined variable should render as empty.
        let output = eng.parse_and_render("{{ missing_var }}", &ctx).unwrap();
        assert_eq!(output, "");

        // Mixed case: defined variables render normally, undefined ones become empty.
        let mut ctx2 = Object::new();
        ctx2.insert("name".into(), LiquidValue::scalar("World"));
        let output2 = eng
            .parse_and_render("Hello {{ name }}, {{ missing }}!", &ctx2)
            .unwrap();
        assert_eq!(output2, "Hello World, !");
    }

    // ========================================================================
    // For loops
    // ========================================================================

    #[test]
    fn test_for_loop_basic() {
        let eng = engine();
        let items = LiquidValue::Array(vec![
            LiquidValue::scalar("a"),
            LiquidValue::scalar("b"),
            LiquidValue::scalar("c"),
        ]);
        let mut ctx = Object::new();
        ctx.insert("items".into(), items);
        let output = eng
            .parse_and_render("{% for item in items %}{{ item }} {% endfor %}", &ctx)
            .unwrap();
        assert_eq!(output, "a b c ");
    }

    #[test]
    fn test_for_loop_index() {
        let eng = engine();
        let items = LiquidValue::Array(vec![
            LiquidValue::scalar("a"),
            LiquidValue::scalar("b"),
            LiquidValue::scalar("c"),
        ]);
        let mut ctx = Object::new();
        ctx.insert("items".into(), items);
        let output = eng
            .parse_and_render(
                "{% for item in items %}{{ forloop.index }}{% endfor %}",
                &ctx,
            )
            .unwrap();
        assert_eq!(output, "123");
    }

    #[test]
    fn test_for_loop_first_last() {
        let eng = engine();
        let items = LiquidValue::Array(vec![
            LiquidValue::scalar("a"),
            LiquidValue::scalar("b"),
            LiquidValue::scalar("c"),
        ]);
        let mut ctx = Object::new();
        ctx.insert("items".into(), items);
        let output = eng
            .parse_and_render(
                "{% for item in items %}{% if forloop.first %}F{% endif %}{% if forloop.last %}L{% endif %}{{ item }}{% endfor %}",
                &ctx,
            )
            .unwrap();
        assert_eq!(output, "FabLc");
    }

    #[test]
    fn test_for_loop_limit() {
        let eng = engine();
        let items = LiquidValue::Array(vec![
            LiquidValue::scalar("a"),
            LiquidValue::scalar("b"),
            LiquidValue::scalar("c"),
        ]);
        let mut ctx = Object::new();
        ctx.insert("items".into(), items);
        let output = eng
            .parse_and_render(
                "{% for item in items limit:2 %}{{ item }}{% endfor %}",
                &ctx,
            )
            .unwrap();
        assert_eq!(output, "ab");
    }

    #[test]
    fn test_nested_for_loops() {
        let eng = engine();
        let outer = LiquidValue::Array(vec![LiquidValue::scalar("X"), LiquidValue::scalar("Y")]);
        let inner = LiquidValue::Array(vec![LiquidValue::scalar("1"), LiquidValue::scalar("2")]);
        let mut ctx = Object::new();
        ctx.insert("outer".into(), outer);
        ctx.insert("inner".into(), inner);
        let output = eng
            .parse_and_render(
                "{% for o in outer %}{% for i in inner %}{{ o }}{{ i }}{% endfor %}{% endfor %}",
                &ctx,
            )
            .unwrap();
        assert_eq!(output, "X1X2Y1Y2");
    }

    #[test]
    fn test_for_loop_empty_array() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("items".into(), LiquidValue::Array(vec![]));
        let output = eng
            .parse_and_render("{% for item in items %}{{ item }}{% endfor %}", &ctx)
            .unwrap();
        assert_eq!(output, "");
    }

    // ========================================================================
    // Conditionals
    // ========================================================================

    #[test]
    fn test_if_present_and_absent() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("x".into(), LiquidValue::scalar("val"));
        let output = eng
            .parse_and_render("{% if x %}yes{% endif %}", &ctx)
            .unwrap();
        assert_eq!(output, "yes");

        let ctx2 = Object::new();
        let output2 = eng
            .parse_and_render("{% if x %}yes{% endif %}", &ctx2)
            .unwrap();
        assert_eq!(output2, "");
    }

    #[test]
    fn test_if_elsif_else() {
        let eng = engine();
        let mut ctx1 = Object::new();
        ctx1.insert("x".into(), LiquidValue::scalar(1i64));
        let out1 = eng
            .parse_and_render(
                "{% if x == 1 %}one{% elsif x == 2 %}two{% else %}other{% endif %}",
                &ctx1,
            )
            .unwrap();
        assert_eq!(out1, "one");

        let mut ctx2 = Object::new();
        ctx2.insert("x".into(), LiquidValue::scalar(2i64));
        let out2 = eng
            .parse_and_render(
                "{% if x == 1 %}one{% elsif x == 2 %}two{% else %}other{% endif %}",
                &ctx2,
            )
            .unwrap();
        assert_eq!(out2, "two");

        let mut ctx3 = Object::new();
        ctx3.insert("x".into(), LiquidValue::scalar(3i64));
        let out3 = eng
            .parse_and_render(
                "{% if x == 1 %}one{% elsif x == 2 %}two{% else %}other{% endif %}",
                &ctx3,
            )
            .unwrap();
        assert_eq!(out3, "other");
    }

    #[test]
    fn test_unless() {
        let eng = engine();
        let mut ctx_truthy = Object::new();
        ctx_truthy.insert("x".into(), LiquidValue::scalar(true));
        let out = eng
            .parse_and_render("{% unless x %}no{% endunless %}", &ctx_truthy)
            .unwrap();
        assert_eq!(out, "");

        let mut ctx_falsy = Object::new();
        ctx_falsy.insert("x".into(), LiquidValue::scalar(false));
        let out2 = eng
            .parse_and_render("{% unless x %}no{% endunless %}", &ctx_falsy)
            .unwrap();
        assert_eq!(out2, "no");
    }

    #[test]
    fn test_contains_operator() {
        let eng = engine();
        let items = LiquidValue::Array(vec![LiquidValue::scalar("a"), LiquidValue::scalar("b")]);
        let mut ctx = Object::new();
        ctx.insert("items".into(), items);
        let out = eng
            .parse_and_render("{% if items contains \"b\" %}yes{% endif %}", &ctx)
            .unwrap();
        assert_eq!(out, "yes");

        let items2 = LiquidValue::Array(vec![LiquidValue::scalar("a"), LiquidValue::scalar("c")]);
        let mut ctx2 = Object::new();
        ctx2.insert("items".into(), items2);
        let out2 = eng
            .parse_and_render("{% if items contains \"b\" %}yes{% endif %}", &ctx2)
            .unwrap();
        assert_eq!(out2, "");
    }

    #[test]
    fn test_and_operator() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("a".into(), LiquidValue::scalar(true));
        ctx.insert("b".into(), LiquidValue::scalar(true));
        let out = eng
            .parse_and_render("{% if a and b %}both{% endif %}", &ctx)
            .unwrap();
        assert_eq!(out, "both");

        let mut ctx2 = Object::new();
        ctx2.insert("a".into(), LiquidValue::scalar(true));
        ctx2.insert("b".into(), LiquidValue::scalar(false));
        let out2 = eng
            .parse_and_render("{% if a and b %}both{% endif %}", &ctx2)
            .unwrap();
        assert_eq!(out2, "");
    }

    #[test]
    fn test_or_operator() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("a".into(), LiquidValue::scalar(false));
        ctx.insert("b".into(), LiquidValue::scalar(true));
        let out = eng
            .parse_and_render("{% if a or b %}either{% endif %}", &ctx)
            .unwrap();
        assert_eq!(out, "either");

        let mut ctx2 = Object::new();
        ctx2.insert("a".into(), LiquidValue::scalar(false));
        ctx2.insert("b".into(), LiquidValue::scalar(false));
        let out2 = eng
            .parse_and_render("{% if a or b %}either{% endif %}", &ctx2)
            .unwrap();
        assert_eq!(out2, "");
    }

    #[test]
    fn test_not_equal_operator() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("x".into(), LiquidValue::scalar("good"));
        let out = eng
            .parse_and_render("{% if x != \"bad\" %}good{% endif %}", &ctx)
            .unwrap();
        assert_eq!(out, "good");
    }

    #[test]
    fn test_less_equal_operator() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("x".into(), LiquidValue::scalar(3i64));
        let out = eng
            .parse_and_render("{% if x <= 5 %}lte{% endif %}", &ctx)
            .unwrap();
        assert_eq!(out, "lte");
    }

    #[test]
    fn test_realistic_condition_from_podcast() {
        let eng = engine();
        let mut links = Object::new();
        links.insert(
            "youtube".into(),
            LiquidValue::scalar("https://youtube.com/watch?v=abc"),
        );
        let mut page = Object::new();
        page.insert("links".into(), LiquidValue::Object(links));
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page));
        let out = eng
            .parse_and_render(
                "{% if page.links.youtube and page.links.youtube != 'TODO' %}show{% endif %}",
                &ctx,
            )
            .unwrap();
        assert_eq!(out, "show");
    }

    // ========================================================================
    // Assign and capture
    // ========================================================================

    #[test]
    fn test_assign() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render("{% assign x = \"hello\" %}{{ x }}", &ctx)
            .unwrap();
        assert_eq!(out, "hello");
    }

    #[test]
    fn test_assign_with_filter() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("base".into(), LiquidValue::scalar("https://example.com"));
        let out = eng
            .parse_and_render("{% assign url = base | append: \"/path\" %}{{ url }}", &ctx)
            .unwrap();
        assert_eq!(out, "https://example.com/path");
    }

    #[test]
    fn test_capture() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("name".into(), LiquidValue::scalar("World"));
        let out = eng
            .parse_and_render(
                "{% capture msg %}hello {{ name }}{% endcapture %}{{ msg }}",
                &ctx,
            )
            .unwrap();
        assert_eq!(out, "hello World");
    }

    // ========================================================================
    // Break
    // ========================================================================

    #[test]
    fn test_break_in_for_loop() {
        let eng = engine();
        let items = LiquidValue::Array(vec![
            LiquidValue::scalar("a"),
            LiquidValue::scalar("stop"),
            LiquidValue::scalar("c"),
        ]);
        let mut ctx = Object::new();
        ctx.insert("items".into(), items);
        let out = eng
            .parse_and_render(
                "{% for item in items %}{% if item == \"stop\" %}{% break %}{% endif %}{{ item }}{% endfor %}",
                &ctx,
            )
            .unwrap();
        assert_eq!(out, "a");
    }

    // ========================================================================
    // Whitespace control
    // ========================================================================

    #[test]
    fn test_whitespace_control() {
        let eng = engine();
        let ctx = Object::new();
        // {%- strips whitespace before the tag, -%} strips whitespace after
        let out = eng
            .parse_and_render("  {%- assign x = \"hi\" -%}  {{ x }}", &ctx)
            .unwrap();
        assert_eq!(out, "hi");
    }

    /// Issue 517: capture with whitespace-stripping dashes should strip
    /// the captured content's leading/trailing whitespace.
    #[test]
    fn test_517_capture_whitespace_stripping() {
        let eng = engine();
        let ctx = Object::new();
        // {%- capture -%} should strip whitespace from captured content
        let out = eng
            .parse_and_render(
                "{%- capture val -%}\n  hello\n{%- endcapture -%}[{{ val }}]",
                &ctx,
            )
            .unwrap();
        assert_eq!(
            out, "[hello]",
            "Capture with dashes should strip inner whitespace"
        );
    }

    /// Issue 517/565/569: capture with include that uses {{- -}} dash output.
    /// With runtime whitespace stripping (issue 569), {{-}} strips the preceding
    /// whitespace from the output buffer, so the captured value is clean.
    #[test]
    fn test_517_capture_include_whitespace_stripping() {
        let mut includes = std::collections::HashMap::new();
        // Simulate media-url.html that uses assigns and outputs with dashes
        includes.insert(
            "media-url.html".to_string(),
            "\n{% assign url = include.src %}\n\n{{- url -}}\n".to_string(),
        );
        let eng = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let template = "{%- capture img_url -%}\n  {% include \"media-url.html\" src=\"/path/to/image.png\" %}\n{%- endcapture -%}[{{ img_url }}]";
        let out = eng.parse_and_render(template, &ctx).unwrap();
        // Issue 569: runtime whitespace stripping means {{- url -}} strips
        // all preceding whitespace from the output buffer
        assert_eq!(
            out, "[/path/to/image.png]",
            "Issue 517/565/569: {{- -}} in include strips preceding whitespace at runtime"
        );
    }

    /// Issue 517/569: assign then {{- strips ALL preceding whitespace at runtime.
    /// With issue 569's runtime whitespace stripping, {{- url -}} strips the
    /// whitespace that accumulated from the \n before assign and the \n\n between
    /// assign and the expression.
    #[test]
    fn test_517_assign_then_dash_output() {
        let eng = engine();
        let ctx = Object::new();
        // Issue 569: {{- url -}} now strips all preceding whitespace from the
        // output buffer at runtime, matching Ruby Liquid's behavior.
        let template = "\n{% assign url = \"hello\" %}\n\n{{- url -}}\n";
        let out = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(
            out, "hello",
            "Issue 569: {{- -}} strips all preceding whitespace at runtime"
        );
    }

    /// Issue 517: endcomment with dash should eat whitespace up to next tag
    #[test]
    fn test_517_endcomment_dash_then_assign() {
        let eng = engine();
        let ctx = Object::new();
        let template =
            "{%- comment -%}foo{%- endcomment -%}\n\n{% assign url = \"hello\" %}\n\n{{- url -}}\n";
        let out = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(
            out, "hello",
            "endcomment dash should eat subsequent whitespace"
        );
    }

    /// Issue 517/565: test with full media-url.html-like template for whitespace.
    /// Capture preserves internal whitespace verbatim -- runtime whitespace from
    /// includes that don't use dash tags will remain in the captured value.
    /// This matches Jekyll's behavior.
    #[test]
    fn test_517_media_url_full_template() {
        let mut includes = std::collections::HashMap::new();
        let media_url = r#"{%- comment -%}
  Generate media resource final URL
{%- endcomment -%}

{% assign url = include.src %}

{%- if url -%}
  {% unless url contains ':' %}
    {% assign url = include.subpath | default: '' | append: '/' | append: url %}

    {% assign url = url | replace: '///', '/' | replace: '//', '/' | replace: ':/', '://' %}

    {% unless url contains '://' %}
      {% assign url = url %}
    {% endunless %}
  {% endunless %}
{%- endif -%}

{{- url -}}
"#;
        includes.insert("media-url.html".to_string(), media_url.to_string());
        let eng = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let template = "{%- capture img_url -%}\n  {% include \"media-url.html\" src=\"/path/img.png\" %}\n{%- endcapture -%}[{{ img_url }}]";
        let out = eng.parse_and_render(template, &ctx).unwrap();
        // Capture preserves internal whitespace including newlines from includes
        // that don't use dash tags. This matches Jekyll's behavior.
        assert!(
            out.contains("/path/img.png"),
            "Issue 517/565: media-url include should produce the URL. Got: {:?}",
            out,
        );
    }

    /// Issue 517/569: include with {{- -}} dash output strips runtime whitespace.
    /// With issue 569's runtime whitespace stripping, {{- url -}} in the include
    /// template strips the preceding whitespace from the output buffer.
    #[test]
    fn test_517_include_output_dash_stripping() {
        let mut includes = std::collections::HashMap::new();
        includes.insert(
            "simple.html".to_string(),
            "\n{% assign url = include.val %}\n\n{{- url -}}\n".to_string(),
        );
        let eng = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        // Issue 569: {{- url -}} now strips all preceding whitespace at runtime
        let template = "[{% include \"simple.html\" val=\"hello\" %}]";
        let out = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(
            out, "[hello]",
            "Issue 569: {{- -}} in include strips preceding whitespace at runtime"
        );
    }

    // ========================================================================
    // Built-in filters
    // ========================================================================

    #[test]
    fn test_where_and_first_filter() {
        let eng = engine();
        let items = LiquidValue::Array(vec![
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("name".into(), LiquidValue::scalar("a"));
                o
            }),
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("name".into(), LiquidValue::scalar("b"));
                o
            }),
        ]);
        let mut ctx = Object::new();
        ctx.insert("items".into(), items);
        // where filter returns an array, first extracts the first element
        let out = eng
            .parse_and_render(
                "{% assign found = items | where: \"name\", \"b\" | first %}{{ found.name }}",
                &ctx,
            )
            .unwrap();
        assert_eq!(out, "b");
    }

    #[test]
    fn test_sort_and_first_filter() {
        let eng = engine();
        let items = LiquidValue::Array(vec![
            LiquidValue::scalar("c"),
            LiquidValue::scalar("a"),
            LiquidValue::scalar("b"),
        ]);
        let mut ctx = Object::new();
        ctx.insert("items".into(), items);
        let out = eng
            .parse_and_render("{{ items | sort | first }}", &ctx)
            .unwrap();
        assert_eq!(out, "a");
    }

    #[test]
    fn test_strip_html_filter() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert(
            "text".into(),
            LiquidValue::scalar("<p>Hello <b>World</b></p>"),
        );
        let out = eng
            .parse_and_render("{{ text | strip_html }}", &ctx)
            .unwrap();
        assert_eq!(out, "Hello World");
    }

    /// Issue 296: Does Liquid's {{ }} output strip trailing newlines from scalar values?
    #[test]
    fn test_issue296_liquid_output_preserves_trailing_newline() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("text".into(), LiquidValue::scalar("Hello World\n"));
        let out = eng.parse_and_render("{{ text }}", &ctx).unwrap();
        assert_eq!(
            out, "Hello World\n",
            "Liquid {{ }} should preserve trailing newline in scalar value. Got: {:?}",
            out
        );
    }

    /// Issue 296: strip_html should preserve trailing newlines, matching Jekyll.
    #[test]
    fn test_issue296_strip_html_preserves_trailing_newline() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("text".into(), LiquidValue::scalar("<p>Hello World</p>\n"));
        let out = eng
            .parse_and_render("{{ text | strip_html }}", &ctx)
            .unwrap();
        assert_eq!(
            out, "Hello World\n",
            "strip_html should preserve trailing newline. Got: {:?}",
            out
        );
    }

    #[test]
    fn test_default_filter() {
        let eng = engine();

        // With value set
        let mut ctx = Object::new();
        ctx.insert("x".into(), LiquidValue::scalar("present"));
        let out = eng
            .parse_and_render("{{ x | default: \"fallback\" }}", &ctx)
            .unwrap();
        assert_eq!(out, "present");

        // With value unset -- default only works on existing nil/empty values
        // in liquid; for completely missing variables the render may fail,
        // so we assign nil first
        let ctx2 = Object::new();
        let out2 = eng
            .parse_and_render("{% assign x = nil %}{{ x | default: \"fallback\" }}", &ctx2)
            .unwrap();
        assert_eq!(out2, "fallback");
    }

    #[test]
    fn test_plus_filter() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("x".into(), LiquidValue::scalar(5i64));
        let out = eng.parse_and_render("{{ x | plus: 1 }}", &ctx).unwrap();
        assert_eq!(out, "6");
    }

    #[test]
    fn test_map_and_join_filters() {
        let eng = engine();
        let items = LiquidValue::Array(vec![
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("name".into(), LiquidValue::scalar("Alice"));
                o
            }),
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("name".into(), LiquidValue::scalar("Bob"));
                o
            }),
        ]);
        let mut ctx = Object::new();
        ctx.insert("items".into(), items);
        let out = eng
            .parse_and_render("{{ items | map: \"name\" | join: \", \" }}", &ctx)
            .unwrap();
        assert_eq!(out, "Alice, Bob");
    }

    #[test]
    fn test_reverse_filter() {
        let eng = engine();
        let items = LiquidValue::Array(vec![
            LiquidValue::scalar("a"),
            LiquidValue::scalar("b"),
            LiquidValue::scalar("c"),
        ]);
        let mut ctx = Object::new();
        ctx.insert("items".into(), items);
        let out = eng
            .parse_and_render("{{ items | reverse | join: \"\" }}", &ctx)
            .unwrap();
        assert_eq!(out, "cba");
    }

    #[test]
    fn test_truncate_filter() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert(
            "text".into(),
            LiquidValue::scalar("Hello World, this is a long text"),
        );
        let out = eng
            .parse_and_render("{{ text | truncate: 10 }}", &ctx)
            .unwrap();
        assert_eq!(out, "Hello W...");
    }

    #[test]
    fn test_slugify_filter() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("title".into(), LiquidValue::scalar("Hello World!"));
        let out = eng.parse_and_render("{{ title | slugify }}", &ctx).unwrap();
        assert_eq!(out, "hello-world");
    }

    // ========================================================================
    // Edge cases
    // ========================================================================

    #[test]
    fn test_nil_in_condition_is_falsy() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("x".into(), LiquidValue::Nil);
        let out = eng
            .parse_and_render("{% if x %}truthy{% else %}falsy{% endif %}", &ctx)
            .unwrap();
        assert_eq!(out, "falsy");
    }

    #[test]
    fn test_parse_error_returns_template_error() {
        let eng = engine();
        let result = eng.parse("{% for %}");
        assert!(matches!(result, Err(TemplateError::ParseError(_))));
    }

    #[test]
    fn test_deeply_nested_dot_access() {
        let eng = engine();
        let mut d = Object::new();
        d.insert("value".into(), LiquidValue::scalar("deep"));
        let mut c = Object::new();
        c.insert("d".into(), LiquidValue::Object(d));
        let mut b = Object::new();
        b.insert("c".into(), LiquidValue::Object(c));
        let mut a = Object::new();
        a.insert("b".into(), LiquidValue::Object(b));
        let mut ctx = Object::new();
        ctx.insert("a".into(), LiquidValue::Object(a));
        let out = eng.parse_and_render("{{ a.b.c.d.value }}", &ctx).unwrap();
        assert_eq!(out, "deep");
    }

    #[test]
    fn test_integer_zero_is_truthy() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("x".into(), LiquidValue::scalar(0i64));
        let out = eng
            .parse_and_render("{% if x %}truthy{% else %}falsy{% endif %}", &ctx)
            .unwrap();
        // Liquid considers 0 truthy (unlike many languages)
        assert_eq!(out, "truthy");
    }

    // ========================================================================
    // Integration: Realistic template rendering
    // ========================================================================

    #[test]
    fn test_realistic_author_pattern() {
        let eng = engine();

        // Simulate: posts array with author field, using where + for + unless forloop.last
        let posts = LiquidValue::Array(vec![
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("title".into(), LiquidValue::scalar("Post A"));
                o.insert("author".into(), LiquidValue::scalar("alice"));
                o
            }),
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("title".into(), LiquidValue::scalar("Post B"));
                o.insert("author".into(), LiquidValue::scalar("bob"));
                o
            }),
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("title".into(), LiquidValue::scalar("Post C"));
                o.insert("author".into(), LiquidValue::scalar("alice"));
                o
            }),
        ]);
        let mut ctx = Object::new();
        ctx.insert("posts".into(), posts);

        let template = r#"{% assign alice_posts = posts | where: "author", "alice" %}{% for post in alice_posts %}{{ post.title }}{% unless forloop.last %}, {% endunless %}{% endfor %}"#;
        let out = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(out, "Post A, Post C");
    }

    #[test]
    fn test_realistic_podcast_capture_pattern() {
        let eng = engine();

        let mut links = Object::new();
        links.insert(
            "youtube".into(),
            LiquidValue::scalar("https://youtube.com/watch?v=abc"),
        );
        links.insert(
            "spotify".into(),
            LiquidValue::scalar("https://open.spotify.com/episode/xyz"),
        );
        let mut page = Object::new();
        page.insert("links".into(), LiquidValue::Object(links));
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page));

        let template = r#"{% capture actions %}{% if page.links.youtube %}<a href="{{ page.links.youtube }}">YouTube</a> {% endif %}{% if page.links.spotify %}<a href="{{ page.links.spotify }}">Spotify</a>{% endif %}{% endcapture %}{{ actions }}"#;
        let out = eng.parse_and_render(template, &ctx).unwrap();
        assert!(out.contains("YouTube"));
        assert!(out.contains("Spotify"));
        assert!(out.contains("https://youtube.com/watch?v=abc"));
    }

    // ========================================================================
    // Integration tests: custom filters (Issue 07)
    // ========================================================================

    #[test]
    fn test_integration_where_exp_with_size() {
        let eng = engine();
        let items = LiquidValue::Array(vec![
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("draft".into(), LiquidValue::scalar(true));
                o.insert("name".into(), LiquidValue::scalar("a"));
                o
            }),
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("draft".into(), LiquidValue::scalar(false));
                o.insert("name".into(), LiquidValue::scalar("b"));
                o
            }),
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("name".into(), LiquidValue::scalar("c"));
                o
            }),
        ]);
        let mut ctx = Object::new();
        ctx.insert("items".into(), items);
        let out = eng
            .parse_and_render(
                r#"{{ items | where_exp: "item", "item.draft != true" | size }}"#,
                &ctx,
            )
            .unwrap();
        assert_eq!(out, "2");
    }

    #[test]
    fn test_integration_where_exp_chaining() {
        let eng = engine();
        let items = LiquidValue::Array(vec![
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("draft".into(), LiquidValue::scalar(true));
                o.insert("time".into(), LiquidValue::scalar(20i64));
                o
            }),
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("draft".into(), LiquidValue::scalar(false));
                o.insert("time".into(), LiquidValue::scalar(20i64));
                o
            }),
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("draft".into(), LiquidValue::scalar(false));
                o.insert("time".into(), LiquidValue::scalar(5i64));
                o
            }),
        ]);
        let mut ctx = Object::new();
        ctx.insert("items".into(), items);
        let out = eng
            .parse_and_render(
                r#"{{ items | where_exp: "e", "e.draft != true" | where_exp: "e", "e.time > 10" | size }}"#,
                &ctx,
            )
            .unwrap();
        assert_eq!(out, "1");
    }

    #[test]
    fn test_integration_where_exp_runtime_context() {
        let eng = engine();
        let items = LiquidValue::Array(vec![
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert(
                    "authors".into(),
                    LiquidValue::Array(vec![
                        LiquidValue::scalar("alice"),
                        LiquidValue::scalar("bob"),
                    ]),
                );
                o.insert("title".into(), LiquidValue::scalar("Post 1"));
                o
            }),
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert(
                    "authors".into(),
                    LiquidValue::Array(vec![LiquidValue::scalar("carol")]),
                );
                o.insert("title".into(), LiquidValue::scalar("Post 2"));
                o
            }),
        ]);
        let mut page = Object::new();
        page.insert("short".into(), LiquidValue::scalar("alice"));
        let mut ctx = Object::new();
        ctx.insert("items".into(), items);
        ctx.insert("page".into(), LiquidValue::Object(page));
        let out = eng
            .parse_and_render(
                r#"{% assign found = items | where_exp: "post", "post.authors contains page.short" %}{{ found | map: "title" | join: ", " }}"#,
                &ctx,
            )
            .unwrap();
        assert_eq!(out, "Post 1");
    }

    #[test]
    fn test_integration_jsonify_string() {
        let eng = engine();
        let mut page = Object::new();
        page.insert("title".into(), LiquidValue::scalar("My Title"));
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page));
        let out = eng
            .parse_and_render("{{ page.title | jsonify }}", &ctx)
            .unwrap();
        assert_eq!(out, "\"My Title\"");
    }

    #[test]
    fn test_integration_date_to_string() {
        let eng = engine();
        let mut page = Object::new();
        page.insert("date".into(), LiquidValue::scalar("2024-03-15"));
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page));
        let out = eng
            .parse_and_render("{{ page.date | date_to_string }}", &ctx)
            .unwrap();
        assert_eq!(out, "15 Mar 2024");
    }

    /// Issue 140: naive YAML timestamp "2025-10-10 23:59:59" should produce
    /// "11 Oct 2025" (not "10 Oct 2025") when the site timezone is Europe/Berlin,
    /// because Ruby YAML treats this as UTC and Jekyll converts to local time.
    #[test]
    fn test_integration_date_to_string_book_end_date_with_timezone() {
        let eng = engine();
        let mut book = Object::new();
        book.insert("end".into(), LiquidValue::scalar("2025-10-10 23:59:59"));
        let mut site = Object::new();
        site.insert("timezone".into(), LiquidValue::scalar("Europe/Berlin"));
        let mut ctx = Object::new();
        ctx.insert("book".into(), LiquidValue::Object(book));
        ctx.insert("site".into(), LiquidValue::Object(site));
        let out = eng
            .parse_and_render("{{ book.end | date_to_string }}", &ctx)
            .unwrap();
        // CET (UTC+1 in Oct) shifts 23:59 UTC to 01:59 next day
        assert_eq!(out, "11 Oct 2025");
    }

    /// Issue 140: with UTC timezone, the naive datetime stays on same day
    /// since UTC->UTC conversion doesn't shift the date.
    #[test]
    fn test_integration_date_to_string_book_end_date_utc() {
        let eng = engine();
        let mut book = Object::new();
        book.insert("end".into(), LiquidValue::scalar("2025-10-10 23:59:59"));
        let mut site = Object::new();
        site.insert("timezone".into(), LiquidValue::scalar("UTC"));
        let mut ctx = Object::new();
        ctx.insert("book".into(), LiquidValue::Object(book));
        ctx.insert("site".into(), LiquidValue::Object(site));
        let out = eng
            .parse_and_render("{{ book.end | date_to_string }}", &ctx)
            .unwrap();
        assert_eq!(out, "10 Oct 2025");
    }

    /// Issue 140: full YAML-to-template pipeline for book date range.
    /// Parses YAML front matter, converts to Liquid values, and renders
    /// the same template pattern used on books.html.
    #[test]
    fn test_integration_book_date_range_yaml_pipeline() {
        let eng = engine();
        let yaml_str = r#"
start: 2025-10-06 00:00:00
end: 2025-10-10 23:59:59
title: "Test Book"
"#;
        let yaml: serde_yaml::Value = crate::yaml::parse_yaml_lenient(yaml_str).unwrap();
        let book_liquid = crate::template::context::yaml_to_liquid(&yaml);

        let mut site = Object::new();
        site.insert("timezone".into(), LiquidValue::scalar("Europe/Berlin"));
        let mut ctx = Object::new();
        ctx.insert("book".into(), book_liquid);
        ctx.insert("site".into(), LiquidValue::Object(site));
        let out = eng
            .parse_and_render(
                "(from {{ book.start | date_to_string }} to {{ book.end | date_to_string }})",
                &ctx,
            )
            .unwrap();
        // Europe/Berlin in October is CET (UTC+1):
        // start 2025-10-06 00:00:00 UTC -> 2025-10-06 01:00:00 CET -> "06 Oct 2025"
        // end   2025-10-10 23:59:59 UTC -> 2025-10-11 00:59:59 CET -> "11 Oct 2025"
        assert_eq!(out, "(from 06 Oct 2025 to 11 Oct 2025)");
    }

    /// Issue 140: multiple book end dates that historically had off-by-one.
    /// Covers different months and DST vs non-DST periods.
    #[test]
    fn test_integration_book_end_dates_various_months() {
        let eng = engine();
        let mut site = Object::new();
        site.insert("timezone".into(), LiquidValue::scalar("Europe/Berlin"));

        // Each (input, expected_output) pair
        let cases = [
            // December: CET (UTC+1), 23:59 UTC -> 00:59+1 CET next day
            ("2020-12-18 23:59:59", "19 Dec 2020"),
            // September: CEST (UTC+2), 22:59 UTC -> 00:59+1 CEST next day
            ("2021-09-10 22:59:58", "11 Sep 2021"),
            // July: CEST (UTC+2), 22:59 UTC -> 00:59+1 CEST next day
            ("2021-07-16 22:59:58", "17 Jul 2021"),
            // March: CET (UTC+1), 22:59 UTC -> 23:59 CET same day
            ("2021-03-12 22:59:58", "12 Mar 2021"),
            // Start date at midnight UTC -> stays same day
            ("2020-12-14 00:00:00", "14 Dec 2020"),
        ];

        for (input, expected) in &cases {
            let mut book = Object::new();
            book.insert("end".into(), LiquidValue::scalar(*input));
            let mut ctx = Object::new();
            ctx.insert("book".into(), LiquidValue::Object(book));
            ctx.insert("site".into(), LiquidValue::Object(site.clone()));
            let out = eng
                .parse_and_render("{{ book.end | date_to_string }}", &ctx)
                .unwrap();
            assert_eq!(
                out, *expected,
                "Input '{}' should produce '{}', got '{}'",
                input, expected, out
            );
        }
    }

    #[test]
    fn test_integration_date_to_xmlschema_and_jsonify_chained() {
        let eng = engine();
        let mut page = Object::new();
        page.insert("date".into(), LiquidValue::scalar("2024-01-15"));
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page));
        let out = eng
            .parse_and_render("{{ page.date | date_to_xmlschema | jsonify }}", &ctx)
            .unwrap();
        assert_eq!(out, "\"2024-01-15T00:00:00+00:00\"");
    }

    #[test]
    fn test_integration_markdownify() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("text".into(), LiquidValue::scalar("**bold** and *italic*"));
        let out = eng
            .parse_and_render("{{ text | markdownify }}", &ctx)
            .unwrap();
        assert!(out.contains("<strong>bold</strong>"));
        assert!(out.contains("<em>italic</em>"));
    }

    #[test]
    fn test_integration_relative_url_no_baseurl() {
        let eng = engine();
        let mut page = Object::new();
        page.insert("image".into(), LiquidValue::scalar("images/photo.jpg"));
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page));
        let out = eng
            .parse_and_render("{{ page.image | relative_url }}", &ctx)
            .unwrap();
        assert_eq!(out, "/images/photo.jpg");
    }

    #[test]
    fn test_integration_relative_url_with_baseurl() {
        let eng = engine();
        let mut site = Object::new();
        site.insert("baseurl".into(), LiquidValue::scalar("/blog"));
        let mut page = Object::new();
        page.insert("image".into(), LiquidValue::scalar("images/photo.jpg"));
        let mut ctx = Object::new();
        ctx.insert("site".into(), LiquidValue::Object(site));
        ctx.insert("page".into(), LiquidValue::Object(page));
        let out = eng
            .parse_and_render("{{ page.image | relative_url }}", &ctx)
            .unwrap();
        assert_eq!(out, "/blog/images/photo.jpg");
    }

    #[test]
    fn test_integration_realistic_json_ld() {
        let eng = engine();
        let mut page = Object::new();
        page.insert("title".into(), LiquidValue::scalar("My Post"));
        page.insert("date".into(), LiquidValue::scalar("2024-01-15"));
        page.insert(
            "description".into(),
            LiquidValue::scalar("A \"great\" post"),
        );
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page));
        let template = r#"{"name":{{ page.title | jsonify }},"datePublished":{{ page.date | date_to_xmlschema | jsonify }},"description":{{ page.description | jsonify }}}"#;
        let out = eng.parse_and_render(template, &ctx).unwrap();
        assert!(out.contains("\"My Post\""));
        assert!(out.contains("\"2024-01-15T00:00:00+00:00\""));
        assert!(out.contains("\"A \\\"great\\\" post\""));
        // Verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["name"], "My Post");
    }

    // ========================================================================
    // Issue 24: absolute_url filter with runtime context
    // ========================================================================

    fn ctx_with_site(url: &str, baseurl: &str) -> Object {
        let mut site = Object::new();
        site.insert("url".into(), LiquidValue::scalar(url.to_owned()));
        site.insert("baseurl".into(), LiquidValue::scalar(baseurl.to_owned()));
        let mut ctx = Object::new();
        ctx.insert("site".into(), LiquidValue::Object(site));
        ctx
    }

    #[test]
    fn test_absolute_url_basic() {
        let eng = engine();
        let ctx = ctx_with_site("https://example.com", "");
        let out = eng
            .parse_and_render(r#"{{ "/about.html" | absolute_url }}"#, &ctx)
            .unwrap();
        assert_eq!(out, "https://example.com/about.html");
    }

    #[test]
    fn test_absolute_url_with_baseurl() {
        let eng = engine();
        let ctx = ctx_with_site("https://example.com", "/blog");
        let out = eng
            .parse_and_render(r#"{{ "/about.html" | absolute_url }}"#, &ctx)
            .unwrap();
        assert_eq!(out, "https://example.com/blog/about.html");
    }

    #[test]
    fn test_absolute_url_no_leading_slash() {
        let eng = engine();
        let ctx = ctx_with_site("https://example.com", "/blog");
        let out = eng
            .parse_and_render(r#"{{ "about.html" | absolute_url }}"#, &ctx)
            .unwrap();
        assert_eq!(out, "https://example.com/blog/about.html");
    }

    #[test]
    fn test_absolute_url_trailing_slash_on_url() {
        let eng = engine();
        let ctx = ctx_with_site("https://example.com/", "/blog");
        let out = eng
            .parse_and_render(r#"{{ "/page" | absolute_url }}"#, &ctx)
            .unwrap();
        assert_eq!(out, "https://example.com/blog/page");
    }

    #[test]
    fn test_absolute_url_trailing_slash_on_baseurl() {
        let eng = engine();
        let ctx = ctx_with_site("https://example.com", "/blog/");
        let out = eng
            .parse_and_render(r#"{{ "/page" | absolute_url }}"#, &ctx)
            .unwrap();
        assert_eq!(out, "https://example.com/blog/page");
    }

    #[test]
    fn test_absolute_url_empty_url_empty_baseurl() {
        let eng = engine();
        let ctx = ctx_with_site("", "");
        let out = eng
            .parse_and_render(r#"{{ "/about.html" | absolute_url }}"#, &ctx)
            .unwrap();
        assert_eq!(out, "/about.html");
    }

    #[test]
    fn test_absolute_url_empty_url_with_baseurl() {
        let eng = engine();
        let ctx = ctx_with_site("", "/blog");
        let out = eng
            .parse_and_render(r#"{{ "/about.html" | absolute_url }}"#, &ctx)
            .unwrap();
        assert_eq!(out, "/blog/about.html");
    }

    #[test]
    fn test_absolute_url_empty_input() {
        let eng = engine();
        let ctx = ctx_with_site("https://example.com", "");
        let out = eng
            .parse_and_render(r#"{{ "" | absolute_url }}"#, &ctx)
            .unwrap();
        assert_eq!(out, "https://example.com");
    }

    // ========================================================================
    // Issue 24: relative_url filter with baseurl in context
    // ========================================================================

    #[test]
    fn test_relative_url_with_baseurl_context() {
        let eng = engine();
        let ctx = ctx_with_site("https://example.com", "/blog");
        let out = eng
            .parse_and_render(r#"{{ "/assets/style.css" | relative_url }}"#, &ctx)
            .unwrap();
        assert_eq!(out, "/blog/assets/style.css");
    }

    #[test]
    fn test_relative_url_empty_baseurl_context() {
        let eng = engine();
        let ctx = ctx_with_site("https://example.com", "");
        let out = eng
            .parse_and_render(r#"{{ "/assets/style.css" | relative_url }}"#, &ctx)
            .unwrap();
        assert_eq!(out, "/assets/style.css");
    }

    // ========================================================================
    // Issue 24: End-to-end template rendering with absolute_url
    // ========================================================================

    #[test]
    fn test_absolute_url_in_html_template() {
        let eng = engine();
        let ctx = ctx_with_site("https://example.com", "/blog");
        let template = r#"<a href="{{ "/about" | absolute_url }}">About</a>"#;
        let out = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(out, r#"<a href="https://example.com/blog/about">About</a>"#);
    }

    #[test]
    fn test_relative_url_in_html_template() {
        let eng = engine();
        let ctx = ctx_with_site("https://example.com", "/blog");
        let template = r#"<link href="{{ "/style.css" | relative_url }}">"#;
        let out = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(out, r#"<link href="/blog/style.css">"#);
    }

    // ========================================================================
    // Issue 23: Extras rendered through template engine
    // ========================================================================

    #[test]
    fn test_site_extras_in_template() {
        let eng = engine();
        let mut site = Object::new();
        site.insert("locale".into(), LiquidValue::scalar("en"));
        site.insert("author".into(), LiquidValue::scalar("Alice"));
        let mut ctx = Object::new();
        ctx.insert("site".into(), LiquidValue::Object(site));
        let out = eng
            .parse_and_render("{{ site.locale }}-{{ site.author }}", &ctx)
            .unwrap();
        assert_eq!(out, "en-Alice");
    }

    #[test]
    fn test_site_twitter_map_in_template() {
        let eng = engine();
        let mut twitter = Object::new();
        twitter.insert("username".into(), LiquidValue::scalar("handle"));
        let mut site = Object::new();
        site.insert("twitter".into(), LiquidValue::Object(twitter));
        let mut ctx = Object::new();
        ctx.insert("site".into(), LiquidValue::Object(site));
        let out = eng
            .parse_and_render("{{ site.twitter.username }}", &ctx)
            .unwrap();
        assert_eq!(out, "handle");
    }

    #[test]
    fn test_site_nested_extra_in_template() {
        let eng = engine();
        let mut sass = Object::new();
        sass.insert("style".into(), LiquidValue::scalar("compressed"));
        let mut site = Object::new();
        site.insert("sass".into(), LiquidValue::Object(sass));
        let mut ctx = Object::new();
        ctx.insert("site".into(), LiquidValue::Object(site));
        let out = eng.parse_and_render("{{ site.sass.style }}", &ctx).unwrap();
        assert_eq!(out, "compressed");
    }

    // ========================================================================
    // normalize_whitespace filter (Issue 37)
    // ========================================================================

    #[test]
    fn test_normalize_whitespace_filter() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert(
            "text".into(),
            LiquidValue::scalar("  hello   world\n\t foo  "),
        );
        let out = eng
            .parse_and_render("{{ text | normalize_whitespace }}", &ctx)
            .unwrap();
        assert_eq!(out, "hello world foo");
    }

    #[test]
    fn test_normalize_whitespace_empty() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("text".into(), LiquidValue::scalar(""));
        let out = eng
            .parse_and_render("{{ text | normalize_whitespace }}", &ctx)
            .unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn test_normalize_whitespace_already_clean() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("text".into(), LiquidValue::scalar("already clean"));
        let out = eng
            .parse_and_render("{{ text | normalize_whitespace }}", &ctx)
            .unwrap();
        assert_eq!(out, "already clean");
    }

    #[test]
    fn test_normalize_whitespace_with_truncate() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert(
            "description".into(),
            LiquidValue::scalar("  hello   world\n\t foo  "),
        );
        let out = eng
            .parse_and_render(
                "{{ description | normalize_whitespace | truncate: 10 }}",
                &ctx,
            )
            .unwrap();
        assert_eq!(out, "hello w...");
    }

    // ========================================================================
    // Unknown filter handling (Issue 37)
    // ========================================================================

    #[test]
    fn test_unknown_filter_passes_through() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("value".into(), LiquidValue::scalar("hello"));
        let out = eng
            .parse_and_render("{{ value | nonexistent_filter }}", &ctx)
            .unwrap();
        assert_eq!(out, "hello");
    }

    #[test]
    fn test_unknown_filter_with_known_filters() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("value".into(), LiquidValue::scalar("hello"));
        let out = eng
            .parse_and_render("{{ value | upcase | nonexistent_xyz }}", &ctx)
            .unwrap();
        assert_eq!(out, "HELLO");
    }

    #[test]
    fn test_unknown_filter_erl_encode() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("value".into(), LiquidValue::scalar("test string"));
        let out = eng
            .parse_and_render("{{ value | erl_encode }}", &ctx)
            .unwrap();
        assert_eq!(out, "test string");
    }

    #[test]
    fn test_multiple_unknown_filters() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("value".into(), LiquidValue::scalar("test"));
        let out = eng
            .parse_and_render("{{ value | fake_one | fake_two }}", &ctx)
            .unwrap();
        assert_eq!(out, "test");
    }

    // ========================================================================
    // Issue 44: Hash integer indexing
    // ========================================================================

    /// Helper to build a page-level object for hash indexing tests.
    /// Objects must be in the "page" namespace to go through LenientValue.
    fn page_ctx_with_hash(key: &'static str, obj: Object) -> Object {
        let mut page = Object::new();
        page.insert(key.into(), LiquidValue::Object(obj));
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page));
        ctx
    }

    #[test]
    fn test_hash_integer_index_first_entry() {
        let eng = engine();
        // Use a single-entry object for deterministic ordering
        let mut obj = Object::new();
        obj.insert("mykey".into(), LiquidValue::scalar(42i64));
        let ctx = page_ctx_with_hash("obj", obj);
        let out = eng.parse_and_render("{{ page.obj[0][0] }}", &ctx).unwrap();
        assert_eq!(out, "mykey");
    }

    #[test]
    fn test_hash_integer_index_returns_key() {
        let eng = engine();
        let mut obj = Object::new();
        obj.insert("name".into(), LiquidValue::scalar("Alice"));
        let ctx = page_ctx_with_hash("obj", obj);
        let out = eng.parse_and_render("{{ page.obj[0][0] }}", &ctx).unwrap();
        assert_eq!(out, "name");
    }

    #[test]
    fn test_hash_integer_index_returns_value() {
        let eng = engine();
        let mut obj = Object::new();
        obj.insert("name".into(), LiquidValue::scalar("Alice"));
        let ctx = page_ctx_with_hash("obj", obj);
        let out = eng.parse_and_render("{{ page.obj[0][1] }}", &ctx).unwrap();
        assert_eq!(out, "Alice");
    }

    #[test]
    fn test_hash_integer_index_out_of_bounds() {
        let eng = engine();
        let mut obj = Object::new();
        obj.insert("a".into(), LiquidValue::scalar(1i64));
        obj.insert("b".into(), LiquidValue::scalar(2i64));
        obj.insert("c".into(), LiquidValue::scalar(3i64));
        let ctx = page_ctx_with_hash("obj", obj);
        let out = eng.parse_and_render("{{ page.obj[5] }}", &ctx).unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn test_hash_integer_index_negative() {
        let eng = engine();
        let mut obj = Object::new();
        obj.insert("a".into(), LiquidValue::scalar(1i64));
        let ctx = page_ctx_with_hash("obj", obj);
        let out = eng.parse_and_render("{{ page.obj[-1] }}", &ctx).unwrap();
        // Negative indexing on hashes returns nil in Jekyll
        assert_eq!(out, "");
    }

    #[test]
    fn test_hash_string_key_priority_over_integer() {
        let eng = engine();
        let mut obj = Object::new();
        obj.insert("0".into(), LiquidValue::scalar("zero"));
        obj.insert("a".into(), LiquidValue::scalar(1i64));
        let ctx = page_ctx_with_hash("obj", obj);
        // Key "0" exists as a string key, so it should be returned directly
        let out = eng.parse_and_render("{{ page.obj[0] }}", &ctx).unwrap();
        assert_eq!(out, "zero");
    }

    #[test]
    fn test_hash_normal_string_access_unaffected() {
        let eng = engine();
        let mut obj = Object::new();
        obj.insert("name".into(), LiquidValue::scalar("Alice"));
        let ctx = page_ctx_with_hash("obj", obj);
        let out = eng.parse_and_render("{{ page.obj.name }}", &ctx).unwrap();
        assert_eq!(out, "Alice");
    }

    #[test]
    fn test_hash_bracket_string_access_unaffected() {
        let eng = engine();
        let mut obj = Object::new();
        obj.insert("name".into(), LiquidValue::scalar("Alice"));
        let ctx = page_ctx_with_hash("obj", obj);
        let out = eng
            .parse_and_render(r#"{{ page.obj["name"] }}"#, &ctx)
            .unwrap();
        assert_eq!(out, "Alice");
    }

    #[test]
    fn test_hash_integer_index_with_assign() {
        let eng = engine();
        let mut obj = Object::new();
        obj.insert("en".into(), LiquidValue::scalar("English"));
        obj.insert("es".into(), LiquidValue::scalar("Spanish"));
        let ctx = page_ctx_with_hash("locales", obj);
        // Assign from hash[0] then access the key
        let out = eng
            .parse_and_render("{% assign first = page.locales[0] %}{{ first[0] }}", &ctx)
            .unwrap();
        // Object uses HashMap, so iteration order is not guaranteed
        assert!(
            out == "en" || out == "es",
            "Expected 'en' or 'es', got: {}",
            out
        );
    }

    #[test]
    fn test_hash_integer_index_site_data() {
        let eng = engine();
        let mut locales = Object::new();
        locales.insert("en".into(), LiquidValue::scalar("English"));
        locales.insert("es".into(), LiquidValue::scalar("Spanish"));
        let mut data = Object::new();
        data.insert("locales".into(), LiquidValue::Object(locales));
        let mut site = Object::new();
        site.insert("data".into(), LiquidValue::Object(data));
        let mut ctx = Object::new();
        ctx.insert("site".into(), LiquidValue::Object(site));

        // Object uses HashMap, so order is not guaranteed.
        // Verify we get a valid locale key.
        let out = eng
            .parse_and_render(
                "{% assign first = site.data.locales[0] %}{{ first[0] }}",
                &ctx,
            )
            .unwrap();
        assert!(
            out == "en" || out == "es",
            "Expected 'en' or 'es', got: {}",
            out
        );
    }

    #[test]
    fn test_hash_integer_index_second_entry() {
        let eng = engine();
        let mut obj = Object::new();
        obj.insert("a".into(), LiquidValue::scalar(1i64));
        obj.insert("b".into(), LiquidValue::scalar(2i64));
        obj.insert("c".into(), LiquidValue::scalar(3i64));
        let ctx = page_ctx_with_hash("obj", obj);
        // Object uses HashMap so iteration order is not guaranteed.
        // Just verify that [1] returns a valid [key, value] pair.
        let key = eng.parse_and_render("{{ page.obj[1][0] }}", &ctx).unwrap();
        let value = eng.parse_and_render("{{ page.obj[1][1] }}", &ctx).unwrap();
        assert!(
            ["a", "b", "c"].contains(&key.as_str()),
            "Expected a valid key, got: {}",
            key
        );
        assert!(
            ["1", "2", "3"].contains(&value.as_str()),
            "Expected a valid value, got: {}",
            value
        );
    }

    // ========================================================================
    // Issue 53: include_cached tag registration
    // ========================================================================

    #[test]
    fn test_include_cached_renders_same_as_include() {
        let mut includes = HashMap::new();
        includes.insert(
            "foo.html".to_string(),
            "Hello {{ include.name }}!".to_string(),
        );
        let eng = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();

        let output_cached = eng
            .parse_and_render(r#"{% include_cached foo.html name="World" %}"#, &ctx)
            .unwrap();
        let output_regular = eng
            .parse_and_render(r#"{% include foo.html name="World" %}"#, &ctx)
            .unwrap();
        assert_eq!(output_cached, output_regular);
        assert_eq!(output_cached, "Hello World!");
    }

    #[test]
    fn test_include_cached_with_variable_param() {
        let mut includes = HashMap::new();
        includes.insert(
            "greeting.html".to_string(),
            "Hi {{ include.locale }}!".to_string(),
        );
        let eng = TemplateEngine::with_includes_map(&includes).unwrap();
        let mut ctx = Object::new();
        ctx.insert("locale".into(), LiquidValue::scalar("en"));

        let output = eng
            .parse_and_render(r#"{% include_cached greeting.html locale=locale %}"#, &ctx)
            .unwrap();
        assert_eq!(output, "Hi en!");
    }

    #[test]
    fn test_include_cached_missing_partial_errors() {
        let includes = HashMap::new();
        let eng = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();

        let result = eng.parse_and_render("{% include_cached missing.html %}", &ctx);
        assert!(result.is_err(), "Should error on missing partial");
    }

    #[test]
    fn test_date_filter_with_format_string() {
        let eng = engine();
        let mut page = Object::new();
        page.insert("date".into(), LiquidValue::scalar("2024-07-24"));
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page));
        let out = eng
            .parse_and_render(r#"{{ page.date | date: "%d.%m.%Y" }}"#, &ctx)
            .unwrap();
        assert_eq!(out, "24.07.2024");
    }

    // ========================================================================
    // Issue 74: Capture tag tolerance
    // ========================================================================

    #[test]
    fn test_capture_with_extra_tokens() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("name".into(), LiquidValue::scalar("World"));
        let out = eng
            .parse_and_render(
                "{% capture myvar do %}hello {{ name }}{% endcapture %}{{ myvar }}",
                &ctx,
            )
            .unwrap();
        assert_eq!(out, "hello World");
    }

    #[test]
    fn test_capture_with_multiple_extra_tokens() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render(
                "{% capture msg extra ignored tokens %}content{% endcapture %}{{ msg }}",
                &ctx,
            )
            .unwrap();
        assert_eq!(out, "content");
    }

    #[test]
    fn test_capture_normal_still_works() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render("{% capture msg %}hello{% endcapture %}{{ msg }}", &ctx)
            .unwrap();
        assert_eq!(out, "hello");
    }

    // ========================================================================
    // Issue 74: Jekyll tag preprocessing (link, post_url)
    // ========================================================================

    #[test]
    fn test_link_tag_preprocessing() {
        // Collection docs (path starts with _) should produce extensionless URLs
        let result = preprocess_jekyll_tags(r#"<a href="{% link _pages/about.md %}">About</a>"#);
        assert_eq!(result, r#"<a href="/pages/about">About</a>"#);
    }

    #[test]
    fn test_post_url_tag_preprocessing() {
        // post_url should produce URL with trailing slash matching Jekyll's permalink
        let result = preprocess_jekyll_tags("{% post_url 2022-09-07-homebrew-3.6.0 %}");
        assert_eq!(result, "/2022/09/07/homebrew-3.6.0/");
    }

    #[test]
    fn test_capture_preprocess_with_dash() {
        let result = preprocess_capture_tags("{%- capture myvar do -%}content{%- endcapture -%}");
        assert_eq!(result, "{%- capture myvar -%}content{%- endcapture -%}");
    }

    // Issue 171: preprocess_nil_contains tests

    #[test]
    fn test_issue171_preprocess_nil_contains_simple_if() {
        let input = r#"{% if page.path contains "zh-TW" %}yes{% endif %}"#;
        let output = preprocess_nil_contains(input);
        assert!(
            output.contains(r#"page.path and page.path contains "zh-TW""#),
            "Should add nil guard, got: {}",
            output
        );
    }

    #[test]
    fn test_issue171_preprocess_nil_contains_elsif() {
        let input = r#"{% elsif page.path contains "de-DE" %}"#;
        let output = preprocess_nil_contains(input);
        assert!(
            output.contains(r#"page.path and page.path contains "de-DE""#),
            "Should add nil guard to elsif, got: {}",
            output
        );
    }

    #[test]
    fn test_issue171_preprocess_nil_contains_no_change_for_other_tags() {
        let input = r#"{% assign x = "hello" %}{% for item in items %}{% endfor %}"#;
        let output = preprocess_nil_contains(input);
        assert_eq!(output, input, "Should not modify non-if/elsif tags");
    }

    #[test]
    fn test_issue171_preprocess_nil_contains_no_change_without_contains() {
        let input = r#"{% if page.title == "hello" %}yes{% endif %}"#;
        let output = preprocess_nil_contains(input);
        assert_eq!(output, input, "Should not modify if without contains");
    }

    #[test]
    fn test_issue171_preprocess_nil_contains_with_or() {
        let input = r#"{% if url contains "play.google.com" or url contains "itunes.apple.com" %}"#;
        let output = preprocess_nil_contains(input);
        assert!(
            output.contains("url and url contains \"play.google.com\""),
            "Should guard first contains, got: {}",
            output
        );
        assert!(
            output.contains("url and url contains \"itunes.apple.com\""),
            "Should guard second contains, got: {}",
            output
        );
    }

    #[test]
    fn test_issue171_nil_contains_render_with_nil_variable() {
        // End-to-end: template with contains on nil variable should render without error
        let eng = TemplateEngine::new().unwrap();
        let ctx = Object::new(); // no page.path defined
        let output = eng
            .parse_and_render(
                r#"{% if page.path contains "zh-TW" %}ZH{% else %}DEFAULT{% endif %}"#,
                &ctx,
            )
            .unwrap();
        assert_eq!(output.trim(), "DEFAULT");
    }

    #[test]
    fn test_issue171_nil_contains_render_with_set_variable() {
        // When the variable IS set, contains should still work correctly
        let eng = TemplateEngine::new().unwrap();
        let mut ctx = Object::new();
        let mut page = Object::new();
        page.insert(
            "path".into(),
            Value::scalar("posts/zh-TW/hello.md".to_owned()),
        );
        ctx.insert("page".into(), Value::Object(page));
        let output = eng
            .parse_and_render(
                r#"{% if page.path contains "zh-TW" %}ZH{% else %}DEFAULT{% endif %}"#,
                &ctx,
            )
            .unwrap();
        assert_eq!(output.trim(), "ZH");
    }

    #[test]
    fn test_issue171_preprocess_preserves_dash_whitespace_control() {
        let input = r#"{%- if page.path contains "zh-TW" -%}yes{%- endif -%}"#;
        let output = preprocess_nil_contains(input);
        assert!(output.starts_with("{%-"), "Should preserve leading dash");
        assert!(output.contains("-%}"), "Should preserve trailing dash");
        assert!(
            output.contains("page.path and page.path contains"),
            "Should add nil guard, got: {}",
            output
        );
    }

    // ========================================================================
    // Issue 326: nil == false preprocessing
    // ========================================================================

    #[test]
    fn test_preprocess_nil_eq_false_basic() {
        let input = r#"{% unless page.toc == false %}TOC{% endunless %}"#;
        let output = preprocess_nil_eq_false(input);
        assert!(
            output.contains("page.toc == false and page.toc != nil"),
            "Should add nil guard for == false. Got: {}",
            output
        );
    }

    #[test]
    fn test_preprocess_nil_eq_false_neq_no_rewrite() {
        // Issue 504: != false should NOT be rewritten (the or it introduces
        // breaks precedence in compound conditions).
        let input = r#"{% if page.comments != false %}COMMENTS{% endif %}"#;
        let output = preprocess_nil_eq_false(input);
        assert_eq!(
            output, input,
            "!= false should not be rewritten. Got: {}",
            output
        );
    }

    #[test]
    fn test_nil_eq_false_render_undefined_variable() {
        // When page.toc is undefined (nil), {% unless page.toc == false %} should render
        let eng = engine();
        let mut ctx = Object::new();
        let page = Object::new(); // no toc field
        ctx.insert("page".into(), Value::Object(page));

        let output = eng
            .parse_and_render(
                "{% unless page.toc == false %}TOC_PRESENT{% endunless %}",
                &ctx,
            )
            .unwrap();
        assert_eq!(
            output.trim(),
            "TOC_PRESENT",
            "nil should not equal false, so unless block should render"
        );
    }

    #[test]
    fn test_nil_eq_false_render_true_variable() {
        // When page.toc is explicitly true, {% unless page.toc == false %} should render
        let eng = engine();
        let mut ctx = Object::new();
        let mut page = Object::new();
        page.insert("toc".into(), Value::scalar(true));
        ctx.insert("page".into(), Value::Object(page));

        let output = eng
            .parse_and_render(
                "{% unless page.toc == false %}TOC_PRESENT{% endunless %}",
                &ctx,
            )
            .unwrap();
        assert_eq!(
            output.trim(),
            "TOC_PRESENT",
            "true != false, so unless block should render"
        );
    }

    // ========================================================================
    // Issue 327: Nested braces {% if {{var}} %} preprocessing
    // ========================================================================

    #[test]
    fn test_preprocess_nested_braces_in_if() {
        let input = r#"{% if {{include.url}} %}<a href="{{include.url}}">link</a>{% endif %}"#;
        let output = preprocess_nested_braces(input);
        assert!(
            output.contains("{% if include.url %}"),
            "Nested {{}} inside tag should be unwrapped. Got: {}",
            output
        );
        // The {{include.url}} in the body (outside {% %}) should remain
        assert!(
            output.contains("<a href=\"{{include.url}}\">"),
            "Variable references outside tags should remain. Got: {}",
            output
        );
    }

    #[test]
    fn test_preprocess_nested_braces_preserves_body() {
        // Only {{}} inside {% %} tags should be affected; body {{}} stays
        let input = r#"{% if {{include.show}} %}{{include.content}}{% endif %}"#;
        let output = preprocess_nested_braces(input);
        assert!(
            output.contains("{% if include.show %}"),
            "Nested braces in tag should be unwrapped. Got: {}",
            output
        );
        assert!(
            output.contains("{{include.content}}"),
            "Body variable references should be preserved. Got: {}",
            output
        );
    }

    #[test]
    fn test_preprocess_nested_braces_unless() {
        let input = r#"{% unless {{include.hide}} %}visible{% endunless %}"#;
        let output = preprocess_nested_braces(input);
        assert!(
            output.contains("{% unless include.hide %}"),
            "Should work with unless tag too. Got: {}",
            output
        );
    }

    #[test]
    fn test_preprocess_nested_braces_no_change_when_absent() {
        let input = r#"{% if include.url %}link{% endif %}"#;
        let output = preprocess_nested_braces(input);
        assert_eq!(
            input, output,
            "Should not modify templates without nested braces"
        );
    }

    #[test]
    fn test_nested_braces_end_to_end_render() {
        // End-to-end: template with nested braces should parse and render
        let eng = engine();
        let mut ctx = Object::new();
        let mut include_obj = Object::new();
        include_obj.insert("url".into(), Value::scalar("http://example.com"));
        ctx.insert("include".into(), Value::Object(include_obj));

        let template = r#"{% if {{include.url}} %}<a href="{{include.url}}">link</a>{% endif %}"#;
        let result = eng.parse_and_render(template, &ctx).unwrap();
        assert!(
            result.contains("<a href=\"http://example.com\">link</a>"),
            "Should render the link. Got: {}",
            result
        );
    }

    #[test]
    fn test_nested_braces_end_to_end_falsy() {
        // When the variable is not set, the if block should not render
        let eng = engine();
        let mut ctx = Object::new();
        let include_obj = Object::new(); // no url field
        ctx.insert("include".into(), Value::Object(include_obj));

        let template = r#"{% if {{include.url}} %}<a>link</a>{% endif %}"#;
        let result = eng.parse_and_render(template, &ctx).unwrap();
        assert!(
            !result.contains("<a>link</a>"),
            "Should not render when variable is missing. Got: {}",
            result
        );
    }

    // ========================================================================
    // url_encode and cgi_escape filters (Issue 178)
    // ========================================================================

    #[test]
    fn test_url_encode_spaces_as_plus() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("value".into(), LiquidValue::scalar("foo bar baz"));
        let out = eng
            .parse_and_render("{{ value | url_encode }}", &ctx)
            .unwrap();
        assert_eq!(out, "foo+bar+baz");
    }

    #[test]
    fn test_cgi_escape_spaces_as_plus() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("value".into(), LiquidValue::scalar("foo bar baz"));
        let out = eng
            .parse_and_render("{{ value | cgi_escape }}", &ctx)
            .unwrap();
        assert_eq!(out, "foo+bar+baz");
    }

    #[test]
    fn test_url_encode_special_chars() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert(
            "value".into(),
            LiquidValue::scalar("https://example.com/path?q=1"),
        );
        let out = eng
            .parse_and_render("{{ value | url_encode }}", &ctx)
            .unwrap();
        assert_eq!(out, "https%3A%2F%2Fexample.com%2Fpath%3Fq%3D1");
    }

    #[test]
    fn test_cgi_escape_twitter_share() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert(
            "text".into(),
            LiquidValue::scalar("Creating an AWS Account by @Al_Grigor https://example.com"),
        );
        let out = eng
            .parse_and_render("{{ text | cgi_escape }}", &ctx)
            .unwrap();
        assert_eq!(
            out,
            "Creating+an+AWS+Account+by+%40Al_Grigor+https%3A%2F%2Fexample.com"
        );
    }

    // ========================================================================
    // Issue 185: JSON-LD FAQ/transcript whitespace in markdownify output
    // ========================================================================

    /// FAQ answer: markdownify | strip | jsonify must produce a single-line
    /// JSON string with no trailing whitespace inside the string value.
    #[test]
    fn test_jsonld_faq_answer_no_trailing_space() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("answer".into(), LiquidValue::scalar("There are no fees."));
        let template = r#"{% assign html = answer | markdownify | strip %}{{ html | jsonify }}"#;
        let out = eng.parse_and_render(template, &ctx).unwrap();
        // jsonify wraps in quotes; the value should end with </p>" (no trailing space)
        assert!(
            out.ends_with("</p>\""),
            "FAQ answer should end with </p>\" (no trailing space). Got: {:?}",
            out
        );
        // Should not contain literal newlines (all newlines should be JSON-escaped)
        // The output from jsonify is a JSON string that gets embedded in HTML.
        // Literal newlines would break JSON parsing.
        let inner = &out[1..out.len() - 1]; // strip outer quotes
        assert!(
            !inner.contains('\n'),
            "FAQ answer jsonify output should not contain literal newlines. Got: {:?}",
            out
        );
    }

    /// Multi-paragraph FAQ answer: the jsonify output should produce valid JSON
    /// with escaped newlines, not literal newlines that break JSON parsing.
    #[test]
    fn test_jsonld_faq_answer_multi_paragraph_valid_json() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert(
            "answer".into(),
            LiquidValue::scalar("First paragraph.\n\nSecond paragraph with details."),
        );
        let template =
            r#"{% assign html = answer | markdownify | strip %}"text": {{ html | jsonify }}"#;
        let out = eng.parse_and_render(template, &ctx).unwrap();
        // The output should be on a single line (no literal newlines in JSON string)
        assert!(
            !out.contains('\n') || out.trim().lines().count() == 1,
            "Multi-paragraph FAQ answer should produce single-line JSON. Got: {:?}",
            out
        );
        // Extract the JSON value and verify it's valid
        let json_part = out.trim_start_matches("\"text\": ");
        let parsed: Result<String, _> = serde_json::from_str(json_part);
        assert!(
            parsed.is_ok(),
            "FAQ answer JSON value should be parseable. Got: {:?}",
            json_part
        );
    }

    /// Multiple FAQ answers should all be trimmed consistently.
    #[test]
    fn test_jsonld_faq_multiple_answers_all_trimmed() {
        let eng = engine();
        let mut ctx = Object::new();
        let faqs = LiquidValue::Array(vec![
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("question".into(), LiquidValue::scalar("Q1?"));
                o.insert("answer".into(), LiquidValue::scalar("Answer one."));
                o
            }),
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("question".into(), LiquidValue::scalar("Q2?"));
                o.insert(
                    "answer".into(),
                    LiquidValue::scalar("Answer two.\n\nMore details."),
                );
                o
            }),
        ]);
        ctx.insert("faqs".into(), faqs);
        let template = r#"{% for faq in faqs %}{% assign html = faq.answer | markdownify | strip %}{{ html | jsonify }}
{% endfor %}"#;
        let out = eng.parse_and_render(template, &ctx).unwrap();
        for (i, line) in out.trim().lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            // Each jsonify output should be a valid JSON string
            assert!(
                line.starts_with('"') && line.ends_with('"'),
                "FAQ answer {} should be a JSON string. Got: {:?}",
                i,
                line
            );
            let inner = &line[1..line.len() - 1];
            assert!(
                !inner.contains('\n'),
                "FAQ answer {} should not contain literal newlines. Got: {:?}",
                i,
                line
            );
        }
    }

    /// Author description: content | strip_html | strip_newlines should have
    /// no trailing newline characters.
    #[test]
    fn test_jsonld_author_description_no_trailing_newline() {
        let eng = engine();
        let mut ctx = Object::new();
        // Simulate what happens with author content that has a trailing newline
        ctx.insert(
            "content".into(),
            LiquidValue::scalar("<p>John is a developer at DataTalks.Club</p>\n"),
        );
        let template =
            r#"{% assign desc = content | strip_html | strip_newlines %}{{ desc | jsonify }}"#;
        let out = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(
            out, "\"John is a developer at DataTalks.Club\"",
            "Author description should have no trailing newline. Got: {:?}",
            out
        );
    }

    /// Author description with markdown links: should be rendered to plain text.
    /// The template uses `content | strip_html | strip_newlines` where content
    /// is the rendered HTML of the author's markdown. Markdown links in the
    /// source become <a> tags in HTML, which strip_html removes leaving just text.
    #[test]
    fn test_jsonld_author_description_markdown_links_stripped() {
        let eng = engine();
        let mut ctx = Object::new();
        // Content is already rendered HTML (markdownify was applied during collection processing)
        ctx.insert(
            "content".into(),
            LiquidValue::scalar("<p>Founded <a href=\"https://example.com\">Company</a></p>\n"),
        );
        let template =
            r#"{% assign desc = content | strip_html | strip_newlines %}{{ desc | jsonify }}"#;
        let out = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(
            out, "\"Founded Company\"",
            "Markdown links should be stripped to plain text. Got: {:?}",
            out
        );
    }

    /// Issue 296: Podcast JSON-LD uses `content | strip_html | jsonify` (without
    /// strip_newlines). Jekyll's strip_html does NOT remove trailing newlines, so
    /// the trailing `\n` from rendered HTML (e.g. `<p>text</p>\n`) must survive
    /// through the pipeline into the JSON string, producing `"text\n"`.
    #[test]
    fn test_issue296_podcast_description_preserves_trailing_newline() {
        let eng = engine();
        let mut ctx = Object::new();
        // Simulate guest.content = rendered HTML with trailing newline
        ctx.insert(
            "content".into(),
            LiquidValue::scalar("<p>Born in Argentina, passionate about mentoring.</p>\n"),
        );
        // Podcast template uses strip_html | jsonify (no strip_newlines)
        let template = r#"{{ content | strip_html | jsonify }}"#;
        let out = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(
            out, "\"Born in Argentina, passionate about mentoring.\\n\"",
            "Podcast description should preserve trailing newline. Got: {:?}",
            out
        );
    }

    /// Issue 296: Multi-paragraph content should preserve all newlines including
    /// the trailing one, matching Jekyll's `content | strip_html | jsonify`.
    #[test]
    fn test_issue296_multi_paragraph_description_trailing_newline() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert(
            "content".into(),
            LiquidValue::scalar(
                "<p>First paragraph about the guest.</p>\n\n<p>Second paragraph about mentoring.</p>\n",
            ),
        );
        let template = r#"{{ content | strip_html | jsonify }}"#;
        let out = eng.parse_and_render(template, &ctx).unwrap();
        // The output should end with \n" (escaped newline then closing quote)
        assert!(
            out.ends_with("\\n\""),
            "Multi-paragraph description should end with trailing newline in JSON. Got: {:?}",
            out
        );
    }

    /// Simulate the full rendering pipeline for a markdown blog post that
    /// includes a FAQ accordion with JSON-LD. The post content goes through:
    /// 1. Liquid template rendering (expanding includes, running markdownify)
    /// 2. markdown_to_html (processing the markdown post body)
    /// The JSON-LD inside <script> tags must survive step 2 intact.
    #[test]
    fn test_jsonld_faq_survives_markdown_pipeline() {
        let eng = engine();
        let mut ctx = Object::new();
        // Simulate FAQ data with a multi-paragraph answer
        let faqs = LiquidValue::Array(vec![LiquidValue::Object({
            let mut o = Object::new();
            o.insert("question".into(), LiquidValue::scalar("What is this?"));
            o.insert(
                "answer".into(),
                LiquidValue::scalar("First paragraph.\n\nSecond paragraph."),
            );
            o
        })]);
        ctx.insert("faqs".into(), faqs);

        // Simulate the FAQ accordion template inline (without include)
        let template = r#"Some markdown text.

<script type="application/ld+json">
{
  "@type": "FAQPage",
  "mainEntity": [
    {% for faq in faqs %}
    {% assign answer_html = faq.answer | markdownify | strip %}
    {
      "@type": "Question",
      "name": {{ faq.question | jsonify }},
      "acceptedAnswer": {
        "@type": "Answer",
        "text": {{ answer_html | jsonify }}
      }
    }{% unless forloop.last %},{% endunless %}
    {% endfor %}
  ]
}
</script>"#;

        // Step 1: Liquid rendering
        let liquid_output = eng.parse_and_render(template, &ctx).unwrap();

        // Step 2: markdown_to_html (simulating the markdown page pipeline)
        let html_output = crate::frontmatter::markdown_to_html(&liquid_output);

        // The JSON-LD text value should be valid JSON
        // Find the "text": value
        let text_idx = html_output
            .find("\"text\":")
            .expect("should have text field");
        let rest = &html_output[text_idx..];
        // Find the JSON string value
        let colon_idx = rest.find(':').unwrap();
        let value_start = rest[colon_idx + 1..].trim_start();
        // Extract the JSON string (starts with " ends with ")
        assert!(
            value_start.starts_with('"'),
            "JSON text value should start with quote. Got: {:?}",
            &value_start[..50.min(value_start.len())]
        );
        // Find the matching close quote
        let inner_start = 1; // skip opening quote
        let mut i = inner_start;
        let bytes = value_start.as_bytes();
        while i < bytes.len() {
            if bytes[i] == b'\\' {
                i += 2; // skip escape sequence
            } else if bytes[i] == b'"' {
                break;
            } else {
                i += 1;
            }
        }
        let json_value = &value_start[..i + 1];
        // The JSON value should not contain literal newlines
        let inner = &json_value[1..json_value.len() - 1];
        assert!(
            !inner.contains('\n'),
            "JSON-LD text value should not contain literal newlines after markdown pipeline. Got: {:?}",
            json_value
        );
    }

    /// Regression: markdownify in regular (non-JSON-LD) templates should not
    /// have its output changed -- trailing newline is expected.
    #[test]
    fn test_markdownify_filter_in_template_unchanged() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("text".into(), LiquidValue::scalar("hello"));
        let out = eng
            .parse_and_render("{{ text | markdownify }}", &ctx)
            .unwrap();
        assert_eq!(
            out, "<p>hello</p>\n",
            "markdownify in regular templates should preserve trailing newline"
        );
    }

    // ========================================================================
    // Issue 202: Special characters and trailing whitespace in descriptions
    // ========================================================================

    /// When content has trailing whitespace before a newline (soft break),
    /// the strip_html | strip_newlines pipeline should preserve the space.
    /// This matches kramdown behavior where "with a \n$500" becomes
    /// "with a $500" after strip_newlines (space preserved).
    #[test]
    fn test_issue202_strip_pipeline_preserves_trailing_space() {
        let eng = engine();
        let mut ctx = Object::new();
        // Simulate HTML content from kramdown-compatible markdown rendering
        // Source: "with a \n$500,000 grand prize"
        // HTML: "<p>with a \n$500,000 grand prize</p>\n"
        ctx.insert(
            "content".into(),
            LiquidValue::scalar("<p>with a \n$500,000 grand prize</p>\n"),
        );
        let template =
            r#"{% assign desc = content | strip_html | strip_newlines %}{{ desc | jsonify }}"#;
        let out = eng.parse_and_render(template, &ctx).unwrap();
        assert!(
            out.contains("with a $500,000"),
            "Space before $ must be preserved after strip_newlines. Got: {:?}",
            out
        );
    }

    /// The truncate: 200 filter should work correctly with special characters
    /// like $ in the content.
    #[test]
    fn test_issue202_truncate_preserves_dollar_sign() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("text".into(), LiquidValue::scalar("won a $500,000 prize"));
        let out = eng
            .parse_and_render("{{ text | truncate: 200 }}", &ctx)
            .unwrap();
        assert_eq!(
            out, "won a $500,000 prize",
            "Dollar sign must not be stripped. Got: {:?}",
            out
        );
    }

    // ========================================================================
    // Issue 196: shift filter, feed_meta tag, github_edit_link tag
    // ========================================================================

    /// The `shift` filter removes the first element from an array.
    /// Used by jekyll-toc.html in opensource-guide (337 pages affected).
    #[test]
    fn test_shift_filter_removes_first_element() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert(
            "items".into(),
            LiquidValue::Array(vec![
                LiquidValue::scalar("a"),
                LiquidValue::scalar("b"),
                LiquidValue::scalar("c"),
            ]),
        );
        let out = eng
            .parse_and_render("{{ items | shift | join: ',' }}", &ctx)
            .unwrap();
        assert_eq!(out, "b,c", "shift should remove first element");
    }

    /// The `shift` filter on an empty array should return an empty array.
    #[test]
    fn test_shift_filter_empty_array() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("items".into(), LiquidValue::Array(vec![]));
        let out = eng
            .parse_and_render("{{ items | shift | size }}", &ctx)
            .unwrap();
        assert_eq!(out, "0", "shift of empty array should be empty");
    }

    /// The `{% feed_meta %}` tag should parse and render without error.
    /// Used by jekyll-feed plugin in jekyll-docs (109 pages affected).
    #[test]
    fn test_feed_meta_tag_renders() {
        let eng = engine();
        let ctx = Object::new();
        let result = eng.parse_and_render("before{% feed_meta %}after", &ctx);
        assert!(
            result.is_ok(),
            "feed_meta tag should parse and render: {:?}",
            result.err()
        );
        let out = result.unwrap();
        assert!(out.contains("before"), "content before tag preserved");
        assert!(out.contains("after"), "content after tag preserved");
    }

    /// The `{% github_edit_link %}` tag should parse and render without error.
    /// Used by jekyll-github-metadata in choosealicense.com (55 pages affected).
    #[test]
    fn test_github_edit_link_tag_renders() {
        let eng = engine();
        let ctx = Object::new();
        let result = eng.parse_and_render(
            "before{% github_edit_link \"Improve this page\" %}after",
            &ctx,
        );
        assert!(
            result.is_ok(),
            "github_edit_link tag should parse and render: {:?}",
            result.err()
        );
        let out = result.unwrap();
        assert!(out.contains("before"), "content before tag preserved");
        assert!(out.contains("after"), "content after tag preserved");
    }

    /// Accessing an out-of-bounds array index should return nil (empty string),
    /// not error. This matches Ruby Liquid behavior and is critical for
    /// templates like jekyll-toc.html that do `array[1]` after split.
    #[test]
    fn test_out_of_bounds_array_index_returns_nil() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert(
            "items".into(),
            LiquidValue::Array(vec![LiquidValue::scalar("only")]),
        );
        // Accessing index 1 on a 1-element array should return empty string (nil)
        let result = eng.parse_and_render("{{ items[1] }}", &ctx);
        assert!(
            result.is_ok(),
            "Out-of-bounds array index should not error: {:?}",
            result.err()
        );
        let out = result.unwrap();
        assert_eq!(out, "", "Out-of-bounds index should render as empty");
    }

    /// Variable path through an out-of-bounds index should return nil.
    /// This is used by jekyll-toc: `{% assign x = workspace[0] | split: 'class="' %}`
    /// followed by `{% assign y = x[1] %}` where x has only 1 element.
    #[test]
    fn test_split_then_out_of_bounds_index_returns_nil() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("text".into(), LiquidValue::scalar("no class here"));
        // Split by something not in the string -> single-element array
        // Then access [1] -> should be nil/empty
        let result = eng.parse_and_render(
            r#"{% assign parts = text | split: 'class="' %}{{ parts[1] }}"#,
            &ctx,
        );
        assert!(
            result.is_ok(),
            "Split then out-of-bounds index should not error: {:?}",
            result.err()
        );
        let out = result.unwrap();
        assert_eq!(out, "", "parts[1] should be empty when split has 1 result");
    }

    // ========================================================================
    // Issue 209: Link tag no .html for collection docs
    // ========================================================================

    /// Helper: build a LinkTagConfig with a page permalink style and no collection suffixes.
    fn link_config_page(style: &str) -> crate::collection::LinkTagConfig {
        crate::collection::LinkTagConfig {
            page_permalink_style: style.to_string(),
            collection_suffixes: None,
        }
    }

    /// Helper: build a LinkTagConfig with collection suffix map.
    fn link_config_collection(suffixes: &[(&str, &str)]) -> crate::collection::LinkTagConfig {
        let mut map = std::collections::HashMap::new();
        for &(name, suffix) in suffixes {
            map.insert(name.to_string(), suffix.to_string());
        }
        crate::collection::LinkTagConfig {
            page_permalink_style: String::new(),
            collection_suffixes: Some(map),
        }
    }

    #[test]
    fn test_link_tag_no_html_for_collection_pages() {
        // Collection docs (path starts with _) -> extensionless URL
        let cfg = crate::collection::LinkTagConfig::default();
        let result = preprocess_jekyll_tags_with_config(
            r#"<a href="{% link _pages/banners.md %}">Link</a>"#,
            &cfg,
        );
        assert_eq!(result, r#"<a href="/pages/banners">Link</a>"#);
    }

    #[test]
    fn test_link_tag_root_page_keeps_html() {
        let cfg = link_config_page("date");
        let result = preprocess_jekyll_tags_with_config(r#"{% link about.md %}"#, &cfg);
        assert_eq!(result, "/about.html");
    }

    #[test]
    fn test_link_tag_unicode_collection_doc() {
        let cfg = crate::collection::LinkTagConfig::default();
        let result = preprocess_jekyll_tags_with_config(r#"{% link _pages/uber-uns.md %}"#, &cfg);
        assert_eq!(result, "/pages/uber-uns");
    }

    #[test]
    fn test_link_tag_nested_collection_doc() {
        let cfg = crate::collection::LinkTagConfig::default();
        let result =
            preprocess_jekyll_tags_with_config(r#"{% link _notes/2018/my-note.md %}"#, &cfg);
        assert_eq!(result, "/notes/2018/my-note");
    }

    #[test]
    fn test_link_tag_posts_uses_permalink_pattern() {
        // NOTE: post permalink pattern is a separate global (frontmatter module),
        // not affected by LinkTagConfig. This test still uses the global for now.
        crate::frontmatter::set_post_permalink_pattern("/posts/:title");
        let cfg = crate::collection::LinkTagConfig::default();
        let result = preprocess_jekyll_tags_with_config(
            r#"{% link _posts/2020-06-06-reparations.md %}"#,
            &cfg,
        );
        assert_eq!(result, "/posts/reparations");

        let result2 = preprocess_jekyll_tags_with_config(
            r#"{% link _posts/2024-11-02-javascript.md %}"#,
            &cfg,
        );
        assert_eq!(result2, "/posts/javascript");

        crate::frontmatter::set_post_permalink_pattern(
            "/:categories/:year/:month/:day/:title.html",
        );
        let result3 = preprocess_jekyll_tags_with_config(
            r#"{% link _posts/2024-11-02-javascript.md %}"#,
            &cfg,
        );
        assert_eq!(result3, "/2024/11/02/javascript.html");

        crate::frontmatter::set_post_permalink_pattern(
            "/:categories/:year/:month/:day/:title.html",
        );
    }

    // ========================================================================
    // Issue 216: link tag resolves .html collection documents
    // ========================================================================

    #[test]
    fn test_link_tag_html_collection_doc_no_extension() {
        let cfg = crate::collection::LinkTagConfig::default();
        let result = preprocess_jekyll_tags_with_config(
            r#"<a href="{% link _pages/issues.html %}">Issues</a>"#,
            &cfg,
        );
        assert_eq!(result, r#"<a href="/pages/issues">Issues</a>"#);
    }

    #[test]
    fn test_link_tag_html_root_page_keeps_extension() {
        let cfg = link_config_page("date");
        let result = preprocess_jekyll_tags_with_config(r#"{% link about.html %}"#, &cfg);
        assert_eq!(result, "/about.html");
    }

    #[test]
    fn test_link_tag_html_collection_unicode() {
        let cfg = crate::collection::LinkTagConfig::default();
        let result =
            preprocess_jekyll_tags_with_config("{% link _pages/\u{00fc}ber-uns.html %}", &cfg);
        assert_eq!(result, "/pages/\u{00fc}ber-uns");
    }

    // ========================================================================
    // Issue 502: link tag respects permalink: pretty
    // ========================================================================

    #[test]
    fn test_link_tag_pretty_permalink_md_page() {
        let cfg = link_config_page("pretty");
        let result =
            preprocess_jekyll_tags_with_config(r#"{% link docs/configuration.md %}"#, &cfg);
        assert_eq!(result, "/docs/configuration/");
    }

    #[test]
    fn test_link_tag_pretty_permalink_md_root() {
        let cfg = link_config_page("pretty");
        let result = preprocess_jekyll_tags_with_config(r#"{% link about.md %}"#, &cfg);
        assert_eq!(result, "/about/");
    }

    #[test]
    fn test_link_tag_pretty_permalink_html_page() {
        let cfg = link_config_page("pretty");
        let result = preprocess_jekyll_tags_with_config(r#"{% link CHANGELOG.html %}"#, &cfg);
        assert_eq!(result, "/CHANGELOG/");
    }

    #[test]
    fn test_link_tag_pretty_permalink_with_anchor() {
        let cfg = link_config_page("pretty");
        let result = preprocess_jekyll_tags_with_config(
            r#"<a href="{% link docs/configuration.md %}#aux-links">x</a>"#,
            &cfg,
        );
        assert_eq!(result, r#"<a href="/docs/configuration/#aux-links">x</a>"#);
    }

    #[test]
    fn test_link_tag_default_permalink_keeps_html() {
        let cfg = link_config_page("date");
        let result =
            preprocess_jekyll_tags_with_config(r#"{% link docs/configuration.md %}"#, &cfg);
        assert_eq!(result, "/docs/configuration.html");
    }

    #[test]
    fn test_link_tag_collection_unaffected_by_pretty() {
        let cfg = link_config_page("pretty");
        let result = preprocess_jekyll_tags_with_config(r#"{% link _pages/banners.md %}"#, &cfg);
        assert_eq!(result, "/pages/banners");
    }

    #[test]
    fn test_link_tag_pretty_permalink_unicode_page() {
        let cfg = link_config_page("pretty");
        let result = preprocess_jekyll_tags_with_config(
            "{% link \u{0447}\u{0430}\u{0441}\u{0442}\u{044c}.md %}",
            &cfg,
        );
        assert_eq!(result, "/\u{0447}\u{0430}\u{0441}\u{0442}\u{044c}/");
    }

    // ========================================================================
    // Issue 527: collection link tags respect collection permalink trailing slash
    // ========================================================================

    #[test]
    fn test_link_tag_collection_with_trailing_slash_permalink() {
        let cfg = link_config_collection(&[("docs", "/")]);
        let result = preprocess_jekyll_tags_with_config(r#"{% link _docs/variables.md %}"#, &cfg);
        assert_eq!(result, "/docs/variables/");
    }

    #[test]
    fn test_link_tag_collection_without_trailing_slash_permalink() {
        let cfg = link_config_collection(&[("docs", "")]);
        let result = preprocess_jekyll_tags_with_config(r#"{% link _docs/variables.md %}"#, &cfg);
        assert_eq!(result, "/docs/variables");
    }

    #[test]
    fn test_link_tag_collection_index_becomes_directory() {
        let cfg = link_config_collection(&[("docs", "/")]);
        let result = preprocess_jekyll_tags_with_config(r#"{% link _docs/index.md %}"#, &cfg);
        assert_eq!(result, "/docs/");
    }

    #[test]
    fn test_link_tag_collection_trailing_slash_html_extension() {
        let cfg = link_config_collection(&[("docs", "/")]);
        let result = preprocess_jekyll_tags_with_config(r#"{% link _docs/datafiles.html %}"#, &cfg);
        assert_eq!(result, "/docs/datafiles/");
    }

    #[test]
    fn test_link_tag_collection_no_config_falls_back_to_extensionless() {
        let cfg = crate::collection::LinkTagConfig::default();
        let result = preprocess_jekyll_tags_with_config(r#"{% link _pages/banners.md %}"#, &cfg);
        assert_eq!(result, "/pages/banners");
    }

    #[test]
    fn test_link_tag_collection_unicode_with_trailing_slash() {
        let cfg = link_config_collection(&[("docs", "/")]);
        let result = preprocess_jekyll_tags_with_config(
            "{% link _docs/\u{0443}\u{0441}\u{0442}\u{0430}\u{043d}\u{043e}\u{0432}\u{043a}\u{0430}.md %}",
            &cfg,
        );
        assert_eq!(
            result,
            "/docs/\u{0443}\u{0441}\u{0442}\u{0430}\u{043d}\u{043e}\u{0432}\u{043a}\u{0430}/"
        );
    }

    // ========================================================================
    // Issue 209/233: map filter preserves nested arrays (does NOT auto-flatten)
    // ========================================================================

    #[test]
    fn test_map_filter_preserves_nested_arrays() {
        let eng = engine();
        let mut ctx = Object::new();

        // Create items with tags arrays (including Unicode)
        let items = LiquidValue::Array(vec![
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert(
                    "tags".into(),
                    LiquidValue::Array(vec![
                        LiquidValue::scalar("Book"),
                        LiquidValue::scalar("Mental health"),
                    ]),
                );
                o
            }),
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert(
                    "tags".into(),
                    LiquidValue::Array(vec![
                        LiquidValue::scalar("Hobby"),
                        LiquidValue::scalar("Life"),
                    ]),
                );
                o
            }),
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert(
                    "tags".into(),
                    LiquidValue::Array(vec![
                        LiquidValue::scalar("Book"),
                        LiquidValue::scalar("Gesundheit"),
                    ]),
                );
                o
            }),
        ]);
        ctx.insert("items".into(), items);

        // In Jekyll/Ruby, map does NOT auto-flatten. Result is array of 3 sub-arrays.
        let template = r#"{% assign all_tags = items | map: "tags" %}{{ all_tags | size }}"#;
        let result = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(result, "3");
    }

    #[test]
    fn test_map_filter_flat_property_unchanged() {
        let eng = engine();
        let mut ctx = Object::new();

        // Items with scalar title property (including Unicode)
        let items = LiquidValue::Array(vec![
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("title".into(), LiquidValue::scalar("Первый пост"));
                o
            }),
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("title".into(), LiquidValue::scalar("Second"));
                o
            }),
        ]);
        ctx.insert("items".into(), items);

        let template = r#"{% assign titles = items | map: "title" %}{% for t in titles %}{{ t }};{% endfor %}"#;
        let result = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(result, "Первый пост;Second;");
    }

    // Issue 233: map filter must NOT flatten nested arrays.
    // In Jekyll, `map` preserves nested arrays. The `group_by | map: "items" | first`
    // pattern used by just-the-docs relies on this.
    #[test]
    fn test_map_filter_preserves_nested_arrays_for_group_by() {
        let eng = engine();
        let mut ctx = Object::new();

        // Simulate group_by output: [{name: "", items: [a, b]}, {name: "X", items: [c]}]
        let groups = LiquidValue::Array(vec![
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("name".into(), LiquidValue::scalar(""));
                o.insert(
                    "items".into(),
                    LiquidValue::Array(vec![
                        LiquidValue::scalar("alpha"),
                        LiquidValue::scalar("beta"),
                    ]),
                );
                o
            }),
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("name".into(), LiquidValue::scalar("X"));
                o.insert(
                    "items".into(),
                    LiquidValue::Array(vec![LiquidValue::scalar("gamma")]),
                );
                o
            }),
        ]);
        ctx.insert("groups".into(), groups);

        // map: "items" should preserve the arrays, so first gives ["alpha", "beta"]
        let template = r#"{% assign mapped = groups | map: "items" %}{{ mapped | size }}:{{ mapped | first | size }}"#;
        let result = eng.parse_and_render(template, &ctx).unwrap();
        // mapped should be [["alpha","beta"],["gamma"]], size=2
        // first element is ["alpha","beta"], size=2
        assert_eq!(result, "2:2");
    }

    // Issue 233: group_by_exp must access template-assigned variables in expression
    #[test]
    fn test_group_by_exp_with_jsonify_and_assigned_var() {
        let eng = engine();
        let mut ctx = Object::new();

        // Simulate pages with numeric nav_order
        let pages = LiquidValue::Array(vec![
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("title".into(), LiquidValue::scalar("Home"));
                o.insert("nav_order".into(), LiquidValue::scalar(1i64));
                o
            }),
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("title".into(), LiquidValue::scalar("About"));
                o.insert("nav_order".into(), LiquidValue::scalar("second"));
                o
            }),
        ]);
        ctx.insert("pages".into(), pages);

        // The just-the-docs pattern:
        // assign double_quote, then group_by_exp using jsonify/slice/remove/size
        // Numbers: jsonify="1", slice:0="1", remove:dq="1", size=1
        // Strings: jsonify='"second"', slice:0='"', remove:dq="", size=0
        let template = r#"{% assign double_quote = '"' %}{% assign groups = pages | group_by_exp: "item", "item.nav_order | jsonify | slice: 0 | remove: double_quote | size" %}{% for g in groups %}[{{ g.name }}:{{ g.items | size }}]{% endfor %}"#;
        let result = eng.parse_and_render(template, &ctx).unwrap();
        // Should have two groups: "1" (number, size 1) and "0" (string, size 1)
        assert!(
            result.contains("[1:1]") && result.contains("[0:1]"),
            "Expected groups [1:1] and [0:1], got: {}",
            result
        );
    }

    // ── Issue 197: String literal as default filter shorthand ──

    #[test]
    fn test_pipe_to_string_literal_acts_as_default_filter() {
        let eng = engine();
        // When x is nil, the string literal acts as default
        let ctx = Object::new();
        let out = eng
            .parse_and_render(r#"{% assign x = nil %}{{ x | "fallback" }}"#, &ctx)
            .unwrap();
        assert_eq!(out, "fallback");

        // When x has a value, it passes through
        let mut ctx2 = Object::new();
        ctx2.insert("x".into(), LiquidValue::scalar("hello"));
        let out2 = eng
            .parse_and_render(r#"{{ x | "fallback" }}"#, &ctx2)
            .unwrap();
        assert_eq!(out2, "hello");
    }

    #[test]
    fn test_pipe_to_string_literal_with_unicode_default() {
        let eng = engine();
        // Czech text with diacritics
        let ctx = Object::new();
        let out = eng
            .parse_and_render(
                "{% assign x = nil %}{{ x | \"Parametr neexistuje\" }}",
                &ctx,
            )
            .unwrap();
        assert_eq!(out, "Parametr neexistuje");

        // When x has a value, it passes through
        let mut ctx2 = Object::new();
        ctx2.insert("x".into(), LiquidValue::scalar("existuje"));
        let out2 = eng
            .parse_and_render("{{ x | \"Parametr neexistuje\" }}", &ctx2)
            .unwrap();
        assert_eq!(out2, "existuje");
    }

    // ── Issue 197: Parenthesized conditions in if blocks (integration) ──

    #[test]
    fn test_if_with_parenthesized_comparison_integration() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("a".into(), LiquidValue::scalar(true));
        ctx.insert("b".into(), LiquidValue::scalar("hello"));
        ctx.insert("c".into(), LiquidValue::scalar("world"));
        let out = eng
            .parse_and_render(r#"{% if a and (b != c) %}yes{% else %}no{% endif %}"#, &ctx)
            .unwrap();
        assert_eq!(out, "yes");
    }

    // ── Issue 197: For loop over scalar (integration) ──

    #[test]
    fn test_for_loop_over_string_integration() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("val".into(), LiquidValue::scalar("hello"));
        let out = eng
            .parse_and_render("{% for item in val %}[{{ item }}]{% endfor %}", &ctx)
            .unwrap();
        assert_eq!(out, "[hello]");
    }

    // ── Issue 197: endraw after endhighlight ──

    #[test]
    fn test_endraw_after_endhighlight_same_line() {
        let eng = engine();
        let ctx = Object::new();
        let template = concat!(
            "{% highlight yaml %}",
            "{% raw %}{% highlight some_language %}\n",
            "Some code\n",
            "{% endhighlight %}{% endraw %}",
            "{% endhighlight %}",
        );
        let result = eng.parse_and_render(template, &ctx);
        assert!(
            result.is_ok(),
            "Template with endraw after endhighlight should parse: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_endraw_after_endhighlight_with_unicode_content() {
        let eng = engine();
        let ctx = Object::new();
        // French: "quelque chose en fran\u{00E7}ais"
        let template = concat!(
            "{% highlight yaml %}",
            "{% raw %}quelque chose en fran\u{00E7}ais{% endraw %}",
            "{% endhighlight %}",
        );
        let result = eng.parse_and_render(template, &ctx);
        assert!(
            result.is_ok(),
            "Template with unicode in raw should parse: {:?}",
            result.err()
        );
        let output = result.unwrap();
        assert!(
            output.contains("fran\u{00E7}ais"),
            "Unicode content should be preserved, got: {}",
            output
        );
    }

    // ========================================================================
    // Unknown tag graceful handling (issue #215)
    // ========================================================================

    #[test]
    fn test_unknown_tag_empty_output() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng.parse_and_render("{% nonexistent_tag %}", &ctx).unwrap();
        assert_eq!(out, "", "unknown tag should produce empty output");
    }

    #[test]
    fn test_unknown_tag_with_positional_args() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render("{% nonexistent_tag arg1 arg2 %}", &ctx)
            .unwrap();
        assert_eq!(out, "", "unknown tag with args should produce empty output");
    }

    #[test]
    fn test_unknown_tag_with_key_value_args() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render(r#"{% nonexistent_tag key:value class:"something" %}"#, &ctx)
            .unwrap();
        assert_eq!(
            out, "",
            "unknown tag with key:value args should produce empty output"
        );
    }

    #[test]
    fn test_unknown_tag_surrounding_content_preserved() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render("before{% nonexistent_tag %}after", &ctx)
            .unwrap();
        assert_eq!(out, "beforeafter");
    }

    #[test]
    fn test_unknown_tag_surrounding_html_preserved() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render("<p>Hello</p>{% unknown_tag %}<p>World</p>", &ctx)
            .unwrap();
        assert_eq!(out, "<p>Hello</p><p>World</p>");
    }

    #[test]
    fn test_unknown_tag_surrounding_expressions_preserved() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render(r#"{{ "hello" }}{% unknown_tag %}{{ "world" }}"#, &ctx)
            .unwrap();
        assert_eq!(out, "helloworld");
    }

    #[test]
    fn test_unknown_tag_unicode_content_preserved() {
        let eng = engine();
        let ctx = Object::new();
        // German and French accented characters
        let out = eng
            .parse_and_render(
                "<p>Z\u{00fc}rich \u{00dc}bersicht</p>{% unknown_tag %}<p>caf\u{00e9} r\u{00e9}sum\u{00e9}</p>",
                &ctx,
            )
            .unwrap();
        assert!(
            out.contains("Z\u{00fc}rich"),
            "German chars preserved: {}",
            out
        );
        assert!(
            out.contains("caf\u{00e9}"),
            "French chars preserved: {}",
            out
        );
    }

    #[test]
    fn test_unknown_tag_cjk_content_preserved() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render("{% unknown_tag %}<p>\u{6771}\u{4eac}\u{90fd}</p>", &ctx)
            .unwrap();
        assert!(
            out.contains("\u{6771}\u{4eac}\u{90fd}"),
            "CJK characters preserved: {}",
            out
        );
    }

    #[test]
    fn test_multiple_unknown_tags() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render("{% foo %}middle{% bar %}", &ctx)
            .unwrap();
        assert_eq!(out, "middle");
    }

    #[test]
    fn test_multiple_unknown_tags_empty() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render("{% foo %}{% bar %}{% baz %}", &ctx)
            .unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn test_mixed_known_and_unknown_tags_if() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render("{% if true %}yes{% endif %}{% unknown_tag %}done", &ctx)
            .unwrap();
        assert_eq!(out, "yesdone");
    }

    #[test]
    fn test_mixed_known_and_unknown_tags_assign() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render(
                r#"{% assign x = "hi" %}{{ x }}{% unknown_tag %}{{ x }}"#,
                &ctx,
            )
            .unwrap();
        assert_eq!(out, "hihi");
    }

    #[test]
    fn test_octicon_tag_like_government_github() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render(
                r#"<a>{% octicon mark-github height:24 class:"fill-gray-light" aria-label:github %}</a>"#,
                &ctx,
            )
            .unwrap();
        assert!(
            out.starts_with("<a><svg"),
            "octicon should render SVG inside anchor: {}",
            out
        );
        assert!(
            out.ends_with("</svg></a>"),
            "SVG should close before anchor: {}",
            out
        );
        assert!(
            out.contains("octicon-mark-github"),
            "Should have octicon class: {}",
            out
        );
    }

    // ========================================================================
    // Issue 250: Include files must NOT have void elements normalized
    // ========================================================================

    #[test]
    fn test_load_includes_preserves_bare_hr() {
        let dir = tempfile::tempdir().unwrap();
        let includes_dir = dir.path();
        std::fs::write(
            includes_dir.join("footer.html"),
            "<hr>\n<footer>Footer</footer>",
        )
        .unwrap();
        let includes = load_includes(includes_dir).unwrap();
        let content = includes.get("footer.html").unwrap();
        assert!(
            content.contains("<hr>"),
            "Include loading must preserve bare <hr>, got: {}",
            content
        );
        assert!(
            !content.contains("<hr />"),
            "Include loading must NOT convert <hr> to <hr />, got: {}",
            content
        );
    }

    #[test]
    fn test_load_includes_preserves_bare_br() {
        let dir = tempfile::tempdir().unwrap();
        let includes_dir = dir.path();
        std::fs::write(includes_dir.join("component.html"), "<p>line1<br>line2</p>").unwrap();
        let includes = load_includes(includes_dir).unwrap();
        let content = includes.get("component.html").unwrap();
        assert!(
            content.contains("<br>"),
            "Include loading must preserve bare <br>, got: {}",
            content
        );
        assert!(
            !content.contains("<br />"),
            "Include loading must NOT convert <br> to <br />, got: {}",
            content
        );
    }

    #[test]
    fn test_load_includes_preserves_mixed_html_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let includes_dir = dir.path();
        let original = "<hr>\n<meta charset=\"utf-8\">\n<div>content</div>\n<br>";
        std::fs::write(includes_dir.join("mixed.html"), original).unwrap();
        let includes = load_includes(includes_dir).unwrap();
        let content = includes.get("mixed.html").unwrap();
        assert_eq!(
            content, original,
            "Include files should pass through without any modification"
        );
    }

    // ========================================================================
    // Uniq/sort/compact must flatten nested arrays (Ruby InputIterator compat)
    // ========================================================================

    #[test]
    fn test_uniq_flattens_nested_arrays_from_map() {
        // In Jekyll: notes | map: "tags" | uniq | sort
        // map returns array-of-arrays; uniq must flatten one level first,
        // matching Ruby Liquid's InputIterator which calls .flatten on inputs.
        let eng = engine();
        let mut ctx = Object::new();

        let items = LiquidValue::Array(vec![
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert(
                    "tags".into(),
                    LiquidValue::Array(vec![
                        LiquidValue::scalar("Book"),
                        LiquidValue::scalar("Mental health"),
                    ]),
                );
                o
            }),
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert(
                    "tags".into(),
                    LiquidValue::Array(vec![
                        LiquidValue::scalar("Life"),
                        LiquidValue::scalar("Book"),
                    ]),
                );
                o
            }),
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("tags".into(), LiquidValue::Array(vec![]));
                o
            }),
        ]);
        ctx.insert("items".into(), items);

        // map: "tags" -> [["Book","Mental health"],["Life","Book"],[]]
        // uniq should flatten -> ["Book","Mental health","Life","Book"] -> dedup -> ["Book","Mental health","Life"]
        // sort -> ["Book","Life","Mental health"]
        let template = r#"{% assign tags = items | map: "tags" | uniq | sort %}{% for t in tags %}[{{ t }}]{% endfor %}"#;
        let result = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(result, "[Book][Life][Mental health]");
    }

    #[test]
    fn test_sort_flattens_nested_arrays() {
        let eng = engine();
        let mut ctx = Object::new();

        let items = LiquidValue::Array(vec![
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert(
                    "tags".into(),
                    LiquidValue::Array(vec![
                        LiquidValue::scalar("Zebra"),
                        LiquidValue::scalar("Apple"),
                    ]),
                );
                o
            }),
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert(
                    "tags".into(),
                    LiquidValue::Array(vec![LiquidValue::scalar("Mango")]),
                );
                o
            }),
        ]);
        ctx.insert("items".into(), items);

        // map: "tags" | sort should flatten then sort
        let template = r#"{% assign tags = items | map: "tags" | sort %}{% for t in tags %}[{{ t }}]{% endfor %}"#;
        let result = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(result, "[Apple][Mango][Zebra]");
    }

    #[test]
    fn test_compact_flattens_nested_arrays() {
        let eng = engine();
        let mut ctx = Object::new();

        let items = LiquidValue::Array(vec![
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert(
                    "tags".into(),
                    LiquidValue::Array(vec![LiquidValue::scalar("A"), LiquidValue::scalar("B")]),
                );
                o
            }),
            LiquidValue::Object({
                let o = Object::new();
                // No tags key at all -> map returns Nil
                o
            }),
        ]);
        ctx.insert("items".into(), items);

        // map: "tags" -> [["A","B"], nil]
        // compact should flatten arrays and remove nils -> ["A","B"]
        let template = r#"{% assign tags = items | map: "tags" | compact %}{% for t in tags %}[{{ t }}]{% endfor %}"#;
        let result = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(result, "[A][B]");
    }

    #[test]
    fn test_uniq_sort_unicode_tags() {
        // Non-ASCII tag names must work correctly with flatten+uniq+sort
        let eng = engine();
        let mut ctx = Object::new();

        let items = LiquidValue::Array(vec![
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert(
                    "tags".into(),
                    LiquidValue::Array(vec![
                        LiquidValue::scalar("\u{53f0}\u{7063}"), // Taiwan in Chinese
                        LiquidValue::scalar("Design"),
                    ]),
                );
                o
            }),
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert(
                    "tags".into(),
                    LiquidValue::Array(vec![
                        LiquidValue::scalar("Design"),
                        LiquidValue::scalar("\u{00c4}sthetik"), // Asthetik with umlaut
                    ]),
                );
                o
            }),
        ]);
        ctx.insert("items".into(), items);

        let template = r#"{% assign tags = items | map: "tags" | uniq | sort %}{% for t in tags %}[{{ t }}]{% endfor %}"#;
        let result = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(result, "[Design][\u{00c4}sthetik][\u{53f0}\u{7063}]");
    }

    // ========================================================================
    // Issue 326: .size on data mappings (LenientValue objects)
    // ========================================================================

    #[test]
    fn test_size_on_nested_object_via_lenient_render() {
        // site.data.locales is an Object with 3 keys; .size should return 3
        let eng = engine();

        let mut locales = Object::new();
        locales.insert("en".into(), LiquidValue::scalar("English"));
        locales.insert("es".into(), LiquidValue::scalar("Espa\u{00f1}ol"));
        locales.insert("fr".into(), LiquidValue::scalar("Fran\u{00e7}ais"));

        let mut data = Object::new();
        data.insert("locales".into(), LiquidValue::Object(locales));

        let mut site = Object::new();
        site.insert("data".into(), LiquidValue::Object(data));

        let mut ctx = Object::new();
        ctx.insert("site".into(), LiquidValue::Object(site));

        let output = eng
            .parse_and_render("{{ site.data.locales.size }}", &ctx)
            .unwrap();
        assert_eq!(output, "3", "Expected .size on Object to return key count");
    }

    #[test]
    fn test_size_on_nested_object_in_condition() {
        // {% if site.data.locales.size > 1 %} should evaluate to true
        let eng = engine();

        let mut locales = Object::new();
        locales.insert("en".into(), LiquidValue::scalar("English"));
        locales.insert("de".into(), LiquidValue::scalar("Deutsch"));
        locales.insert("ja".into(), LiquidValue::scalar("\u{65e5}\u{672c}\u{8a9e}"));

        let mut data = Object::new();
        data.insert("locales".into(), LiquidValue::Object(locales));

        let mut site = Object::new();
        site.insert("data".into(), LiquidValue::Object(data));

        let mut ctx = Object::new();
        ctx.insert("site".into(), LiquidValue::Object(site));

        let output = eng
            .parse_and_render(
                "{% if site.data.locales.size > 1 %}yes{% else %}no{% endif %}",
                &ctx,
            )
            .unwrap();
        assert_eq!(
            output, "yes",
            "Condition on .size > 1 should be true for 3-key Object"
        );
    }

    #[test]
    fn test_size_on_object_with_unicode_keys() {
        // Nested Object with non-ASCII keys; .size should still work
        let eng = engine();

        let mut mapping = Object::new();
        mapping.insert("caf\u{00e9}".into(), LiquidValue::scalar(1));
        mapping.insert("\u{00fc}ber".into(), LiquidValue::scalar(2));
        mapping.insert("\u{4f60}\u{597d}".into(), LiquidValue::scalar(3));

        let mut data = Object::new();
        data.insert("items".into(), LiquidValue::Object(mapping));

        let mut site = Object::new();
        site.insert("data".into(), LiquidValue::Object(data));

        let mut ctx = Object::new();
        ctx.insert("site".into(), LiquidValue::Object(site));

        let output = eng
            .parse_and_render("{{ site.data.items.size }}", &ctx)
            .unwrap();
        assert_eq!(output, "3");
    }

    // ========================================================================
    // Sort filter on Object/Mapping -- hreflang pattern (issue 326)
    // ========================================================================

    #[test]
    fn test_sort_object_in_template_produces_key_value_pairs() {
        // Simulates: {% assign sorted = site.data.locales | sort %}
        //            {% for locale in sorted %}{{ locale[0] }},{% endfor %}
        let eng = engine();

        let mut locales = Object::new();
        locales.insert("es".into(), LiquidValue::scalar("Spanish"));
        locales.insert("ar".into(), LiquidValue::scalar("Arabic"));
        locales.insert("en".into(), LiquidValue::scalar("English"));

        let mut data = Object::new();
        data.insert("locales".into(), LiquidValue::Object(locales));
        let mut site = Object::new();
        site.insert("data".into(), LiquidValue::Object(data));

        let mut ctx = Object::new();
        ctx.insert("site".into(), LiquidValue::Object(site));

        let template = r#"{% assign sorted = site.data.locales | sort %}{% for locale in sorted %}{{ locale[0] }},{% endfor %}"#;
        let output = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(output, "ar,en,es,");
    }

    #[test]
    fn test_untranslated_absent_not_equal_true() {
        // When `untranslated` is absent from page, `page.untranslated != true`
        // must evaluate to true (nil != true is true in Jekyll).
        let eng = engine();

        let page = Object::new(); // no untranslated key
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page));

        let template = r#"{% if page.untranslated != true %}yes{% else %}no{% endif %}"#;
        let output = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(output, "yes");
    }

    #[test]
    fn test_untranslated_true_equals_true() {
        // When `untranslated: true`, `page.untranslated != true` must be false.
        let eng = engine();

        let mut page = Object::new();
        page.insert("untranslated".into(), LiquidValue::scalar(true));
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page));

        let template = r#"{% if page.untranslated != true %}yes{% else %}no{% endif %}"#;
        let output = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(output, "no");
    }

    #[test]
    fn test_hreflang_end_to_end_pattern() {
        // End-to-end test matching opensource-guide's head.html hreflang logic.
        let eng = engine();

        let mut locales = Object::new();
        locales.insert("en".into(), LiquidValue::scalar("English"));
        locales.insert("hu".into(), LiquidValue::scalar("Hungarian"));
        locales.insert("ar".into(), LiquidValue::scalar("Arabic"));

        let mut data = Object::new();
        data.insert("locales".into(), LiquidValue::Object(locales));
        let mut site = Object::new();
        site.insert("data".into(), LiquidValue::Object(data));

        let mut page = Object::new();
        page.insert("lang".into(), LiquidValue::scalar("hu"));
        page.insert("url".into(), LiquidValue::scalar("/hu/"));

        let mut ctx = Object::new();
        ctx.insert("site".into(), LiquidValue::Object(site));
        ctx.insert("page".into(), LiquidValue::Object(page));

        let template = r#"{% if page.lang and page.untranslated != true and site.data.locales.size > 1 %}{% assign locales = site.data.locales | sort %}{% for locale in locales %}{% assign lang = locale[0] %}<link hreflang="{{ lang }}">
{% endfor %}{% endif %}"#;

        let output = eng.parse_and_render(template, &ctx).unwrap();
        assert!(
            output.contains(r#"<link hreflang="ar">"#),
            "Should contain ar hreflang, got: {output}"
        );
        assert!(
            output.contains(r#"<link hreflang="en">"#),
            "Should contain en hreflang, got: {output}"
        );
        assert!(
            output.contains(r#"<link hreflang="hu">"#),
            "Should contain hu hreflang, got: {output}"
        );
        // 3 locales = 3 hreflang links
        let count = output.matches("<link hreflang=").count();
        assert_eq!(count, 3, "Expected 3 hreflang links, got {count}: {output}");
    }

    #[test]
    fn test_sort_object_unicode_in_template() {
        // Template-level test with non-ASCII locale values.
        let eng = engine();

        let mut locales = Object::new();
        let mut ar_obj = Object::new();
        ar_obj.insert(
            "name".into(),
            LiquidValue::scalar("\u{0627}\u{0644}\u{0639}\u{0631}\u{0628}\u{064A}\u{0629}"),
        ); // العربية
        locales.insert("ar".into(), LiquidValue::Object(ar_obj));

        let mut zh_obj = Object::new();
        zh_obj.insert(
            "name".into(),
            LiquidValue::scalar("\u{7B80}\u{4F53}\u{4E2D}\u{6587}"),
        ); // 简体中文
        locales.insert("zh-hans".into(), LiquidValue::Object(zh_obj));

        let mut data = Object::new();
        data.insert("locales".into(), LiquidValue::Object(locales));
        let mut site = Object::new();
        site.insert("data".into(), LiquidValue::Object(data));

        let mut ctx = Object::new();
        ctx.insert("site".into(), LiquidValue::Object(site));

        let template = r#"{% assign sorted = site.data.locales | sort %}{% for locale in sorted %}{{ locale[0] }}:{{ locale[1].name }},{% endfor %}"#;
        let output = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(output, "ar:\u{0627}\u{0644}\u{0639}\u{0631}\u{0628}\u{064A}\u{0629},zh-hans:\u{7B80}\u{4F53}\u{4E2D}\u{6587},");
    }

    // ========================================================================
    // Issue 328: preprocess_for_loop_filters
    // ========================================================================

    #[test]
    fn test_preprocess_for_loop_filters_sort() {
        let input = "{% for name in __names | sort %}{{ name }}{% endfor %}";
        let output = preprocess_for_loop_filters(input);
        assert!(
            !output.contains("{% for name in __names | sort %}"),
            "Filter chain in for loop should be extracted"
        );
        assert!(
            output.contains("{% assign __for_name"),
            "Should create temp assign: {}",
            output
        );
        assert!(
            output.contains("| sort"),
            "Filter should be preserved in assign: {}",
            output
        );
        assert!(
            output.contains("{% for name in __for_name"),
            "For loop should use temp var: {}",
            output
        );
    }

    #[test]
    fn test_preprocess_for_loop_filters_no_change_without_filter() {
        let input = "{% for item in items %}{{ item }}{% endfor %}";
        let output = preprocess_for_loop_filters(input);
        assert_eq!(
            output, input,
            "For loops without filters should be unchanged"
        );
    }

    #[test]
    fn test_preprocess_for_loop_filters_preserves_limit() {
        let input = "{% for post in site.posts limit:4 %}{{ post }}{% endfor %}";
        let output = preprocess_for_loop_filters(input);
        assert_eq!(
            output, input,
            "For loops with limit but no filter should be unchanged"
        );
    }

    #[test]
    fn test_preprocess_for_loop_filters_with_whitespace_control() {
        let input = "{%- for name in __names | sort -%}{{ name }}{%- endfor -%}";
        let output = preprocess_for_loop_filters(input);
        assert!(
            output.contains("| sort"),
            "Filter should be preserved: {}",
            output
        );
        assert!(
            output.contains("for name in __for_name"),
            "For loop should use temp var: {}",
            output
        );
    }

    // ========================================================================
    // preprocess_if_condition_filters
    // ========================================================================

    #[test]
    fn test_preprocess_if_condition_filters_basic() {
        let input =
            "{% if site.touchpoints.active and page.survey | default: false %}yes{% endif %}";
        let output = preprocess_if_condition_filters(input);
        assert!(
            output.contains("assign __if_filter_"),
            "Should extract filter chain into assign: {}",
            output
        );
        assert!(
            output.contains("| default: false"),
            "Filter chain should be in the assign: {}",
            output
        );
        assert!(
            !output.contains("and page.survey | default"),
            "Original filter chain should not be in the if condition: {}",
            output
        );
    }

    #[test]
    fn test_preprocess_if_condition_filters_no_change_without_pipe() {
        let input = "{% if site.active and page.survey %}yes{% endif %}";
        let output = preprocess_if_condition_filters(input);
        assert_eq!(output, input, "Should not change conditions without pipes");
    }

    #[test]
    fn test_preprocess_if_condition_filters_preserves_whitespace_control() {
        let input = "{%- if x and y | default: false -%}yes{%- endif -%}";
        let output = preprocess_if_condition_filters(input);
        assert!(
            output.contains("{%-"),
            "Should preserve leading dash: {}",
            output
        );
        assert!(
            output.contains("-%}"),
            "Should preserve trailing dash: {}",
            output
        );
    }

    #[test]
    fn test_preprocess_if_condition_filters_string_pipe_ignored() {
        // Pipes inside string literals should not be treated as filter separators
        let input = r#"{% if page.title == "Hello | World" %}yes{% endif %}"#;
        let output = preprocess_if_condition_filters(input);
        assert_eq!(
            output, input,
            "Pipes inside strings should not trigger rewrite"
        );
    }

    // ========================================================================
    // discover_unknown_filters_in_includes
    // ========================================================================

    #[test]
    fn test_discover_unknown_filters_in_includes() {
        let mut includes = HashMap::new();
        includes.insert(
            "test.html".to_string(),
            "{{ x | resolve_permalink }}".to_string(),
        );
        let result = TemplateEngine::discover_unknown_filters_in_includes(&includes).unwrap();
        assert!(
            result.contains("resolve_permalink"),
            "Should discover resolve_permalink as unknown: {:?}",
            result
        );
    }

    #[test]
    fn test_discover_known_filters_not_flagged() {
        let mut includes = HashMap::new();
        includes.insert(
            "test.html".to_string(),
            "{{ x | slugify | prepend: 'a' }}".to_string(),
        );
        let result = TemplateEngine::discover_unknown_filters_in_includes(&includes).unwrap();
        assert!(
            !result.contains("slugify"),
            "Known filter slugify should not be flagged: {:?}",
            result
        );
        assert!(
            !result.contains("prepend"),
            "Known filter prepend should not be flagged: {:?}",
            result
        );
    }

    // ========================================================================
    // Issue 328: preprocess_parenthesized_assign
    // ========================================================================

    #[test]
    fn test_preprocess_parenthesized_assign_basic() {
        let input = "{% assign tag_hashes = (page_tags | split: ',' | sort) %}";
        let output = preprocess_parenthesized_assign(input);
        assert!(
            !output.contains('('),
            "Parentheses should be removed: {}",
            output
        );
        assert!(
            output.contains("assign tag_hashes = page_tags | split: ',' | sort"),
            "Expression should be preserved without parens: {}",
            output
        );
    }

    #[test]
    fn test_preprocess_parenthesized_assign_no_change() {
        let input = "{% assign x = arr | sort %}";
        let output = preprocess_parenthesized_assign(input);
        assert_eq!(
            output, input,
            "Non-parenthesized assigns should be unchanged"
        );
    }

    #[test]
    fn test_preprocess_parenthesized_assign_with_colon_arg() {
        // sort:0 is valid Jekyll syntax (sort with argument 0)
        let input = "{% assign tag_hashes = (page_tags | split: ',' | sort:0) %}";
        let output = preprocess_parenthesized_assign(input);
        assert!(
            !output.contains('('),
            "Parentheses should be removed: {}",
            output
        );
    }

    // ========================================================================
    // Issue 328: Include content gets full preprocessing
    // ========================================================================

    #[test]
    fn test_include_with_for_loop_filter_renders() {
        // Simulate the group-by-array include pattern:
        // The include file uses {% for name in __names | sort %}
        let dir = tempfile::TempDir::new().unwrap();
        let includes_dir = dir.path().join("_includes");
        std::fs::create_dir_all(&includes_dir).unwrap();
        std::fs::write(
            includes_dir.join("test-include"),
            "{% assign items = 'c,a,b' | split: ',' %}{% assign __for_item = items | sort %}{% for item in __for_item %}{{ item }},{% endfor %}",
        ).unwrap();

        let eng = TemplateEngine::with_includes(&includes_dir).unwrap();
        let ctx = Object::new();
        let output = eng
            .parse_and_render("{% include test-include %}", &ctx)
            .unwrap();
        assert!(
            output.contains("a,b,c,"),
            "Include should render sorted items: {}",
            output
        );
    }

    #[test]
    fn test_extensionless_include_with_unicode() {
        let dir = tempfile::TempDir::new().unwrap();
        let includes_dir = dir.path().join("_includes");
        std::fs::create_dir_all(&includes_dir).unwrap();
        // File without extension, containing Unicode content
        std::fs::write(
            includes_dir.join("unicode-test"),
            "<p>Formule: $$\\alpha + \\beta$$, \u{4F60}\u{597D}</p>",
        )
        .unwrap();

        let eng = TemplateEngine::with_includes(&includes_dir).unwrap();
        let ctx = Object::new();
        let output = eng
            .parse_and_render("{% include unicode-test %}", &ctx)
            .unwrap();
        assert!(
            output.contains("$$\\alpha + \\beta$$"),
            "Unicode math notation should be preserved: {}",
            output
        );
        assert!(
            output.contains("\u{4F60}\u{597D}"),
            "CJK characters should be preserved: {}",
            output
        );
    }

    #[test]
    fn test_extensionless_include_with_params() {
        let dir = tempfile::TempDir::new().unwrap();
        let includes_dir = dir.path().join("_includes");
        std::fs::create_dir_all(&includes_dir).unwrap();
        std::fs::write(
            includes_dir.join("group-helper"),
            "field={{ include.field }}",
        )
        .unwrap();

        let eng = TemplateEngine::with_includes(&includes_dir).unwrap();
        let ctx = Object::new();
        let output = eng
            .parse_and_render("{% include group-helper field=\"categories\" %}", &ctx)
            .unwrap();
        assert!(
            output.contains("field=categories"),
            "Include params should be accessible: {}",
            output
        );
    }

    // ========================================================================
    // Issue 328: for-loop filter integration test
    // ========================================================================

    #[test]
    fn test_for_loop_with_filter_chain_renders() {
        let eng = TemplateEngine::new().unwrap();
        let mut ctx = Object::new();
        ctx.insert(
            "__names".into(),
            LiquidValue::Array(vec![
                LiquidValue::scalar("cherry"),
                LiquidValue::scalar("apple"),
                LiquidValue::scalar("banana"),
            ]),
        );
        // This pattern is used in academicpages group-by-array include
        let template = "{% for name in __names | sort %}{{ name }},{% endfor %}";
        let output = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(output, "apple,banana,cherry,");
    }

    // ========================================================================
    // Issue 348: Malformed frontmatter description renders as Ruby-style hash
    // ========================================================================

    #[test]
    fn test_issue348_malformed_frontmatter_description_renders_like_jekyll_hash() {
        // When YAML parses `description:` with a colon-space in the indented text,
        // it becomes a mapping. Jekyll renders this as a Ruby hash string.
        let eng = engine();
        let mut page = liquid::Object::new();
        let mut desc_obj = liquid::Object::new();
        desc_obj.insert(
            "Learn containerized ML deployment on AWS Lambda".into(),
            LiquidValue::scalar(
                "build, train, and serve with Docker, ECR, and SAM, plus CI/CD via GitHub Actions. Follow this proven guide.",
            ),
        );
        // Simulate __key_order that rustkyll's YAML parser adds
        desc_obj.insert(
            "__key_order".into(),
            LiquidValue::Array(vec![LiquidValue::scalar(
                "Learn containerized ML deployment on AWS Lambda",
            )]),
        );
        page.insert("description".into(), LiquidValue::Object(desc_obj));
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page));

        let template = r#"<meta name="description" content="{{ page.description }}">"#;
        let output = eng.parse_and_render(template, &ctx).unwrap();

        // Jekyll renders mappings as Ruby hash strings
        let expected = r#"<meta name="description" content="{"Learn containerized ML deployment on AWS Lambda"=>"build, train, and serve with Docker, ECR, and SAM, plus CI/CD via GitHub Actions. Follow this proven guide."}">"#;
        assert_eq!(output, expected);
    }

    #[test]
    fn test_issue348_malformed_frontmatter_description_jsonify_stays_object() {
        // The jsonify filter should render the mapping as a JSON object, not a string.
        let eng = engine();
        let mut page = liquid::Object::new();
        let mut desc_obj = liquid::Object::new();
        desc_obj.insert(
            "Learn containerized ML deployment on AWS Lambda".into(),
            LiquidValue::scalar(
                "build, train, and serve with Docker, ECR, and SAM, plus CI/CD via GitHub Actions. Follow this proven guide.",
            ),
        );
        desc_obj.insert(
            "__key_order".into(),
            LiquidValue::Array(vec![LiquidValue::scalar(
                "Learn containerized ML deployment on AWS Lambda",
            )]),
        );
        page.insert("description".into(), LiquidValue::Object(desc_obj));
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page));

        let template = r#"{{ page.description | jsonify }}"#;
        let output = eng.parse_and_render(template, &ctx).unwrap();

        // jsonify renders the mapping as a JSON object
        assert!(
            output.contains("\"Learn containerized ML deployment on AWS Lambda\""),
            "jsonify output should contain the key: {output}"
        );
        assert!(
            output.starts_with('{'),
            "jsonify output should be a JSON object: {output}"
        );
    }

    // ========================================================================
    // Issue 399: for-loop over Object with __key_order uses insertion order
    // ========================================================================

    #[test]
    fn test_for_loop_object_with_key_order_iterates_in_order() {
        let eng = engine();

        // Build an Object with __key_order in non-alphabetical order
        let mut cats = Object::new();
        cats.insert(
            "__key_order".into(),
            LiquidValue::Array(vec![
                LiquidValue::scalar("zebra"),
                LiquidValue::scalar("apple"),
                LiquidValue::scalar("middle"),
            ]),
        );
        cats.insert(
            "zebra".into(),
            LiquidValue::Array(vec![LiquidValue::scalar("z1")]),
        );
        cats.insert(
            "apple".into(),
            LiquidValue::Array(vec![LiquidValue::scalar("a1")]),
        );
        cats.insert(
            "middle".into(),
            LiquidValue::Array(vec![LiquidValue::scalar("m1")]),
        );

        let mut site = Object::new();
        site.insert("categories".into(), LiquidValue::Object(cats));

        let mut ctx = Object::new();
        ctx.insert("site".into(), LiquidValue::Object(site));

        let template = "{% for cat in site.categories %}{{ cat[0] }},{% endfor %}";
        let output = eng.parse_and_render(template, &ctx).unwrap();

        assert_eq!(
            output, "zebra,apple,middle,",
            "For-loop over Object with __key_order should iterate in specified order, not alphabetical"
        );
    }

    // ========================================================================
    // Issue: .size on Object with __key_order excludes metadata key
    // ========================================================================

    #[test]
    fn test_size_excludes_key_order_metadata() {
        // site.tags has 2 real tags + __key_order metadata.
        // .size should return 2, not 3.
        let eng = engine();

        let mut tags = Object::new();
        tags.insert(
            "__key_order".into(),
            LiquidValue::Array(vec![
                LiquidValue::scalar("sample"),
                LiquidValue::scalar("test"),
            ]),
        );
        tags.insert(
            "sample".into(),
            LiquidValue::Array(vec![LiquidValue::scalar("post1")]),
        );
        tags.insert(
            "test".into(),
            LiquidValue::Array(vec![LiquidValue::scalar("post2")]),
        );

        let mut site = Object::new();
        site.insert("tags".into(), LiquidValue::Object(tags));

        let mut ctx = Object::new();
        ctx.insert("site".into(), LiquidValue::Object(site));

        let output = eng.parse_and_render("{{ site.tags.size }}", &ctx).unwrap();
        assert_eq!(
            output, "2",
            ".size on Object with __key_order should exclude the metadata key"
        );
    }

    #[test]
    fn test_size_without_key_order_unchanged() {
        // Object without __key_order should report all keys.
        let eng = engine();

        let mut tags = Object::new();
        tags.insert(
            "sample".into(),
            LiquidValue::Array(vec![LiquidValue::scalar("post1")]),
        );
        tags.insert(
            "test".into(),
            LiquidValue::Array(vec![LiquidValue::scalar("post2")]),
        );

        let mut site = Object::new();
        site.insert("tags".into(), LiquidValue::Object(tags));

        let mut ctx = Object::new();
        ctx.insert("site".into(), LiquidValue::Object(site));

        let output = eng.parse_and_render("{{ site.tags.size }}", &ctx).unwrap();
        assert_eq!(
            output, "2",
            ".size on Object without __key_order should count all keys"
        );
    }

    // ========================================================================
    // Issue 540: site.tags / site.categories iteration with [1].size
    // ========================================================================

    #[test]
    fn test_issue540_tag_pair_index1_size() {
        // basically-basic uses: {% for tag in site.tags %}{% if tag[1].size > tags_max %}
        // tag[0] = tag name, tag[1] = posts array, tag[1].size = number of posts
        let eng = engine();

        let mut post1 = Object::new();
        post1.insert("title".into(), LiquidValue::scalar("Post One"));
        post1.insert(
            "url".into(),
            LiquidValue::scalar("/2024/01/01/post-one.html"),
        );
        let mut post2 = Object::new();
        post2.insert("title".into(), LiquidValue::scalar("Post Two"));
        post2.insert(
            "url".into(),
            LiquidValue::scalar("/2024/01/02/post-two.html"),
        );

        let mut tags = Object::new();
        tags.insert(
            "__key_order".into(),
            LiquidValue::Array(vec![
                LiquidValue::scalar("markup"),
                LiquidValue::scalar("code"),
            ]),
        );
        tags.insert(
            "markup".into(),
            LiquidValue::Array(vec![
                LiquidValue::Object(post1.clone()),
                LiquidValue::Object(post2.clone()),
            ]),
        );
        tags.insert(
            "code".into(),
            LiquidValue::Array(vec![LiquidValue::Object(post1.clone())]),
        );

        let mut site = Object::new();
        site.insert("tags".into(), LiquidValue::Object(tags));

        let mut ctx = Object::new();
        ctx.insert("site".into(), LiquidValue::Object(site));

        // Test tag[1].size -- should return the number of posts in each tag
        let template = "{% for tag in site.tags %}{{ tag[0] }}:{{ tag[1].size }},{% endfor %}";
        let output = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(
            output, "markup:2,code:1,",
            "tag[1].size should return the number of posts for each tag"
        );
    }

    #[test]
    fn test_issue540_tag_pair_index1_size_comparison() {
        // Test the comparison pattern: {% if tag[1].size > tags_max %}
        let eng = engine();

        let mut post1 = Object::new();
        post1.insert("title".into(), LiquidValue::scalar("Post One"));

        let mut tags = Object::new();
        tags.insert(
            "__key_order".into(),
            LiquidValue::Array(vec![
                LiquidValue::scalar("markup"),
                LiquidValue::scalar("code"),
            ]),
        );
        tags.insert(
            "markup".into(),
            LiquidValue::Array(vec![
                LiquidValue::Object(post1.clone()),
                LiquidValue::Object(post1.clone()),
                LiquidValue::Object(post1.clone()),
            ]),
        );
        tags.insert(
            "code".into(),
            LiquidValue::Array(vec![LiquidValue::Object(post1.clone())]),
        );

        let mut site = Object::new();
        site.insert("tags".into(), LiquidValue::Object(tags));

        let mut ctx = Object::new();
        ctx.insert("site".into(), LiquidValue::Object(site));

        // Pattern from basically-basic: find max tag count
        let template = r#"{% assign tags_max = 0 %}{% for tag in site.tags %}{% if tag[1].size > tags_max %}{% assign tags_max = tag[1].size %}{% endif %}{% endfor %}max:{{ tags_max }}"#;
        let output = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(
            output, "max:3",
            "Should find maximum tag count of 3 from markup tag"
        );
    }

    #[test]
    fn test_issue540_tag_last_iterates_posts() {
        // basically-basic also uses: {% for post in tag.last %}
        // tag.last should return the posts array (the value part of the pair)
        let eng = engine();

        let mut post1 = Object::new();
        post1.insert("title".into(), LiquidValue::scalar("Post A"));
        let mut post2 = Object::new();
        post2.insert("title".into(), LiquidValue::scalar("Post B"));

        let mut tags = Object::new();
        tags.insert(
            "__key_order".into(),
            LiquidValue::Array(vec![LiquidValue::scalar("markup")]),
        );
        tags.insert(
            "markup".into(),
            LiquidValue::Array(vec![LiquidValue::Object(post1), LiquidValue::Object(post2)]),
        );

        let mut site = Object::new();
        site.insert("tags".into(), LiquidValue::Object(tags));

        let mut ctx = Object::new();
        ctx.insert("site".into(), LiquidValue::Object(site));

        let template =
            "{% for tag in site.tags %}{% for post in tag.last %}{{ post.title }},{% endfor %}{% endfor %}";
        let output = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(
            output, "Post A,Post B,",
            "tag.last should iterate over the posts array"
        );
    }

    #[test]
    fn test_issue540_category_pair_index1_size() {
        // Same pattern for categories
        let eng = engine();

        let mut post1 = Object::new();
        post1.insert("title".into(), LiquidValue::scalar("Post One"));

        let mut categories = Object::new();
        categories.insert(
            "__key_order".into(),
            LiquidValue::Array(vec![LiquidValue::scalar("tutorials")]),
        );
        categories.insert(
            "tutorials".into(),
            LiquidValue::Array(vec![
                LiquidValue::Object(post1.clone()),
                LiquidValue::Object(post1.clone()),
            ]),
        );

        let mut site = Object::new();
        site.insert("categories".into(), LiquidValue::Object(categories));

        let mut ctx = Object::new();
        ctx.insert("site".into(), LiquidValue::Object(site));

        let template = "{% for category in site.categories %}{{ category[0] }}:{{ category[1].size }},{% endfor %}";
        let output = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(
            output, "tutorials:2,",
            "category[1].size should return the number of posts for each category"
        );
    }

    // ========================================================================
    // Issue 441: Liquid rendering failures for theme sites
    // ========================================================================

    #[test]
    fn test_include_with_undefined_variable_param_renders_nil() {
        // text-theme passes undefined nested variable references as include
        // parameters (e.g., target=layout.header where layout.header is nil).
        // The include tag should treat these as Nil, not error.
        let mut includes = HashMap::new();
        includes.insert(
            "test.html".to_string(),
            "param={{ include.target }}".to_string(),
        );
        let eng = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        // layout.header is completely missing -> should render as nil (empty)
        let result = eng.parse_and_render(r#"{% include test.html target=layout.header %}"#, &ctx);
        assert!(
            result.is_ok(),
            "Include with undefined variable param should not error, got: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_include_with_leading_slash_resolves() {
        // minimal-mistakes uses {% include /comments-providers/scripts.html %}
        // where the registered partial is "comments-providers/scripts.html".
        // The leading slash should be stripped for resolution.
        let mut includes = HashMap::new();
        includes.insert(
            "comments-providers/scripts.html".to_string(),
            "FOUND".to_string(),
        );
        let eng = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let result =
            eng.parse_and_render(r#"{% include "/comments-providers/scripts.html" %}"#, &ctx);
        assert!(
            result.is_ok(),
            "Include with leading slash should resolve, got: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap(), "FOUND");
    }

    #[test]
    fn test_output_or_operator_preprocessed() {
        // hydeout uses {{ page.guid or page.id }} which Jekyll supports.
        // The `or` should be preprocessed to use `default` filter.
        let eng = engine();
        let mut page = Object::new();
        page.insert("id".into(), LiquidValue::scalar("post-123"));
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page));
        // page.guid is nil, so page.id should be used via default
        let result = eng.parse_and_render("{{ page.guid or page.id | render_mapping }}", &ctx);
        assert!(
            result.is_ok(),
            "Output tag with 'or' should render, got: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap(), "post-123");
    }

    #[test]
    fn test_multiline_include_path_resolves() {
        // text-theme uses multiline include tags where the path is followed
        // by a newline before parameters. The path should not include the newline.
        let mut includes = HashMap::new();
        includes.insert("snippets/prepend-path.html".to_string(), "OK".to_string());
        let eng = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let result = eng.parse_and_render(
            "{%- include snippets/prepend-path.html\n  path=something -%}",
            &ctx,
        );
        assert!(
            result.is_ok(),
            "Multiline include should resolve, got: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap(), "OK");
    }

    #[test]
    fn test_stray_brace_in_assign_tag() {
        // text-theme has a typo: {% assign x = y | url_encode } -%}
        // The stray } before -%} should be stripped during preprocessing.
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("tag".into(), LiquidValue::scalar("hello world"));
        let result = eng.parse_and_render(
            "{%- assign encoded = tag | url_encode } -%}{{ encoded }}",
            &ctx,
        );
        assert!(
            result.is_ok(),
            "Stray brace in assign tag should not error, got: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap(), "hello+world");
    }

    #[test]
    fn test_case_with_else_default_branch() {
        // text-theme uses {% case %}...{% else %}...{% else %}...{% endcase %}
        // The else branch should work as the default case.
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("x".into(), LiquidValue::scalar("unknown"));
        let result = eng.parse_and_render(
            "{% case x %}{% when 'a' %}A{% when 'b' %}B{% else %}DEFAULT{% endcase %}",
            &ctx,
        );
        assert!(
            result.is_ok(),
            "Case with else should render, got: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap(), "DEFAULT");
    }

    #[test]
    fn test_case_with_multiple_else_branches() {
        // text-theme has duplicate {% else %} inside {% case %}.
        // Should parse and render the first else body.
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("x".into(), LiquidValue::scalar("unknown"));
        let result = eng.parse_and_render(
            "{% case x %}{% when 'a' %}A{% else %}FIRST{% else %}{% endcase %}",
            &ctx,
        );
        assert!(
            result.is_ok(),
            "Case with multiple else should render, got: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap(), "FIRST");
    }

    // ========================================================================
    // Issue 504: false == nil preprocessing (symmetric to nil == false)
    // ========================================================================

    #[test]
    fn test_issue504_false_var_eq_nil_when_false() {
        // When layout.nav_enabled is false, `layout.nav_enabled == nil` should be false
        let eng = engine();
        let mut ctx = Object::new();
        let mut layout = Object::new();
        layout.insert("nav_enabled".into(), Value::scalar(false));
        ctx.insert("layout".into(), Value::Object(layout));

        let output = eng
            .parse_and_render(
                "{% if layout.nav_enabled == nil %}YES{% else %}NO{% endif %}",
                &ctx,
            )
            .unwrap();
        assert_eq!(output.trim(), "NO", "false should NOT equal nil");
    }

    #[test]
    fn test_issue504_absent_var_eq_nil_when_missing() {
        // When layout.nav_enabled is absent (truly nil), `layout.nav_enabled == nil` should be true
        let eng = engine();
        let mut ctx = Object::new();
        let layout = Object::new(); // no nav_enabled
        ctx.insert("layout".into(), Value::Object(layout));

        let output = eng
            .parse_and_render(
                "{% if layout.nav_enabled == nil %}YES{% else %}NO{% endif %}",
                &ctx,
            )
            .unwrap();
        assert_eq!(output.trim(), "YES", "nil should equal nil");
    }

    #[test]
    fn test_issue504_var_neq_nil_when_false() {
        // When layout.nav_enabled is false, `layout.nav_enabled != nil` should be true
        let eng = engine();
        let mut ctx = Object::new();
        let mut layout = Object::new();
        layout.insert("nav_enabled".into(), Value::scalar(false));
        ctx.insert("layout".into(), Value::Object(layout));

        let output = eng
            .parse_and_render(
                "{% if layout.nav_enabled != nil %}YES{% else %}NO{% endif %}",
                &ctx,
            )
            .unwrap();
        assert_eq!(
            output.trim(),
            "YES",
            "false should NOT equal nil, so != nil should be true"
        );
    }

    #[test]
    fn test_issue504_combined_nil_false_conditions() {
        // Simulates the just-the-docs default.html sidebar logic:
        // site.nav_enabled != false and layout.nav_enabled == nil and page.nav_enabled == nil
        // When layout.nav_enabled is false, this should NOT render
        let eng = engine();
        let mut ctx = Object::new();
        let mut layout = Object::new();
        layout.insert("nav_enabled".into(), Value::scalar(false));
        ctx.insert("layout".into(), Value::Object(layout));
        let site = Object::new();
        ctx.insert("site".into(), Value::Object(site));
        let page = Object::new();
        ctx.insert("page".into(), Value::Object(page));

        let output = eng
            .parse_and_render(
                "{% if site.nav_enabled != false and layout.nav_enabled == nil and page.nav_enabled == nil %}SIDEBAR{% endif %}",
                &ctx,
            )
            .unwrap();
        assert_eq!(
            output.trim(),
            "",
            "Sidebar should NOT render when layout.nav_enabled is false"
        );
    }

    // ========================================================================
    // Issue 523: render_mapping must not leak into code blocks or raw blocks
    // ========================================================================

    #[test]
    fn test_issue523_render_mapping_skips_fenced_code_blocks() {
        // Fenced code blocks (```) should not have render_mapping injected
        let input = "{{ title | render_mapping }}\n```\n{{ page.title }}\n```\n{{ footer | render_mapping }}";
        let result = preprocess_bare_output_render_mapping(input);
        // The code block interior should be untouched
        assert!(
            result.contains("```\n{{ page.title }}\n```"),
            "render_mapping leaked into fenced code block. Got: {}",
            result
        );
    }

    #[test]
    fn test_issue523_render_mapping_skips_tilde_fenced_code_blocks() {
        let input = "~~~\n{{ page.title }}\n~~~\n{{ outside }}";
        let result = preprocess_bare_output_render_mapping(input);
        assert!(
            result.contains("~~~\n{{ page.title }}\n~~~"),
            "render_mapping leaked into tilde-fenced code block. Got: {}",
            result
        );
        // Outside the fence should still get render_mapping
        assert!(
            result.contains("outside") && result.contains("render_mapping"),
            "render_mapping not applied outside fence. Got: {}",
            result
        );
    }

    #[test]
    fn test_issue523_render_mapping_skips_raw_blocks() {
        let input = "{% raw %}{{ page.title }}{% endraw %}";
        let result = preprocess_bare_output_render_mapping(input);
        assert!(
            result.contains("{% raw %}{{ page.title }}{% endraw %}"),
            "render_mapping leaked into raw block. Got: {}",
            result
        );
    }

    #[test]
    fn test_issue523_render_mapping_skips_highlight_blocks() {
        let input = "{% highlight html %}{{ page.title }}{% endhighlight %}";
        let result = preprocess_bare_output_render_mapping(input);
        assert!(
            result.contains("{% highlight html %}{{ page.title }}{% endhighlight %}"),
            "render_mapping leaked into highlight block. Got: {}",
            result
        );
    }

    #[test]
    fn test_issue523_render_mapping_applied_outside_code_blocks() {
        let input = "{{ normal_var }}\n```\n{{ code_var }}\n```\n{{ another_var }}";
        let result = preprocess_bare_output_render_mapping(input);
        assert!(
            result.contains("normal_var") && result.contains("| render_mapping"),
            "render_mapping not applied before code block. Got: {}",
            result
        );
        // Verify outside-code variables get render_mapping while code_var does not
        assert!(
            !result.contains("code_var | render_mapping")
                && !result.contains("code_var  | render_mapping"),
            "render_mapping leaked into code block. Got: {}",
            result
        );
        assert!(
            result.contains("```\n{{ code_var }}\n```"),
            "render_mapping leaked into code block. Got: {}",
            result
        );
    }

    #[test]
    fn test_issue523_render_mapping_fenced_with_language() {
        // Fenced code blocks with language annotation
        let input = "```liquid\n{{ page.title }}\n```";
        let result = preprocess_bare_output_render_mapping(input);
        assert!(
            result.contains("```liquid\n{{ page.title }}\n```"),
            "render_mapping leaked into fenced code block with language. Got: {}",
            result
        );
    }

    #[test]
    fn test_issue504_neq_false_no_rewrite_preserves_compound() {
        // The != false rewrite used to introduce `or` which broke precedence.
        // Verify it no longer rewrites != false.
        let input = "{% if site.nav_enabled != false and layout.nav_enabled == nil and page.nav_enabled == nil %}SIDEBAR{% endif %}";
        let output = preprocess_nil_eq_false(input);
        // Only == nil comparisons should remain untouched; != false should not introduce or
        assert!(
            !output.contains(" or "),
            "!= false should not introduce 'or'. Got: {}",
            output
        );
    }

    // ========================================================================
    // Issue 528: array_to_sentence_string on nil input
    // ========================================================================

    #[test]
    fn test_array_to_sentence_string_nil_returns_empty() {
        // When post.categories is nil, array_to_sentence_string should return ""
        // instead of erroring, matching Jekyll behavior.
        let eng = engine();
        let ctx = Object::new(); // no "cats" variable -> nil
        let out = eng
            .parse_and_render("{{ cats | array_to_sentence_string }}", &ctx)
            .unwrap();
        assert_eq!(out, "", "nil input should produce empty string");
    }

    #[test]
    fn test_array_to_sentence_string_normal_array() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert(
            "cats".into(),
            LiquidValue::Array(vec![
                LiquidValue::scalar("release"),
                LiquidValue::scalar("update"),
            ]),
        );
        let out = eng
            .parse_and_render("{{ cats | array_to_sentence_string }}", &ctx)
            .unwrap();
        assert_eq!(out, "release, and update");
    }

    // ========================================================================
    // Issue 247: Object.first should return [key, value] pair (Ruby Liquid compat)
    // ========================================================================

    #[test]
    fn test_object_first_returns_key_value_pair() {
        // In Ruby Liquid, hash.first returns the first [key, value] pair.
        // categories_list.first[0] should return the first category name (not nil).
        let eng = engine();

        let mut categories = Object::new();
        categories.insert(
            "__key_order".into(),
            LiquidValue::Array(vec![
                LiquidValue::scalar("Jekyll"),
                LiquidValue::scalar("tutorial"),
            ]),
        );
        categories.insert(
            "Jekyll".into(),
            LiquidValue::Array(vec![LiquidValue::scalar("post1")]),
        );
        categories.insert(
            "tutorial".into(),
            LiquidValue::Array(vec![
                LiquidValue::scalar("post1"),
                LiquidValue::scalar("post2"),
            ]),
        );

        let mut site = Object::new();
        site.insert("categories".into(), LiquidValue::Object(categories));

        let mut ctx = Object::new();
        ctx.insert("site".into(), LiquidValue::Object(site));

        // .first on an Object should return the first [key, value] pair
        // .first[0] should return the key (category name)
        let template = "{{ site.categories.first[0] }}";
        let output = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(
            output, "Jekyll",
            "Object.first[0] should return the first key"
        );
    }

    #[test]
    fn test_object_first_null_check_mediumish_pattern() {
        // Mediumish template pattern:
        //   {% assign categories_list = site.categories %}
        //   {% if categories_list.first[0] == null %}
        //     (simple array branch)
        //   {% else %}
        //     (hash branch - should be taken for site.categories)
        //   {% endif %}
        let eng = engine();

        let mut categories = Object::new();
        categories.insert(
            "__key_order".into(),
            LiquidValue::Array(vec![LiquidValue::scalar("Jekyll")]),
        );
        categories.insert(
            "Jekyll".into(),
            LiquidValue::Array(vec![LiquidValue::scalar("post1")]),
        );

        let mut site = Object::new();
        site.insert("categories".into(), LiquidValue::Object(categories));

        let mut ctx = Object::new();
        ctx.insert("site".into(), LiquidValue::Object(site));

        let template = r#"{% assign categories_list = site.categories %}{% if categories_list.first[0] == null %}NULL{% else %}NOT_NULL{% endif %}"#;
        let output = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(
            output, "NOT_NULL",
            "categories_list.first[0] should not be null for an Object with keys"
        );
    }

    #[test]
    fn test_object_last_returns_key_value_pair() {
        let eng = engine();

        let mut categories = Object::new();
        categories.insert(
            "__key_order".into(),
            LiquidValue::Array(vec![
                LiquidValue::scalar("Jekyll"),
                LiquidValue::scalar("tutorial"),
            ]),
        );
        categories.insert(
            "Jekyll".into(),
            LiquidValue::Array(vec![LiquidValue::scalar("post1")]),
        );
        categories.insert(
            "tutorial".into(),
            LiquidValue::Array(vec![LiquidValue::scalar("post2")]),
        );

        let mut site = Object::new();
        site.insert("categories".into(), LiquidValue::Object(categories));

        let mut ctx = Object::new();
        ctx.insert("site".into(), LiquidValue::Object(site));

        let template = "{{ site.categories.last[0] }}";
        let output = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(
            output, "tutorial",
            "Object.last[0] should return the last key"
        );
    }

    // ========================================================================
    // Issue 247: url_escape filter
    // ========================================================================

    #[test]
    fn test_url_escape_filter() {
        // url_escape is a passthrough (not a standard Jekyll filter)
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("input".into(), LiquidValue::scalar("web development"));
        let output = eng
            .parse_and_render("{{ input | url_escape }}", &ctx)
            .unwrap();
        assert_eq!(
            output, "web development",
            "url_escape should pass through unchanged"
        );
    }

    // ========================================================================
    // Issue 247: camelcase filter
    // ========================================================================

    #[test]
    fn test_camelcase_filter() {
        // camelcase is a passthrough (not a standard Jekyll filter)
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("input".into(), LiquidValue::scalar("hello_world"));
        let output = eng
            .parse_and_render("{{ input | camelcase }}", &ctx)
            .unwrap();
        assert_eq!(
            output, "hello_world",
            "camelcase should pass through unchanged"
        );
    }

    #[test]
    fn test_camelcase_filter_simple_word() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("input".into(), LiquidValue::scalar("tutorial"));
        let output = eng
            .parse_and_render("{{ input | camelcase }}", &ctx)
            .unwrap();
        assert_eq!(
            output, "tutorial",
            "camelcase should pass through unchanged"
        );
    }

    // ========================================================================
    // Issue 444: Liquid template rendering gaps -- include resolution
    // ========================================================================

    #[test]
    fn test_include_resolution_existing_file() {
        let mut includes = HashMap::new();
        includes.insert(
            "header.html".to_string(),
            "<header>Site Header</header>".to_string(),
        );
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let output = engine
            .parse_and_render("{% include header.html %}", &ctx)
            .unwrap();
        assert_eq!(output, "<header>Site Header</header>");
    }

    #[test]
    fn test_include_resolution_nested_includes() {
        let mut includes = HashMap::new();
        includes.insert("b.html".to_string(), "B-CONTENT".to_string());
        includes.insert("a.html".to_string(), "A[{% include b.html %}]A".to_string());
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let output = engine
            .parse_and_render("{% include a.html %}", &ctx)
            .unwrap();
        assert_eq!(output, "A[B-CONTENT]A");
    }

    #[test]
    fn test_include_missing_file_returns_error() {
        // Missing includes should produce an error, not raw Liquid in output
        let includes = HashMap::new();
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let result = engine.parse_and_render("{% include missing.html %}", &ctx);
        assert!(
            result.is_err(),
            "Missing include should return an error, not raw Liquid"
        );
    }

    #[test]
    fn test_include_file_without_extension() {
        // Jekyll supports include files without .html extension (e.g., feature_row)
        let mut includes = HashMap::new();
        includes.insert(
            "feature_row".to_string(),
            "<div class=\"feature-row\">Features</div>".to_string(),
        );
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let output = engine
            .parse_and_render("{% include feature_row %}", &ctx)
            .unwrap();
        assert_eq!(output, "<div class=\"feature-row\">Features</div>");
    }

    // ========================================================================
    // Issue 444: Liquid template rendering gaps -- date filter
    // ========================================================================

    #[test]
    fn test_date_filter_yyyy_mm_dd() {
        let eng = engine();
        let mut page = Object::new();
        page.insert("date".into(), LiquidValue::scalar("2024-07-24"));
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page));
        let output = eng
            .parse_and_render(r#"{{ page.date | date: "%Y-%m-%d" }}"#, &ctx)
            .unwrap();
        assert_eq!(output, "2024-07-24");
    }

    #[test]
    fn test_date_filter_full_month_name() {
        let eng = engine();
        let mut page = Object::new();
        page.insert("date".into(), LiquidValue::scalar("2024-07-24"));
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page));
        let output = eng
            .parse_and_render(r#"{{ page.date | date: "%B %d, %Y" }}"#, &ctx)
            .unwrap();
        assert_eq!(output, "July 24, 2024");
    }

    #[test]
    fn test_date_filter_on_nil_returns_empty() {
        // Date filter on nil/missing date should not produce raw Liquid output
        let eng = engine();
        let ctx = Object::new();
        let output = eng
            .parse_and_render(r#"{{ page.date | date: "%Y-%m-%d" }}"#, &ctx)
            .unwrap();
        // nil value should render as empty, not raw Liquid
        assert!(
            !output.contains("{{"),
            "Date filter on nil should not produce raw Liquid, got: {}",
            output
        );
    }

    // ========================================================================
    // Issue 444: Non-ASCII content in Liquid rendering
    // ========================================================================

    #[test]
    fn test_include_with_unicode_content() {
        let mut includes = HashMap::new();
        includes.insert(
            "greeting.html".to_string(),
            "<p>Bonjour {{ include.name }}! \u{1F600}</p>".to_string(),
        );
        let engine = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let output = engine
            .parse_and_render(r#"{% include greeting.html name="Fran\u00e7ois" %}"#, &ctx)
            .unwrap();
        assert!(
            output.contains("Bonjour"),
            "Unicode include should render, got: {}",
            output
        );
        assert!(!output.contains("{%"), "Should not contain raw Liquid tags");
    }

    #[test]
    fn test_cached_site_context_from_object_matches_new() {
        let mut site = Object::new();
        site.insert("title".into(), LiquidValue::scalar("Test Site"));
        site.insert(
            "posts".into(),
            LiquidValue::Array(vec![{
                let mut post = Object::new();
                post.insert("title".into(), LiquidValue::scalar("Hello"));
                post.insert("url".into(), LiquidValue::scalar("/hello.html"));
                LiquidValue::Object(post)
            }]),
        );

        let site_clone = site.clone();
        let cached_new = CachedSiteContext::new(&site);
        let cached_from_obj = CachedSiteContext::from_object(site_clone);

        let eng = engine();
        let mut page = Object::new();
        page.insert("title".into(), LiquidValue::scalar("My Page"));
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page));

        let tpl = eng.parse("{{ site.title }} - {{ page.title }}").unwrap();
        let out_new = eng
            .render_with_cached_site(&tpl, &ctx, &cached_new)
            .unwrap();
        let out_from_obj = eng
            .render_with_cached_site(&tpl, &ctx, &cached_from_obj)
            .unwrap();

        assert_eq!(out_new, out_from_obj);
        assert_eq!(out_new, "Test Site - My Page");
    }

    #[test]
    fn test_cached_site_context_from_object_array_access() {
        let mut site = Object::new();
        site.insert(
            "posts".into(),
            LiquidValue::Array(vec![
                {
                    let mut p = Object::new();
                    p.insert("title".into(), LiquidValue::scalar("First"));
                    LiquidValue::Object(p)
                },
                {
                    let mut p = Object::new();
                    p.insert("title".into(), LiquidValue::scalar("Second"));
                    LiquidValue::Object(p)
                },
            ]),
        );

        let cached = CachedSiteContext::from_object(site);

        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(Object::new()));

        let out = eng
            .parse_and_render_with_cached_site(
                "{% for post in site.posts %}{{ post.title }} {% endfor %}",
                &ctx,
                &cached,
            )
            .unwrap();
        assert_eq!(out, "First Second ");
    }

    #[test]
    fn test_date_filter_with_unicode_format() {
        let eng = engine();
        let mut page = Object::new();
        page.insert("date".into(), LiquidValue::scalar("2024-01-15"));
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page));
        // Japanese date format
        let output = eng
            .parse_and_render(
                r#"{{ page.date | date: "%Y\u5e74%m\u6708%d\u65e5" }}"#,
                &ctx,
            )
            .unwrap();
        assert!(
            !output.contains("{{"),
            "Date filter with unicode format should not produce raw Liquid"
        );
    }

    // ========================================================================
    // Issue 542: load_includes_merged
    // ========================================================================

    #[test]
    fn test_load_includes_merged_custom_overrides_default() {
        let dir = tempfile::tempdir().unwrap();
        let default_dir = dir.path().join("_includes");
        let custom_dir = dir.path().join("custom_inc");
        std::fs::create_dir_all(&default_dir).unwrap();
        std::fs::create_dir_all(&custom_dir).unwrap();
        std::fs::write(default_dir.join("a.html"), "default-a").unwrap();
        std::fs::write(custom_dir.join("a.html"), "custom-a").unwrap();
        std::fs::write(default_dir.join("b.html"), "default-b").unwrap();

        let map = load_includes_merged(&default_dir, &custom_dir).unwrap();
        assert_eq!(map.get("a.html").unwrap(), "custom-a");
        assert_eq!(map.get("b.html").unwrap(), "default-b");
    }

    #[test]
    fn test_load_includes_merged_subdirectory_override() {
        let dir = tempfile::tempdir().unwrap();
        let default_dir = dir.path().join("_includes");
        let custom_dir = dir.path().join("custom_inc");
        std::fs::create_dir_all(default_dir.join("sub")).unwrap();
        std::fs::create_dir_all(custom_dir.join("sub")).unwrap();
        std::fs::write(default_dir.join("sub/x.html"), "default-x").unwrap();
        std::fs::write(custom_dir.join("sub/x.html"), "custom-x").unwrap();

        let map = load_includes_merged(&default_dir, &custom_dir).unwrap();
        assert_eq!(map.get("sub/x.html").unwrap(), "custom-x");
    }

    #[test]
    fn test_load_includes_merged_same_dir_no_duplication() {
        let dir = tempfile::tempdir().unwrap();
        let inc_dir = dir.path().join("_includes");
        std::fs::create_dir_all(&inc_dir).unwrap();
        std::fs::write(inc_dir.join("a.html"), "content-a").unwrap();

        let map = load_includes_merged(&inc_dir, &inc_dir).unwrap();
        assert_eq!(map.get("a.html").unwrap(), "content-a");
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_load_includes_merged_unicode_filenames() {
        let dir = tempfile::tempdir().unwrap();
        let default_dir = dir.path().join("_includes");
        let custom_dir = dir.path().join("custom_inc");
        std::fs::create_dir_all(&default_dir).unwrap();
        std::fs::create_dir_all(&custom_dir).unwrap();
        std::fs::write(default_dir.join("ヘッダー.html"), "default-header").unwrap();
        std::fs::write(custom_dir.join("ヘッダー.html"), "custom-header").unwrap();

        let map = load_includes_merged(&default_dir, &custom_dir).unwrap();
        assert_eq!(map.get("ヘッダー.html").unwrap(), "custom-header");
    }

    // ========================================================================
    // Issue 544: render_with_prebuilt_page_lenient optimization tests
    // ========================================================================

    #[test]
    fn test_render_with_prebuilt_page_lenient_matches_cached_site() {
        // Verify that rendering with a pre-built page LenientValue produces
        // the same output as the standard cached site path.
        let eng = engine();
        let template = eng
            .parse("Title: {{ page.title }} by {{ page.author }}")
            .unwrap();

        // Build a page object with front matter
        let mut page = Object::new();
        page.insert("title".into(), LiquidValue::scalar("My Post"));
        page.insert("author".into(), LiquidValue::scalar("Alice"));

        // Standard path: build context, use cached site render
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page.clone()));
        let site = Object::new();
        let cached = CachedSiteContext::from_object(site);

        let standard_output = eng
            .render_with_cached_site(&template, &ctx, &cached)
            .unwrap();

        // Optimized path: pre-build page LenientValue
        let page_value = Value::Object(page);
        let page_lenient = LenientValue::from_value(page_value);
        let optimized_output = eng
            .render_with_prebuilt_page_lenient(&template, &ctx, &cached, &page_lenient)
            .unwrap();

        assert_eq!(standard_output, optimized_output);
        assert_eq!(standard_output, "Title: My Post by Alice");
    }

    #[test]
    fn test_render_with_prebuilt_page_lenient_empty_front_matter() {
        let eng = engine();
        let template = eng.parse("Empty: {{ page.missing }}").unwrap();

        let page = Object::new();
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page.clone()));
        let site = Object::new();
        let cached = CachedSiteContext::from_object(site);

        let standard_output = eng
            .render_with_cached_site(&template, &ctx, &cached)
            .unwrap();

        let page_lenient = LenientValue::from_value(Value::Object(page));
        let optimized_output = eng
            .render_with_prebuilt_page_lenient(&template, &ctx, &cached, &page_lenient)
            .unwrap();

        assert_eq!(standard_output, optimized_output);
    }

    #[test]
    fn test_render_with_prebuilt_page_lenient_unicode_content() {
        let eng = engine();
        let template = eng.parse("{{ page.title }} - {{ page.lang }}").unwrap();

        let mut page = Object::new();
        page.insert("title".into(), LiquidValue::scalar("日本語のタイトル"));
        page.insert("lang".into(), LiquidValue::scalar("ja"));

        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page.clone()));
        let cached = CachedSiteContext::from_object(Object::new());

        let standard = eng
            .render_with_cached_site(&template, &ctx, &cached)
            .unwrap();

        let page_lenient = LenientValue::from_value(Value::Object(page));
        let optimized = eng
            .render_with_prebuilt_page_lenient(&template, &ctx, &cached, &page_lenient)
            .unwrap();

        assert_eq!(standard, optimized);
        assert_eq!(optimized, "日本語のタイトル - ja");
    }

    #[test]
    fn test_render_with_prebuilt_page_lenient_nested_objects() {
        let eng = engine();
        let template = eng.parse("{{ page.meta.description }}").unwrap();

        let mut meta = Object::new();
        meta.insert(
            "description".into(),
            LiquidValue::scalar("A great post about Rust"),
        );
        let mut page = Object::new();
        page.insert("meta".into(), LiquidValue::Object(meta));

        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(page.clone()));
        let cached = CachedSiteContext::from_object(Object::new());

        let standard = eng
            .render_with_cached_site(&template, &ctx, &cached)
            .unwrap();

        let page_lenient = LenientValue::from_value(Value::Object(page));
        let optimized = eng
            .render_with_prebuilt_page_lenient(&template, &ctx, &cached, &page_lenient)
            .unwrap();

        assert_eq!(standard, optimized);
        assert_eq!(optimized, "A great post about Rust");
    }

    #[test]
    fn test_parse_plain_html_no_liquid_markers() {
        // Content with no {{ or {% should still parse and render correctly
        // (this exercises the fast-path when no preprocessing is needed)
        let eng = engine();
        let plain_html = "<h1>Hello World</h1>\n<p>Some plain content with special chars: &amp; \"quotes\" and unicode: Привет</p>";
        let template = eng.parse(plain_html).unwrap();
        let result = template.inner.render(&liquid::Object::new()).unwrap();
        assert_eq!(result, plain_html);
    }

    #[test]
    fn test_parse_content_with_curly_brace_not_liquid() {
        // Content that has { but not {{ or {% should not be treated as Liquid
        let eng = engine();
        let content = "function() { return 42; }";
        let template = eng.parse(content).unwrap();
        let result = template.inner.render(&liquid::Object::new()).unwrap();
        assert_eq!(result, content);
    }

    #[test]
    fn test_preprocess_all_noop_for_plain_content() {
        // Verify that preprocess_all returns the input unchanged for plain content
        let plain = "<div>No liquid here, just HTML with unicode: 日本語</div>";
        let result = preprocess_all(plain);
        assert_eq!(result, plain);
    }

    #[test]
    fn test_preprocess_all_processes_liquid_content() {
        // Verify that preprocess_all still transforms Liquid content correctly
        let content = "{% link _posts/2020-01-01-hello.md %}";
        let result = preprocess_all(content);
        // The link tag should be preprocessed (replaced with a URL)
        assert!(
            !result.contains("{% link"),
            "link tag should be preprocessed"
        );
    }

    #[test]
    fn test_preprocess_all_output_only_skips_tag_preprocessors() {
        // Content with {{ but not {% should skip tag-related preprocessors
        // but still process output-related preprocessors
        let content = "{{ page.title }}";
        let result = preprocess_all(content);
        // Output should be processed (render_mapping filter added)
        assert!(
            result.contains("render_mapping"),
            "output should be preprocessed: got {}",
            result
        );
    }

    #[test]
    fn test_preprocess_all_tag_only_skips_output_preprocessors() {
        // Content with {% but not {{ should skip output-related preprocessors
        let content = "{% if true %}hello{% endif %}";
        let result = preprocess_all(content);
        // Tag content should remain (it's valid Liquid)
        assert!(
            result.contains("if true"),
            "tag content should be preserved"
        );
        // No render_mapping should be added since no {{ }}
        assert!(
            !result.contains("render_mapping"),
            "output preprocessors should be skipped"
        );
    }

    #[test]
    fn test_preprocess_all_unicode_content_preserved() {
        // Ensure Unicode content is not corrupted by preprocessing
        let content = "Привет мир 日本語 🎉 {{ page.title }}";
        let result = preprocess_all(content);
        assert!(
            result.contains("Привет мир"),
            "Cyrillic should be preserved"
        );
        assert!(result.contains("日本語"), "CJK should be preserved");
    }

    // ========================================================================
    // Issue 569: Include whitespace control with mixed dash/non-dash tags
    // ========================================================================

    /// Issue 569: media-url.html include should produce clean URL output
    /// without leading/trailing whitespace from non-dash control flow tags.
    #[test]
    fn test_569_include_whitespace_mixed_dash_tags() {
        let mut includes = std::collections::HashMap::new();
        // Simplified version of chirpy's media-url.html with mixed dash/non-dash tags
        let media_url = r#"{%- comment -%}
  Generate URL
{%- endcomment -%}

{% assign url = include.src %}

{%- if url -%}
  {% unless url contains ':' %}
    {% assign url = include.subpath | default: '' | append: '/' | append: url %}

    {% assign url = url | replace: '///', '/' | replace: '//', '/' | replace: ':/', '://' %}

    {% unless url contains '://' %}
      {% assign url = url %}
    {% endunless %}
  {% endunless %}
{%- endif -%}

{{- url -}}
"#;
        includes.insert("media-url.html".to_string(), media_url.to_string());
        let eng = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        // Use the include directly (without capture), output should be clean
        let template =
            r#"content="{% include "media-url.html" src="/commons/devices-mockup.png" %}""#;
        let out = eng.parse_and_render(template, &ctx).unwrap();
        // The URL should have no leading whitespace
        assert_eq!(
            out, r#"content="/commons/devices-mockup.png""#,
            "Issue 569: Include output should not have leading whitespace in URL. Got: {:?}",
            out,
        );
    }

    /// Issue 569: Simple include with {{- -}} should strip whitespace from
    /// non-dash tags in the include body.
    #[test]
    fn test_569_include_dash_output_strips_runtime_whitespace() {
        let mut includes = std::collections::HashMap::new();
        // Include with non-dash assign followed by dash output
        includes.insert(
            "simple-url.html".to_string(),
            "\n{% assign url = include.src %}\n\n{{- url -}}\n".to_string(),
        );
        let eng = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let template = r#"[{% include "simple-url.html" src="/test.png" %}]"#;
        let out = eng.parse_and_render(template, &ctx).unwrap();
        // {{- url -}} should strip the whitespace from the assign tag too
        assert_eq!(
            out, "[/test.png]",
            "Issue 569: {{- -}} in include should strip preceding whitespace. Got: {:?}",
            out,
        );
    }

    /// Issue 569: Include with nested unless/if blocks and mixed whitespace control.
    /// Tests the full chirpy media-url pattern with Unicode paths.
    #[test]
    fn test_569_include_whitespace_unicode_path() {
        let mut includes = std::collections::HashMap::new();
        let media_url = r#"{%- if include.src -%}
  {% assign url = include.src %}
{%- endif -%}

{{- url -}}
"#;
        includes.insert("url.html".to_string(), media_url.to_string());
        let eng = TemplateEngine::with_includes_map(&includes).unwrap();
        let ctx = Object::new();
        let template = r#"[{% include "url.html" src="/путь/изображение.png" %}]"#;
        let out = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(
            out, "[/путь/изображение.png]",
            "Issue 569: Unicode paths should also have no whitespace. Got: {:?}",
            out,
        );
    }

    /// Issue 569: {{- -}} should NOT strip meaningful spaces from expression output.
    /// Specifically, the trailing space in `' | '` (from append filter) should be
    /// preserved when followed by {{- site.title -}}.
    /// This matches chirpy's title template pattern.
    #[test]
    fn test_569_dash_output_preserves_expression_trailing_space() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("title".into(), LiquidValue::scalar("About"));
        ctx.insert("site_title".into(), LiquidValue::scalar("Chirpy"));
        // Pattern from chirpy's head.html title:
        //   {{- title | append: ' | ' -}}{{- site_title -}}
        let template = "{{- title | append: ' | ' -}}{{- site_title -}}";
        let out = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(
            out, "About | Chirpy",
            "Issue 569: Trailing space from expression output should be preserved. Got: {:?}",
            out,
        );
    }

    /// Issue 569: {{- -}} strips whitespace with newlines from block output
    /// but preserves single spaces from expression output.
    #[test]
    fn test_569_dash_strips_newline_whitespace_but_not_spaces() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("val".into(), LiquidValue::scalar("hello"));
        // Template where non-dash assign produces newline whitespace
        let template = "\n{% assign x = \"world\" %}\n\n{{- val -}}";
        let out = eng.parse_and_render(template, &ctx).unwrap();
        assert_eq!(
            out, "hello",
            "Issue 569: Newline whitespace should be stripped by {{-. Got: {:?}",
            out,
        );

        // Template where expression output has trailing space (no newline)
        let template2 = "{{ val | append: ' ' }}{{- val -}}";
        let out2 = eng.parse_and_render(template2, &ctx).unwrap();
        assert_eq!(
            out2, "hello hello",
            "Issue 569: Trailing space from expression should NOT be stripped by {{-. Got: {:?}",
            out2,
        );
    }

    /// Issue 579: LenientValue wrapping Nil must report is_nil() == true.
    /// Without this, the SEO tag cannot detect that site.url is absent
    /// (Nil) vs explicitly empty (""), causing spurious canonical/og:url
    /// tags on sites that don't configure url: in _config.yml.
    #[test]
    fn test_lenient_value_nil_is_nil() {
        let lv = LenientValue::from_value(Value::Nil);
        assert!(
            lv.is_nil(),
            "LenientValue wrapping Value::Nil must return is_nil() == true"
        );
        assert_eq!(lv.type_name(), "nil");
    }

    /// Issue 579: LenientValue wrapping a non-nil value must NOT report is_nil().
    #[test]
    fn test_lenient_value_scalar_not_nil() {
        let lv = LenientValue::from_value(Value::scalar("hello"));
        assert!(
            !lv.is_nil(),
            "LenientValue wrapping a scalar must not be nil"
        );
        let lv_empty = LenientValue::from_value(Value::scalar(""));
        assert!(
            !lv_empty.is_nil(),
            "LenientValue wrapping empty string must not be nil"
        );
    }

    /// Issue 579: site.url=Nil in CachedSiteContext should render as empty
    /// in templates ({{ site.url }} -> "") but be detectable as nil.
    #[test]
    fn test_cached_site_nil_url_renders_empty() {
        let mut site = Object::new();
        site.insert("url".into(), LiquidValue::Nil);
        site.insert("title".into(), LiquidValue::scalar("Test"));

        let cached = CachedSiteContext::new(&site);
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("page".into(), LiquidValue::Object(Object::new()));

        // {{ site.url }} should render as empty string (not error)
        let tpl = eng.parse("URL:{{ site.url }}:END").unwrap();
        let out = eng.render_with_cached_site(&tpl, &ctx, &cached).unwrap();
        assert_eq!(
            out, "URL::END",
            "Nil site.url should render as empty string in templates"
        );

        // {% if site.url %} should be falsy for nil
        let tpl2 = eng
            .parse("{% if site.url %}HAS_URL{% else %}NO_URL{% endif %}")
            .unwrap();
        let out2 = eng.render_with_cached_site(&tpl2, &ctx, &cached).unwrap();
        assert_eq!(
            out2, "NO_URL",
            "Nil site.url should be falsy in conditionals"
        );
    }
}
