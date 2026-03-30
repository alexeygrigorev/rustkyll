//! Find a `ValueView` nested in an `ObjectView`

use std::fmt;
use std::slice;

use crate::error::Result;

use super::ScalarCow;
use super::Value;
use super::ValueCow;
use super::ValueView;

/// Path to a value in an `Object`.
///
/// There is guaranteed always at least one element.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Path<'s>(Vec<ScalarCow<'s>>);

impl<'s> Path<'s> {
    /// Create a `Value` reference.
    pub fn with_index<I: Into<ScalarCow<'s>>>(value: I) -> Self {
        let indexes = vec![value.into()];
        Path(indexes)
    }

    /// Append an index.
    pub fn push<I: Into<ScalarCow<'s>>>(&mut self, value: I) {
        self.0.push(value.into());
    }

    /// Reserves capacity for at least `additional` more elements to be inserted
    /// in the given `Path`. The `Path` may reserve more space to avoid
    /// frequent reallocations. After calling `reserve`, capacity will be
    /// greater than or equal to `self.len() + additional`. Does nothing if
    /// capacity is already sufficient.
    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }

    /// Access the `Value` reference.
    pub fn iter(&self) -> PathIter<'_, '_> {
        PathIter(self.0.iter())
    }

    /// Extracts a slice containing the entire vector.
    #[inline]
    pub fn as_slice(&self) -> &[ScalarCow<'s>] {
        self.0.as_slice()
    }
}

impl<'s> Extend<ScalarCow<'s>> for Path<'s> {
    fn extend<T: IntoIterator<Item = ScalarCow<'s>>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

impl<'s> ::std::ops::Deref for Path<'s> {
    type Target = [ScalarCow<'s>];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'s> ::std::borrow::Borrow<[ScalarCow<'s>]> for Path<'s> {
    #[inline]
    fn borrow(&self) -> &[ScalarCow<'s>] {
        self
    }
}

impl<'s> AsRef<[ScalarCow<'s>]> for Path<'s> {
    #[inline]
    fn as_ref(&self) -> &[ScalarCow<'s>] {
        self
    }
}

impl fmt::Display for Path<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let data = itertools::join(self.iter().map(ValueView::render), ".");
        write!(f, "{}", data)
    }
}

/// Iterate over indexes in a `Value`'s `Path`.
#[derive(Debug)]
pub struct PathIter<'i, 's>(slice::Iter<'i, ScalarCow<'s>>);

impl<'i, 's: 'i> Iterator for PathIter<'i, 's> {
    type Item = &'i ScalarCow<'s>;

    #[inline]
    fn next(&mut self) -> Option<&'i ScalarCow<'s>> {
        self.0.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }

    #[inline]
    fn count(self) -> usize {
        self.0.count()
    }
}

impl<'i, 's: 'i> ExactSizeIterator for PathIter<'i, 's> {
    #[inline]
    fn len(&self) -> usize {
        self.0.len()
    }
}

/// Find a `ValueView` nested in an `ObjectView`
pub fn try_find<'o>(value: &'o dyn ValueView, path: &[ScalarCow<'_>]) -> Option<ValueCow<'o>> {
    let indexes = path.iter();
    try_find_borrowed(value, indexes)
}

fn try_find_borrowed<'o, 'i>(
    value: &'o dyn ValueView,
    mut path: impl Iterator<Item = &'i ScalarCow<'i>>,
) -> Option<ValueCow<'o>> {
    let index = match path.next() {
        Some(index) => index,
        None => {
            return Some(ValueCow::Borrowed(value));
        }
    };
    let child = augmented_get(value, index)?;
    match child {
        ValueCow::Owned(child) => try_find_owned(child, path),
        ValueCow::Borrowed(child) => try_find_borrowed(child, path),
    }
}

