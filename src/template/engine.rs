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
            obj.size()
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
        self.inner.size()
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
            "page" => self.page.as_ref().map(|v| v as &dyn ValueView),
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
        let partials = build_partials(&partials_map);
        let parser = Self::builder()
            .tag(super::include_tag::LenientIncludeTag)
            .tag(super::include_tag::LenientIncludeCachedTag)
            .tag(super::seo_tag::SeoTag)
            .tag(super::avatar_tag::AvatarTag)
            .block(super::highlight_tag::HighlightBlock)
            .tag(super::feed_meta_tag::FeedMetaTag)
            .tag(super::noop_tags::GithubEditLinkTag)
            .partials(partials)
            .build()
            .map_err(|e| TemplateError::ParseError(e.to_string()))?;
        Ok(Self {
            parser: RwLock::new(parser),
            includes: Some(partials_map),
            has_include_tag: true,
            passthrough_filters: RwLock::new(HashSet::new()),
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
        let partials = build_partials(includes);
        let parser = Self::builder()
            .tag(super::include_tag::LenientIncludeTag)
            .tag(super::include_tag::LenientIncludeCachedTag)
            .tag(super::seo_tag::SeoTag)
            .tag(super::avatar_tag::AvatarTag)
            .block(super::highlight_tag::HighlightBlock)
            .tag(super::feed_meta_tag::FeedMetaTag)
            .tag(super::noop_tags::GithubEditLinkTag)
            .partials(partials)
            .build()
            .map_err(|e| TemplateError::ParseError(e.to_string()))?;
        Ok(Self {
            parser: RwLock::new(parser),
            includes: Some(includes.clone()),
            has_include_tag: true,
            passthrough_filters: RwLock::new(HashSet::new()),
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
            .filter(liquid_lib::jekyll::ArrayToSentenceString)
            // Custom filters (Issue 07)
            .filter(filters::WhereExp)
            .filter(filters::Where)
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
            // Lenient math filters: non-numeric strings coerce to 0 (Issue 196)
            // Must come after with_stdlib() to override strict versions
            .filter(filters::math::Times)
            .filter(filters::math::Plus)
            .filter(filters::math::Minus)
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
        // Pre-process include paths with subdirectory separators (e.g.,
        // `{% include subdir/file.html %}` -> `{% include "subdir/file.html" %}`).
        // The Liquid parser cannot handle `/` in unquoted tag arguments.
        let preprocessed = super::include_tag::preprocess_include_paths(template_str);
        // Pre-process capture tags to strip extra tokens after the variable name.
        // Jekyll silently ignores extra tokens (e.g., `{% capture var do %}`),
        // but the Liquid parser rejects them.
        let preprocessed = preprocess_capture_tags(&preprocessed);
        // Pre-process Jekyll-specific tags ({% link %}, {% post_url %}) that
        // the Liquid parser does not support. These are replaced with their
        // approximate URL output.
        let preprocessed = preprocess_jekyll_tags(&preprocessed);
        // Pre-process `contains` in if/elsif conditions to add nil guards.
        // Jekyll treats `nil contains "x"` as false, but the liquid crate
        // raises an error. We rewrite `EXPR contains "STR"` to
        // `EXPR and EXPR contains "STR"` so the `and` short-circuits on nil.
        // (Issue 171)
        let preprocessed = preprocess_nil_contains(&preprocessed);
        // Pre-process `== false` comparisons to add nil guards.
        // The Liquid crate treats `nil == false` as true, but Ruby Liquid
        // treats it as false. We rewrite `VAR == false` to
        // `VAR == false and VAR != nil` so nil doesn't match false.
        let preprocessed = preprocess_nil_eq_false(&preprocessed);
        // Pre-process `{{var}}` inside `{% %}` tags. Some themes use
        // `{% if {{include.url}} %}` which is non-standard but tolerated by
        // Jekyll. We strip the inner `{{` / `}}` so the Liquid parser sees
        // a plain variable reference.
        let preprocessed = preprocess_nested_braces(&preprocessed);
        // Pre-process `{% for var in expr | filter %}` to extract the filter
        // chain into a separate assign. Jekyll supports filter chains in for
        // loops but the Liquid crate does not. (Issue 328)
        let preprocessed = preprocess_for_loop_filters(&preprocessed);
        // Pre-process `{% assign var = (expr | filter) %}` to strip parens.
        // Jekyll allows parenthesized expressions but Liquid crate does not.
        // (Issue 328)
        let preprocessed = preprocess_parenthesized_assign(&preprocessed);
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
        builder = builder.tag(super::noop_tags::GithubEditLinkTag);
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
                // Check if the path starts with _ (collection document)
                let is_collection = path.starts_with('_');
                // Strip leading underscore-prefixed directory (e.g., _pages/ -> pages/)
                let url_path = if let Some(stripped) = path.strip_prefix('_') {
                    stripped
                } else {
                    path
                };
                // For collection docs: strip .md or .html extension entirely
                // For root pages: convert .md to .html, keep .html as-is
                let url_path = if let Some(stem) = url_path.strip_suffix(".md") {
                    if is_collection {
                        format!("/{}", stem)
                    } else {
                        format!("/{}.html", stem)
                    }
                } else if let Some(stem) = url_path.strip_suffix(".html") {
                    if is_collection {
                        format!("/{}", stem)
                    } else {
                        format!("/{}", url_path)
                    }
                } else {
                    format!("/{}", url_path)
                };
                result.push_str(&url_path);
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
                    result.push_str(&format!("/{}/{}/{}/{}", year, month, day, title));
                } else {
                    // Fallback: just use slug as path
                    result.push_str(&format!("/{}", slug));
                }
            } else {
                // Not a link/post_url tag, keep original
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

/// Pre-process `== false` / `!= false` comparisons to work around the Liquid
/// crate treating `nil == false` as true (Ruby Liquid returns false).
///
/// Rewrites `VAR == false` to `VAR == false and VAR != nil` so that undefined
/// variables (nil) don't match the literal `false`.
/// Also rewrites `VAR != false` to `VAR != false or VAR == nil` so that nil
/// is correctly treated as "not false" (distinct from false).
fn preprocess_nil_eq_false(template: &str) -> String {
    use regex::Regex;
    use std::sync::LazyLock;

    // Match patterns like: VARIABLE == false or VARIABLE != false
    // VARIABLE is a dotted name like page.toc, site.show_edit, etc.
    static EQ_FALSE_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\b([\w][\w.]*)\s*==\s*false\b").unwrap());
    static NEQ_FALSE_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\b([\w][\w.]*)\s*!=\s*false\b").unwrap());

    let result = EQ_FALSE_RE.replace_all(template, "$1 == false and $1 != nil");
    let result = NEQ_FALSE_RE.replace_all(&result, "$1 != false or $1 == nil");
    result.into_owned()
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

/// Build an `EagerCompiler<InMemorySource>` from a map of partial names to content.
///
/// Each partial's content is pre-processed with the same pipeline as templates:
/// include path quoting, for-loop filter extraction, parenthesized assign stripping,
/// etc. This ensures include files with Jekyll-specific Liquid syntax are handled
/// correctly (Issue 328).
fn build_partials(includes: &HashMap<String, String>) -> EagerCompiler<InMemorySource> {
    let mut partials = EagerCompiler::<InMemorySource>::empty();
    for (name, content) in includes {
        let preprocessed = super::include_tag::preprocess_include_paths(content);
        let preprocessed = preprocess_capture_tags(&preprocessed);
        let preprocessed = preprocess_jekyll_tags(&preprocessed);
        let preprocessed = preprocess_nil_contains(&preprocessed);
        let preprocessed = preprocess_nil_eq_false(&preprocessed);
        let preprocessed = preprocess_nested_braces(&preprocessed);
        let preprocessed = preprocess_for_loop_filters(&preprocessed);
        let preprocessed = preprocess_parenthesized_assign(&preprocessed);
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
        let result = preprocess_jekyll_tags("{% post_url 2022-09-07-homebrew-3.6.0 %}");
        assert_eq!(result, "/2022/09/07/homebrew-3.6.0");
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
    fn test_preprocess_nil_eq_false_neq() {
        let input = r#"{% if page.comments != false %}COMMENTS{% endif %}"#;
        let output = preprocess_nil_eq_false(input);
        assert!(
            output.contains("page.comments != false or page.comments == nil"),
            "Should add nil guard for != false. Got: {}",
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

    #[test]
    fn test_link_tag_no_html_for_collection_pages() {
        // Collection docs (path starts with _) -> extensionless URL
        let result = preprocess_jekyll_tags(r#"<a href="{% link _pages/banners.md %}">Link</a>"#);
        assert_eq!(result, r#"<a href="/pages/banners">Link</a>"#);
    }

    #[test]
    fn test_link_tag_root_page_keeps_html() {
        // Root pages (no _ prefix) -> keep .html extension
        let result = preprocess_jekyll_tags(r#"{% link about.md %}"#);
        assert_eq!(result, "/about.html");
    }

    #[test]
    fn test_link_tag_unicode_collection_doc() {
        // Unicode filenames in collection docs
        let result = preprocess_jekyll_tags(r#"{% link _pages/uber-uns.md %}"#);
        assert_eq!(result, "/pages/uber-uns");
    }

    #[test]
    fn test_link_tag_nested_collection_doc() {
        // Nested paths within collection
        let result = preprocess_jekyll_tags(r#"{% link _notes/2018/my-note.md %}"#);
        assert_eq!(result, "/notes/2018/my-note");
    }

    // ========================================================================
    // Issue 216: link tag resolves .html collection documents
    // ========================================================================

    #[test]
    fn test_link_tag_html_collection_doc_no_extension() {
        // Collection docs with .html extension (path starts with _) -> extensionless URL
        let result =
            preprocess_jekyll_tags(r#"<a href="{% link _pages/issues.html %}">Issues</a>"#);
        assert_eq!(result, r#"<a href="/pages/issues">Issues</a>"#);
    }

    #[test]
    fn test_link_tag_html_root_page_keeps_extension() {
        // Root-level .html files (no _ prefix) -> keep .html extension
        let result = preprocess_jekyll_tags(r#"{% link about.html %}"#);
        assert_eq!(result, "/about.html");
    }

    #[test]
    fn test_link_tag_html_collection_unicode() {
        // Non-ASCII: collection doc with .html extension and German name
        let result = preprocess_jekyll_tags("{% link _pages/\u{00fc}ber-uns.html %}");
        assert_eq!(result, "/pages/\u{00fc}ber-uns");
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
        assert_eq!(out, "<a></a>");
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
                let mut o = Object::new();
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
}
