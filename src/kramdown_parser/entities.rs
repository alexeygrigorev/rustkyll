// kramdown parser - HTML entity resolution
//
// Based on kramdown by Thomas Leitner (MIT License)
// Copyright (C) 2009-2013 Thomas Leitner <t_leitner@gmx.at>
// See LICENSE-kramdown in this directory for the full license text.
//
// Some test cases based on MDTest by Michel Fortin
// Copyright (c) 2007 Michel Fortin <http://www.michelf.com/>

use std::collections::HashMap;
use std::sync::LazyLock;

/// Lookup table for named HTML entities to their Unicode character(s).
static NAMED_ENTITIES: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    // Common entities used in kramdown
    m.insert("amp", "&");
    m.insert("lt", "<");
    m.insert("gt", ">");
    m.insert("quot", "\"");
    m.insert("apos", "'");
    m.insert("nbsp", "\u{00A0}");
    m.insert("ndash", "\u{2013}");
    m.insert("mdash", "\u{2014}");
    m.insert("hellip", "\u{2026}");
    m.insert("lsquo", "\u{2018}");
    m.insert("rsquo", "\u{2019}");
    m.insert("ldquo", "\u{201C}");
    m.insert("rdquo", "\u{201D}");
    m.insert("laquo", "\u{00AB}");
    m.insert("raquo", "\u{00BB}");
    m.insert("copy", "\u{00A9}");
    m.insert("reg", "\u{00AE}");
    m.insert("trade", "\u{2122}");
    m.insert("times", "\u{00D7}");
    m.insert("divide", "\u{00F7}");
    m.insert("para", "\u{00B6}");
    m.insert("sect", "\u{00A7}");
    m.insert("deg", "\u{00B0}");
    m.insert("plusmn", "\u{00B1}");
    m.insert("micro", "\u{00B5}");
    m.insert("middot", "\u{00B7}");
    m.insert("frac14", "\u{00BC}");
    m.insert("frac12", "\u{00BD}");
    m.insert("frac34", "\u{00BE}");
    m.insert("iexcl", "\u{00A1}");
    m.insert("iquest", "\u{00BF}");
    m.insert("cent", "\u{00A2}");
    m.insert("pound", "\u{00A3}");
    m.insert("yen", "\u{00A5}");
    m.insert("euro", "\u{20AC}");
    m.insert("larr", "\u{2190}");
    m.insert("uarr", "\u{2191}");
    m.insert("rarr", "\u{2192}");
    m.insert("darr", "\u{2193}");
    m.insert("harr", "\u{2194}");
    m.insert("lArr", "\u{21D0}");
    m.insert("uArr", "\u{21D1}");
    m.insert("rArr", "\u{21D2}");
    m.insert("dArr", "\u{21D3}");
    m.insert("hArr", "\u{21D4}");
    m.insert("alpha", "\u{03B1}");
    m.insert("beta", "\u{03B2}");
    m.insert("gamma", "\u{03B3}");
    m.insert("delta", "\u{03B4}");
    m.insert("epsilon", "\u{03B5}");
    m.insert("theta", "\u{03B8}");
    m.insert("lambda", "\u{03BB}");
    m.insert("mu", "\u{03BC}");
    m.insert("pi", "\u{03C0}");
    m.insert("sigma", "\u{03C3}");
    m.insert("omega", "\u{03C9}");
    m.insert("bull", "\u{2022}");
    m.insert("prime", "\u{2032}");
    m.insert("Prime", "\u{2033}");
    m.insert("infin", "\u{221E}");
    m.insert("sum", "\u{2211}");
    m.insert("prod", "\u{220F}");
    m.insert("radic", "\u{221A}");
    m.insert("ne", "\u{2260}");
    m.insert("le", "\u{2264}");
    m.insert("ge", "\u{2265}");
    m.insert("sub", "\u{2282}");
    m.insert("sup", "\u{2283}");
    m.insert("empty", "\u{2205}");
    m.insert("exist", "\u{2203}");
    m.insert("forall", "\u{2200}");
    m.insert("isin", "\u{2208}");
    m.insert("notin", "\u{2209}");
    m.insert("and", "\u{2227}");
    m.insert("or", "\u{2228}");
    m.insert("oplus", "\u{2295}");
    m.insert("otimes", "\u{2297}");
    m.insert("perp", "\u{22A5}");
    m.insert("lceil", "\u{2308}");
    m.insert("rceil", "\u{2309}");
    m.insert("lfloor", "\u{230A}");
    m.insert("rfloor", "\u{230B}");
    m.insert("spades", "\u{2660}");
    m.insert("clubs", "\u{2663}");
    m.insert("hearts", "\u{2665}");
    m.insert("diams", "\u{2666}");
    m.insert("loz", "\u{25CA}");
    m.insert("crarr", "\u{21B5}");
    m.insert("ensp", "\u{2002}");
    m.insert("emsp", "\u{2003}");
    m.insert("thinsp", "\u{2009}");
    m.insert("zwj", "\u{200D}");
    m.insert("zwnj", "\u{200C}");
    m.insert("lrm", "\u{200E}");
    m.insert("rlm", "\u{200F}");
    m.insert("shy", "\u{00AD}");
    m.insert("macr", "\u{00AF}");
    m.insert("acute", "\u{00B4}");
    m.insert("cedil", "\u{00B8}");
    m.insert("uml", "\u{00A8}");
    m.insert("ordf", "\u{00AA}");
    m.insert("ordm", "\u{00BA}");
    m.insert("szlig", "\u{00DF}");
    m.insert("Agrave", "\u{00C0}");
    m.insert("Aacute", "\u{00C1}");
    m.insert("Acirc", "\u{00C2}");
    m.insert("Atilde", "\u{00C3}");
    m.insert("Auml", "\u{00C4}");
    m.insert("Aring", "\u{00C5}");
    m.insert("AElig", "\u{00C6}");
    m.insert("Ccedil", "\u{00C7}");
    m.insert("Egrave", "\u{00C8}");
    m.insert("Eacute", "\u{00C9}");
    m.insert("Ecirc", "\u{00CA}");
    m.insert("Euml", "\u{00CB}");
    m.insert("Igrave", "\u{00CC}");
    m.insert("Iacute", "\u{00CD}");
    m.insert("Icirc", "\u{00CE}");
    m.insert("Iuml", "\u{00CF}");
    m.insert("ETH", "\u{00D0}");
    m.insert("Ntilde", "\u{00D1}");
    m.insert("Ograve", "\u{00D2}");
    m.insert("Oacute", "\u{00D3}");
    m.insert("Ocirc", "\u{00D4}");
    m.insert("Otilde", "\u{00D5}");
    m.insert("Ouml", "\u{00D6}");
    m.insert("Oslash", "\u{00D8}");
    m.insert("Ugrave", "\u{00D9}");
    m.insert("Uacute", "\u{00DA}");
    m.insert("Ucirc", "\u{00DB}");
    m.insert("Uuml", "\u{00DC}");
    m.insert("Yacute", "\u{00DD}");
    m.insert("THORN", "\u{00DE}");
    m.insert("agrave", "\u{00E0}");
    m.insert("aacute", "\u{00E1}");
    m.insert("acirc", "\u{00E2}");
    m.insert("atilde", "\u{00E3}");
    m.insert("auml", "\u{00E4}");
    m.insert("aring", "\u{00E5}");
    m.insert("aelig", "\u{00E6}");
    m.insert("ccedil", "\u{00E7}");
    m.insert("egrave", "\u{00E8}");
    m.insert("eacute", "\u{00E9}");
    m.insert("ecirc", "\u{00EA}");
    m.insert("euml", "\u{00EB}");
    m.insert("igrave", "\u{00EC}");
    m.insert("iacute", "\u{00ED}");
    m.insert("icirc", "\u{00EE}");
    m.insert("iuml", "\u{00EF}");
    m.insert("eth", "\u{00F0}");
    m.insert("ntilde", "\u{00F1}");
    m.insert("ograve", "\u{00F2}");
    m.insert("oacute", "\u{00F3}");
    m.insert("ocirc", "\u{00F4}");
    m.insert("otilde", "\u{00F5}");
    m.insert("ouml", "\u{00F6}");
    m.insert("oslash", "\u{00F8}");
    m.insert("ugrave", "\u{00F9}");
    m.insert("uacute", "\u{00FA}");
    m.insert("ucirc", "\u{00FB}");
    m.insert("uuml", "\u{00FC}");
    m.insert("yacute", "\u{00FD}");
    m.insert("thorn", "\u{00FE}");
    m.insert("yuml", "\u{00FF}");
    m
});

/// Resolve an HTML entity name to its Unicode character(s).
/// Returns `None` if the entity is unknown.
pub fn resolve_named_entity(name: &str) -> Option<&'static str> {
    NAMED_ENTITIES.get(name).copied()
}

/// Resolve a numeric HTML entity (decimal) to a character.
/// Returns `None` if the code point is invalid.
pub fn resolve_numeric_entity(num: u32) -> Option<char> {
    char::from_u32(num)
}

/// Resolve a hex HTML entity to a character.
/// Returns `None` if the code point is invalid.
pub fn resolve_hex_entity(hex: &str) -> Option<char> {
    u32::from_str_radix(hex, 16).ok().and_then(char::from_u32)
}