fn try_find_owned<'o, 'i>(
    value: Value,
    mut path: impl Iterator<Item = &'i ScalarCow<'i>>,
) -> Option<ValueCow<'o>> {
    let index = match path.next() {
        Some(index) => index,
        None => {
            return Some(ValueCow::Owned(value));
        }
    };
    let child = augmented_get(&value, index)?;
    match child {
        ValueCow::Owned(child) => try_find_owned(child, path),
        ValueCow::Borrowed(child) => {
            try_find_borrowed(child, path).map(|v| ValueCow::Owned(v.into_owned()))
        }
    }
}

fn augmented_get<'o>(value: &'o dyn ValueView, index: &ScalarCow<'_>) -> Option<ValueCow<'o>> {
    if let Some(arr) = value.as_array() {
        if let Some(index) = index.to_integer() {
            arr.get(index).map(ValueCow::Borrowed)
        } else {
            match &*index.to_kstr() {
                "first" => arr.first().map(ValueCow::Borrowed),
                "last" => arr.last().map(ValueCow::Borrowed),
                "size" => Some(ValueCow::Owned(Value::scalar(arr.size()))),
                _ => None,
            }
        }
    } else if let Some(obj) = value.as_object() {
        let index = index.to_kstr();
        obj.get(index.as_str())
            .map(ValueCow::Borrowed)
            .or_else(|| match index.as_str() {
                "size" => Some(ValueCow::Owned(Value::scalar(obj.size()))),
                "first" => {
                    // Ruby Liquid: hash.first returns the first [key, value] pair.
                    // Respect __key_order if present for consistent iteration order.
                    let first_key = obj
                        .get("__key_order")
                        .and_then(|v| v.as_array())
                        .and_then(|arr| arr.first())
                        .map(|v| v.to_kstr().to_string())
                        .or_else(|| {
                            obj.keys()
                                .find(|k| k.as_str() != "__key_order")
                                .map(|k| k.to_string())
                        });
                    first_key.and_then(|key| {
                        obj.get(&key).map(|val| {
                            ValueCow::Owned(Value::Array(vec![Value::scalar(key), val.to_value()]))
                        })
                    })
                }
                "last" => {
                    // Ruby Liquid: hash.last returns the last [key, value] pair.
                    let last_key = obj
                        .get("__key_order")
                        .and_then(|v| v.as_array())
                        .and_then(|arr| arr.last())
                        .map(|v| v.to_kstr().to_string())
                        .or_else(|| {
                            obj.keys()
                                .filter(|k| k.as_str() != "__key_order")
                                .last()
                                .map(|k| k.to_string())
                        });
                    last_key.and_then(|key| {
                        obj.get(&key).map(|val| {
                            ValueCow::Owned(Value::Array(vec![Value::scalar(key), val.to_value()]))
                        })
                    })
                }
                _ => None,
            })
    } else if let Some(scalar) = value.as_scalar() {
        let index = index.to_kstr();
        match index.as_str() {
            "size" => Some(ValueCow::Owned(Value::scalar(
                scalar.to_kstr().as_str().len() as i64,
            ))),
            _ => None,
        }
    } else {
        None
    }
}

/// Find a `ValueView` nested in an `ObjectView`
///
/// Returns nil (as an owned `Value::Nil`) when a path element cannot be
/// resolved.  This matches Ruby Liquid's behavior where accessing a
/// missing key or an out-of-bounds array index silently evaluates to nil
/// rather than raising an error.
pub fn find<'o>(value: &'o dyn ValueView, path: &[ScalarCow<'_>]) -> Result<ValueCow<'o>> {
    if let Some(res) = try_find(value, path) {
        Ok(res)
    } else {
        // Ruby Liquid returns nil for any unresolvable path (missing key,
        // out-of-bounds array index, indexing into a scalar, etc.).
        // Return Nil instead of an error so templates like jekyll-toc that
        // do `array[1]` after a `split` don't abort the whole render.
        Ok(ValueCow::Owned(Value::Nil))
    }
}
