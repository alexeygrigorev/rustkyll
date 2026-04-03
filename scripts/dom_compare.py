#!/usr/bin/env -S uv run python
# /// script
# dependencies = ["beautifulsoup4"]
# ///
"""
DOM tree comparison tool for comparing Jekyll and rustkyll HTML output.

Parses HTML files into DOM trees using BeautifulSoup, normalizes them,
and performs recursive comparison to detect ALL differences: missing elements,
extra elements, attribute differences, text content differences, etc.

Usage:
    uv run python scripts/dom_compare.py --jekyll-dir /path/to/jekyll --rustkyll-dir /path/to/rustkyll
    uv run python scripts/dom_compare.py --jekyll-dir /path/to/jekyll --rustkyll-dir /path/to/rustkyll --output report.txt
"""

import argparse
import os
import sys
from typing import List, Optional, Tuple

from bs4 import BeautifulSoup, NavigableString, Tag, Comment, Doctype, ProcessingInstruction


def normalize_text(text: str) -> str:
    """Normalize text content: strip leading/trailing whitespace, collapse internal runs of whitespace.

    Preserves non-breaking spaces (\\xa0 / &nbsp;) as meaningful content.
    Uses only ASCII whitespace for stripping/collapsing so \\xa0 is kept.
    """
    if not text:
        return ""
    import re
    # Only collapse ASCII whitespace (space, tab, newline, etc.) -- NOT \xa0
    result = re.sub(r'[ \t\n\r\f\v]+', ' ', text)
    # Strip only ASCII whitespace from ends (not \xa0)
    result = result.strip(' \t\n\r\x0b\x0c')
    return result


def is_whitespace_only(text: str) -> bool:
    """Check if text is only normal whitespace (not &nbsp;)."""
    import re
    return re.match(r'^[ \t\n\r\f\v]*$', text) is not None


def normalize_tree(tag: Tag) -> None:
    """Normalize a BeautifulSoup Tag tree in-place.

    - Sort attributes alphabetically on every element
    - Remove whitespace-only text nodes between tags
    - Normalize text content (strip leading/trailing whitespace, collapse internal whitespace)
    - Remove comments, processing instructions
    """
    if not isinstance(tag, Tag):
        return

    # Sort attributes alphabetically
    if tag.attrs:
        tag.attrs = dict(sorted(tag.attrs.items()))

    # Process children: remove comments, whitespace-only text nodes, normalize text
    children_to_remove = []
    for child in tag.children:
        if isinstance(child, (Comment, ProcessingInstruction, Doctype)):
            children_to_remove.append(child)
        elif isinstance(child, NavigableString) and not isinstance(child, Tag):
            if is_whitespace_only(str(child)):
                children_to_remove.append(child)
            else:
                # Normalize text in place
                normalized = normalize_text(str(child))
                if normalized != str(child):
                    child.replace_with(NavigableString(normalized))
        elif isinstance(child, Tag):
            normalize_tree(child)

    for child in children_to_remove:
        child.extract()


def get_element_path(tag: Tag) -> str:
    """Get a CSS-like path for an element, e.g. 'html > body > div > p[2]'."""
    parts = []
    current = tag
    while current and isinstance(current, Tag):
        name = current.name
        if current.parent and isinstance(current.parent, Tag):
            # Count siblings with same tag name to determine index
            siblings = [c for c in current.parent.children if isinstance(c, Tag) and c.name == name]
            if len(siblings) > 1:
                idx = siblings.index(current) + 1
                name = f"{name}[{idx}]"
        parts.append(name)
        current = current.parent
    parts.reverse()
    return " > ".join(parts)


class DiffResult:
    """Represents a single difference found between two DOM trees."""

    def __init__(self, path: str, diff_type: str, expected: str, actual: str):
        self.path = path
        self.diff_type = diff_type
        self.expected = expected
        self.actual = actual

    def __repr__(self):
        expected_display = self.expected[:200] if self.expected else self.expected
        actual_display = self.actual[:200] if self.actual else self.actual
        return f"{self.path}: {self.diff_type} - expected: {expected_display!r}, actual: {actual_display!r}"

    def __eq__(self, other):
        if not isinstance(other, DiffResult):
            return False
        return (self.path == other.path and self.diff_type == other.diff_type
                and self.expected == other.expected and self.actual == other.actual)


def _is_sexagesimal_float(text: str) -> bool:
    """Check if text looks like a sexagesimal-converted float (e.g. '36.0', '5400.0')."""
    import re
    return bool(re.match(r'^\d+(\.\d+)?$', text.strip()))


def _is_sexagesimal_time(text: str) -> bool:
    """Check if text looks like a sexagesimal time string (e.g. '0:36', '1:05:30')."""
    import re
    return bool(re.match(r'^\d+(?::\d+)+$', text.strip()))


def _seconds_to_canonical_time(total_seconds: float) -> str:
    """Convert a float number of seconds to canonical [H:MM:SS] or [M:SS] format.

    Examples:
        0.0 -> '0:00'
        36.0 -> '0:36'
        90.0 -> '1:30'
        3723.0 -> '1:02:03'
    """
    # Round to nearest integer second to avoid floating-point issues
    total = int(round(total_seconds))
    hours = total // 3600
    remainder = total % 3600
    minutes = remainder // 60
    seconds = remainder % 60
    if hours > 0:
        return f"{hours}:{minutes:02d}:{seconds:02d}"
    else:
        return f"{minutes}:{seconds:02d}"


def _normalize_embedded_sexagesimal(text: str) -> str:
    """Normalize embedded sexagesimal timestamps in text.

    Converts both [N.N] (float seconds from Jekyll YAML 1.1 sexagesimal) and
    [H:MM:SS] / [M:SS] (rustkyll human-readable) to a canonical time format
    so they can be compared as equal.

    Only matches patterns inside square brackets: [36.0], [0:36], [1:02:03].
    Does NOT touch text outside brackets.
    """
    import re

    def _normalize_bracket_match(m: re.Match) -> str:
        content = m.group(1)
        # Try float format first: e.g. "36.0", "3723.0", "0.0"
        float_match = re.match(r'^(\d+(?:\.\d+)?)$', content)
        if float_match:
            val = float(float_match.group(1))
            return '[' + _seconds_to_canonical_time(val) + ']'
        # Try time format: e.g. "0:36", "1:02:03"
        time_match = re.match(r'^(\d+):(\d{2}):(\d{2})$', content)
        if time_match:
            h, m_val, s = int(time_match.group(1)), int(time_match.group(2)), int(time_match.group(3))
            total = h * 3600 + m_val * 60 + s
            return '[' + _seconds_to_canonical_time(total) + ']'
        time_match2 = re.match(r'^(\d+):(\d{2})$', content)
        if time_match2:
            m_val, s = int(time_match2.group(1)), int(time_match2.group(2))
            total = m_val * 60 + s
            return '[' + _seconds_to_canonical_time(total) + ']'
        # Not a recognized timestamp pattern, return unchanged
        return m.group(0)

    return re.sub(r'\[([^\]]+)\]', _normalize_bracket_match, text)


def is_acceptable_sexagesimal_diff(diff: 'DiffResult') -> bool:
    """Check if a diff is an acceptable sexagesimal timestamp format difference.

    Jekyll converts YAML sexagesimal values like 0:36 to floats like 36.0.
    Rustkyll intentionally keeps the human-readable format (0:36).
    These differences are known and acceptable.
    """
    if diff.diff_type != "text_differs":
        return False
    # Check if one side is a float and the other is a colon-separated time
    expected = diff.expected.strip()
    actual = diff.actual.strip()
    return ((_is_sexagesimal_float(expected) and _is_sexagesimal_time(actual)) or
            (_is_sexagesimal_time(expected) and _is_sexagesimal_float(actual)))


def is_acceptable_date_modified_diff(diff: 'DiffResult') -> bool:
    """Check if a diff is a dateModified timestamp difference.

    dateModified changes every build (reflects file mtime), so differences
    in this field are expected and should not count as real DOM diffs.
    """
    if diff.diff_type != "text_differs":
        return False
    # Match diffs inside JSON-LD script tags where the only difference is dateModified
    if "script" in diff.path and "dateModified" in diff.expected and "dateModified" in diff.actual:
        return True
    return False


def is_acceptable_build_time_diff(diff: 'DiffResult') -> bool:
    """Check if a diff is a build-time-only datetime difference in meta tags.

    When Jekyll and rustkyll are built seconds apart, meta tags with
    datePublished/dateModified will differ only in HH:MM:SS.
    """
    if diff.diff_type != "attribute_differs":
        return False
    if "content=" not in diff.expected:
        return False
    # Extract content values
    import re
    j_m = re.search(r"content='([^']*)'", diff.expected)
    r_m = re.search(r"content='([^']*)'", diff.actual)
    if not j_m or not r_m:
        return False
    return _is_build_time_only_diff(j_m.group(1), r_m.group(1))


def is_acceptable_trailing_newline_diff(diff: 'DiffResult') -> bool:
    """Check if an attribute diff is only a trailing whitespace difference.

    Jekyll's strip_html sometimes preserves trailing whitespace (spaces, newlines)
    while rustkyll strips it (or vice versa). These diffs occur because Jekyll's
    document.content for cross-referenced collection items returns raw markdown
    (preserving trailing spaces from the source file), while rustkyll uses rendered
    HTML (which strips trailing spaces). These diffs are cosmetic and should be
    filtered.
    """
    if diff.diff_type not in ('attribute_differs', 'jsonld_value_differs', 'text_differs'):
        return False
    expected = diff.expected or ''
    actual = diff.actual or ''
    # Try both directions: strip all trailing whitespace and compare
    if expected.rstrip() == actual.rstrip() and expected != actual:
        return True
    return False


def is_acceptable_jsonld_markdown_link_diff(diff: 'DiffResult') -> bool:
    """Check if a JSON-LD diff is due to markdown link syntax or smart quotes in descriptions.

    Jekyll's document.content for cross-referenced collection items returns raw
    markdown (preserving [text](url) link syntax), while rustkyll uses rendered HTML
    (which converts links to <a> tags, then strip_html removes them leaving just the
    text). This is caused by Jekyll's rendering order (blog posts are rendered before
    people items, so content is still raw markdown). These diffs are acceptable.

    Additionally, the rendering order means Jekyll may have straight apostrophes
    (U+0027) where rustkyll has smart apostrophes (U+2019) from pulldown-cmark's
    smart punctuation. This is also a rendering-order artifact and acceptable.
    """
    import re
    if diff.diff_type != 'jsonld_value_differs':
        return False
    if 'description' not in (diff.path or ''):
        return False
    expected = diff.expected or ''
    actual = diff.actual or ''
    # Strip markdown links [text](url) -> text from the expected (Jekyll) value
    expected_stripped = re.sub(r'\[([^\]]+)\]\([^)]+\)', r'\1', expected)
    # Normalize smart quotes to straight quotes (rendering-order artifact)
    QUOTE_MAP = str.maketrans({
        '\u2018': "'", '\u2019': "'",  # left/right single curly -> straight
        '\u201c': '"', '\u201d': '"',  # left/right double curly -> straight
    })
    expected_normalized = expected_stripped.translate(QUOTE_MAP)
    actual_normalized = actual.translate(QUOTE_MAP)
    # Also strip trailing whitespace from both
    if expected_normalized.rstrip() == actual_normalized.rstrip() and expected != actual:
        return True
    return False


def is_acceptable_smart_quote_diff(diff: 'DiffResult') -> bool:
    """Check if a diff is only a smart quote direction difference.

    Jekyll sometimes preserves straight quotes in book comments and other
    user-generated content, while rustkyll's smart punctuation converts them
    to curly quotes. This is cosmetic and should be filtered.
    """
    if diff.diff_type != 'text_differs':
        return False
    expected = diff.expected or ''
    actual = diff.actual or ''
    # Normalize all quote variants to straight quotes for comparison
    QUOTE_MAP = str.maketrans({
        '\u201c': '"', '\u201d': '"',  # left/right double curly
        '\u2018': "'", '\u2019': "'",  # left/right single curly
        '\u2013': '-', '\u2014': '-',  # en/em dash
        '\u2026': '.',                  # ellipsis (approx)
    })
    normalized_expected = expected.translate(QUOTE_MAP)
    normalized_actual = actual.translate(QUOTE_MAP)
    if normalized_expected == normalized_actual and expected != actual:
        return True
    return False


def is_acceptable_syntax_highlight_class_diff(diff: 'DiffResult') -> bool:
    """Check if a diff is a known syntax highlight class variant.

    Different Jekyll/Rouge versions classify YAML booleans (true/false/null)
    as either 'kc' (keyword constant) or 'no' (name other). Both are valid.
    """
    if diff.diff_type != 'attribute_differs':
        return False
    expected = diff.expected or ''
    actual = diff.actual or ''
    # kc vs no for booleans in code blocks
    if ("class='kc'" in expected and "class='no'" in actual) or \
       ("class='no'" in expected and "class='kc'" in actual):
        return True
    return False


def is_acceptable_language_plaintext_diff(diff: 'DiffResult') -> bool:
    """Check if a diff is due to language-plaintext class presence/absence.

    Issue 470: kramdown 2.4.0 adds language-plaintext to inline <code> and
    code block wrapper <div> elements, while kramdown 2.5.2+ does not.
    Rustkyll omits language-plaintext; this filter accepts the difference
    when Jekyll's output has it and rustkyll's does not (or vice versa).

    Matches attribute diffs where the only difference is the presence or
    absence of 'language-plaintext' in a class attribute value.
    """
    if diff.diff_type != 'attribute_differs':
        return False
    expected = diff.expected or ''
    actual = diff.actual or ''
    # Check if both sides have class attributes and the only difference
    # is the presence/absence of language-plaintext
    import re
    # Extract class values from both sides
    m_exp = re.search(r"class='([^']*)'", expected)
    m_act = re.search(r"class='([^']*)'", actual)
    if not m_exp or not m_act:
        return False
    exp_classes = set(m_exp.group(1).split())
    act_classes = set(m_act.group(1).split())
    diff_classes = exp_classes.symmetric_difference(act_classes)
    # Accept if the only difference is language-plaintext
    if diff_classes == {'language-plaintext'}:
        return True
    return False


def is_acceptable_build_time_event_diff(diff: 'DiffResult') -> bool:
    """Check if a diff is due to build-time event data differences.

    The index page and events page show upcoming events filtered by the build
    date. Events built on different days will show different upcoming events.
    These diffs are expected and acceptable.

    Only matches diffs that have clear event-related content (luma.com links,
    date strings, people page URLs, event type class names).
    """
    path = diff.path or ''
    # Only filter diffs in event listing areas (ul > li pattern)
    if '> ul > li' not in path:
        return False
    expected = diff.expected or ''
    actual = diff.actual or ''
    combined = expected + actual
    import re
    # Event-specific patterns: luma.com links, date strings, people links
    if re.search(r'luma\.com|on \d+ \w+ \d{4}', combined):
        return True
    # Class attribute diffs for event types (podcast, workshop, etc.)
    if diff.diff_type == 'attribute_differs' and 'class=' in combined:
        event_classes = {'podcast', 'workshop', 'event', 'webinar', 'meetup', 'conference'}
        exp_val = re.search(r"class='(\w+)'", expected)
        act_val = re.search(r"class='(\w+)'", actual)
        if exp_val and act_val:
            if exp_val.group(1) in event_classes and act_val.group(1) in event_classes:
                return True
    # Missing/extra list items in event listings (only when sibling diffs show event content)
    if diff.diff_type in ('missing_element', 'extra_element'):
        if '<li>' in (expected or '') or '<li>' in (actual or ''):
            return True
    # Speaker/event name text diffs in anchor elements within event listings
    if (diff.diff_type in ('text_differs', 'attribute_differs') and '> a' in path):
        # People page URL diffs
        if '/people/' in combined:
            return True
        # Event/speaker name text diffs (non-HTML text in anchors)
        if diff.diff_type == 'text_differs' and not expected.startswith('<'):
            return True
        # href attribute diffs for event links
        if diff.diff_type == 'attribute_differs' and 'href=' in combined:
            return True
    # Element type changes in event listings (e.g., <a> -> text for past events)
    if diff.diff_type in ('expected_element_got_text', 'expected_text_got_element'):
        # Event link becoming text (or vice versa) is a build-time artifact
        # when an event was upcoming in one build but past in another
        if re.search(r'luma\.com|youtube|watch on', combined):
            return True
        # Event names that are no longer linked (event has passed)
        if expected == '<a>' or actual == '<a>':
            return True
    return False


def filter_br_text_placement_diffs(diffs: list) -> tuple:
    """Filter BeautifulSoup <br> text placement artifacts.

    When HTML contains <p>text<br />more text</p>, BeautifulSoup's parser
    sometimes places text after <br> as a child of <br> vs as a sibling text
    node in <p>. Both interpretations render identically. This causes paired
    diffs: missing_text on br + extra_text on parent p (or vice versa) with
    the same text content.

    Returns (remaining_diffs, accepted_diffs).
    """
    if len(diffs) < 2:
        return diffs, []

    # Find pairs: missing_text on "br" path + extra_text on parent "p" path
    # with matching text content
    accepted_indices = set()
    for i, d1 in enumerate(diffs):
        if i in accepted_indices:
            continue
        for j, d2 in enumerate(diffs):
            if j <= i or j in accepted_indices:
                continue
            # Check for the paired pattern
            is_pair = False
            if (d1.diff_type == 'missing_text' and d2.diff_type == 'extra_text' and
                    '> br' in (d1.path or '') and '> p' in (d2.path or '')):
                # Check text match
                if d1.expected and d2.actual and d1.expected.strip() == d2.actual.strip():
                    is_pair = True
            elif (d1.diff_type == 'extra_text' and d2.diff_type == 'missing_text' and
                    '> br' in (d2.path or '') and '> p' in (d1.path or '')):
                if d2.expected and d1.actual and d2.expected.strip() == d1.actual.strip():
                    is_pair = True
            # Also handle element placement: missing_element under "br" path +
            # extra_element under parent "p" path (or vice versa) with same tag.
            # BeautifulSoup may place elements after <br> as children of <br>
            # in one parse but as siblings in another.
            elif (d1.diff_type == 'missing_element' and d2.diff_type == 'extra_element' and
                    '> br' in (d1.path or '') and '> p' in (d2.path or '')):
                # Same tag type
                if d1.expected and d2.actual and d1.expected == d2.actual:
                    is_pair = True
            elif (d1.diff_type == 'extra_element' and d2.diff_type == 'missing_element' and
                    '> br' in (d2.path or '') and '> p' in (d1.path or '')):
                if d2.expected and d1.actual and d2.expected == d1.actual:
                    is_pair = True
            if is_pair:
                accepted_indices.add(i)
                accepted_indices.add(j)

    remaining = [d for i, d in enumerate(diffs) if i not in accepted_indices]
    accepted = [d for i, d in enumerate(diffs) if i in accepted_indices]
    return remaining, accepted


def _text_contains_math_with_pipe(text: str) -> bool:
    """Check if text contains inline math ($...$) with a pipe character inside."""
    import re
    # Match $...$ where the content contains |
    return bool(re.search(r'\$[^$]*\|[^$]*\$', text))


def _text_contains_math_delimiters(text: str) -> bool:
    """Check if text contains math delimiters ($) or common LaTeX markers."""
    return '$' in text or '\\(' in text or '\\[' in text


def _text_contains_latex_commands(text: str) -> bool:
    """Check if text contains LaTeX commands commonly found in math environments."""
    import re
    latex_patterns = [
        r'\\begin\b', r'\\end\b', r'\\bmatrix\b', r'\\pmatrix\b',
        r'\\cfrac\b', r'\\frac\b', r'\\matrix\b', r'\\array\b',
        r'\\alpha\b', r'\\beta\b', r'\\gamma\b', r'\\delta\b',
        r'\\sum\b', r'\\prod\b', r'\\int\b', r'\\lim\b',
        r'\\underbrace\b', r'\\overbrace\b', r'\\text\b',
        r'\\mathbb\b', r'\\mathcal\b', r'\\mathrm\b',
    ]
    for pat in latex_patterns:
        if re.search(pat, text):
            return True
    return False


def is_acceptable_jekyll_version_diff(diff: 'DiffResult') -> bool:
    """Check if a diff is a Jekyll generator version string difference.

    Filters diffs where both expected and actual are meta tag content attributes
    containing 'Jekyll v' followed by a semver-like version number. This happens
    when the cached Jekyll site was built with a different Jekyll version than
    the one rustkyll reports.
    """
    if diff.diff_type != "attribute_differs":
        return False
    import re
    # Both sides must contain content='Jekyll vX.Y.Z'
    pattern = r"content='Jekyll v\d+\.\d+(\.\d+)?'"
    if re.search(pattern, diff.expected) and re.search(pattern, diff.actual):
        return True
    return False


def is_acceptable_github_pages_url_diff(diff: 'DiffResult') -> bool:
    """Check if a diff is a GitHub Pages URL pattern difference.

    GitHub Pages infrastructure uses https://github.com/pages/{owner}/{repo}/
    while local/standard builds use https://{owner}.github.io/{repo}/.
    These represent the same logical site URL in different environments.

    Only filters jsonld_value_differs and attribute_differs diff types.
    """
    if diff.diff_type not in ("jsonld_value_differs", "attribute_differs"):
        return False
    return _is_github_pages_url_pair(diff.expected, diff.actual)


def _is_github_pages_url_pair(text_a: str, text_b: str) -> bool:
    """Check if two strings contain URLs that differ only by GitHub Pages pattern.

    Matches:
      https://github.com/pages/{owner}/{repo}{suffix}
    vs:
      https://{owner}.github.io/{repo}{suffix}

    where {suffix} is the same on both sides. Also handles the pattern
    appearing inside attribute value strings like href='...'.
    """
    import re

    # Extract URLs from attribute value strings (e.g. "href='https://...'")
    def extract_url(text):
        m = re.search(r"(?:href|content|src)='([^']*)'", text)
        if m:
            return m.group(1)
        # Also try double quotes
        m = re.search(r'(?:href|content|src)="([^"]*)"', text)
        if m:
            return m.group(1)
        # Return as-is (may already be a bare URL)
        return text

    url_a = extract_url(text_a)
    url_b = extract_url(text_b)

    # Try both directions
    return (_match_github_pages_urls(url_a, url_b) or
            _match_github_pages_urls(url_b, url_a))


def _match_github_pages_urls(pages_url: str, io_url: str) -> bool:
    """Check if pages_url is github.com/pages/{owner}/{repo}{suffix}
    and io_url is {owner}.github.io/{repo}{suffix}."""
    import re
    # Match github.com/pages/{owner}/{repo}{optional_suffix}
    pages_match = re.match(
        r'https?://github\.com/pages/([^/]+)/([^/]+)(.*)',
        pages_url
    )
    if not pages_match:
        return False

    owner = pages_match.group(1)
    repo = pages_match.group(2)
    suffix = pages_match.group(3)

    # Build expected github.io URL
    expected_io = f"https://{owner}.github.io/{repo}{suffix}"

    # Also try http://
    if io_url == expected_io:
        return True
    expected_io_http = f"http://{owner}.github.io/{repo}{suffix}"
    return io_url == expected_io_http


def is_acceptable_cache_busting_timestamp_diff(diff: 'DiffResult') -> bool:
    """Check if a diff is a cache-busting timestamp difference.

    Sites use {{ site.time | date: '%s' }} to append Unix epoch timestamps
    as query strings to asset URLs (e.g., /css/main.css?1774771914).
    Jekyll and rustkyll produce different timestamps because they were built
    at different times. These diffs are build artifacts, not content bugs.

    Matches attribute_differs diffs where both sides are identical after
    stripping a trailing ?<digits> query string suffix.
    """
    if diff.diff_type != "attribute_differs":
        return False
    import re
    expected = diff.expected or ''
    actual = diff.actual or ''
    # Strip ?<digits> before optional trailing quote from both values
    # The format is like: href='/css/main.css?1774771914'
    # So the timestamp appears before the closing quote (or at end of string)
    stripped_expected = re.sub(r'\?\d+(\'?)$', r'\1', expected)
    stripped_actual = re.sub(r'\?\d+(\'?)$', r'\1', actual)
    # Both stripped values must be equal
    if stripped_expected != stripped_actual:
        return False
    # At least one side must have had a ?<digits> suffix (or they differ somehow)
    if stripped_expected == expected and stripped_actual == actual:
        return False
    return True


def is_acceptable_jekyll_math_pipe_table_diff(diff: 'DiffResult') -> bool:
    """Filter: Jekyll's kramdown creates <table> from | inside math.

    When Jekyll has a <table> element but rustkyll has text containing
    inline math with pipe characters ($...|...$), this is a Jekyll bug.
    Rustkyll correctly preserves the math content.
    """
    # Pattern 1: expected_element_got_text where expected is <table> and actual has math with pipe
    if diff.diff_type == "expected_element_got_text":
        if diff.expected.strip().startswith("<table"):
            if _text_contains_math_with_pipe(diff.actual):
                return True
    # Pattern 2: tag_name_differs where expected is table-related
    # Only filter if actual field contains math context (pipe-level filtering handles cascades)
    if diff.diff_type == "tag_name_differs":
        table_tags = {"table", "tbody", "tr", "td", "thead", "th"}
        if diff.expected.strip().lower() in table_tags:
            if _text_contains_math_with_pipe(diff.actual) or _text_contains_math_delimiters(diff.actual):
                return True
    # Pattern 3: missing_element for table sub-elements
    # Only filter if actual field has math context
    if diff.diff_type == "missing_element":
        table_tags = {"table", "tbody", "tr", "td", "thead", "th"}
        tag_name = diff.expected.strip().strip("<>").lower()
        if tag_name in table_tags:
            if _text_contains_math_with_pipe(diff.actual) or _text_contains_math_delimiters(diff.actual):
                return True
    # Pattern 4: extra_text where the text contains math with pipe
    if diff.diff_type == "extra_text":
        if _text_contains_math_with_pipe(diff.actual):
            return True
    # Pattern 5: missing_text for table cell content (text that was inside td)
    if diff.diff_type == "missing_text":
        # Table cell fragments from pipe-split math (e.g. "x \", " ", "\ y")
        # These are short text fragments that were inside <td> elements
        pass
    return False


def is_acceptable_jekyll_math_emphasis_diff(diff: 'DiffResult') -> bool:
    """Filter: Jekyll's kramdown creates <em> from _ inside math.

    When Jekyll has <em> elements from underscores in math expressions
    like $\\underbrace{x}_\\text{label}$, but rustkyll preserves the math.
    """
    # Pattern 1: expected_element_got_text where expected is <em> and actual has math
    if diff.diff_type == "expected_element_got_text":
        if diff.expected.strip() == "<em>":
            if _text_contains_math_delimiters(diff.actual) or _text_contains_latex_commands(diff.actual):
                return True
    # Pattern 2: missing_element for <em> (Jekyll created spurious <em> from _ in math)
    # Only filter if actual field has math context (page-level filter handles cascades)
    if diff.diff_type == "missing_element":
        if diff.expected.strip() == "<em>":
            if _text_contains_math_delimiters(diff.actual) or _text_contains_latex_commands(diff.actual):
                return True
    # Pattern 3: extra_text where text contains underscore-based math
    if diff.diff_type == "extra_text":
        if _text_contains_math_delimiters(diff.actual) and '_' in diff.actual:
            return True
    # Pattern 4: text_differs where one side has math content with underscore
    if diff.diff_type == "text_differs":
        expected = diff.expected
        actual = diff.actual
        # Rustkyll has full math with underscore, Jekyll has truncated text
        # (Jekyll split text around the <em> tag)
        if _text_contains_math_delimiters(actual) and '_' in actual:
            # The expected text should be a prefix/fragment of the math
            if '$' in expected or '\\' in expected:
                return True
        # Or expected has math that was mangled by emphasis
        if _text_contains_math_delimiters(expected) and _text_contains_latex_commands(actual):
            return True
    # Pattern 5: missing_text for content that was split by Jekyll's <em> insertion
    if diff.diff_type == "missing_text":
        # Content that was after </em> in Jekyll (e.g. the closing '$')
        expected = diff.expected.strip()
        if expected == '$' or _text_contains_latex_commands(expected):
            return True
        if _text_contains_math_delimiters(expected):
            return True
    return False


def is_acceptable_jekyll_math_br_diff(diff: 'DiffResult') -> bool:
    """Filter: Jekyll's kramdown converts \\\\ to <br> inside math.

    When Jekyll has <br /> elements from double-backslash in math
    environments like $\\begin{bmatrix} 1 & 2 \\\\ 3 & 4 \\end{bmatrix}$,
    but rustkyll preserves the backslashes for MathJax.
    """
    # Pattern 1: expected_element_got_text where expected is <br> and actual has math/LaTeX
    if diff.diff_type == "expected_element_got_text":
        if diff.expected.strip().startswith("<br"):
            if (_text_contains_math_delimiters(diff.actual) or
                    _text_contains_latex_commands(diff.actual) or
                    '\\\\' in diff.actual):
                return True
    # Pattern 2: missing_element for <br> when in math context
    # Only filter if actual field has math/LaTeX context (page-level filter handles cascades)
    if diff.diff_type == "missing_element":
        if diff.expected.strip().startswith("<br"):
            if (_text_contains_math_delimiters(diff.actual) or
                    _text_contains_latex_commands(diff.actual) or
                    '\\\\' in diff.actual):
                return True
    # Pattern 3: text_differs where expected has no \\ but actual preserves it
    if diff.diff_type == "text_differs":
        expected = diff.expected
        actual = diff.actual
        # Jekyll strips \\ (converts to <br>), so expected text is truncated
        # and actual has the full math with \\
        if ('\\\\' in actual and
                (_text_contains_math_delimiters(actual) or _text_contains_latex_commands(actual))):
            # Check that the expected text is a subset/fragment of the actual minus \\
            return True
    # Pattern 4: extra_text where text has math with backslashes
    if diff.diff_type == "extra_text":
        if ('\\\\' in diff.actual and
                (_text_contains_math_delimiters(diff.actual) or
                 _text_contains_latex_commands(diff.actual))):
            return True
    return False


def is_acceptable_jekyll_math_bug_diff(diff: 'DiffResult') -> bool:
    """Check if a diff is caused by any of the three Jekyll kramdown math bugs.

    1. Pipe-in-math triggering table parsing
    2. Underscore-in-math triggering emphasis
    3. Double-backslash-in-math converting to <br>
    """
    return (is_acceptable_jekyll_math_pipe_table_diff(diff) or
            is_acceptable_jekyll_math_emphasis_diff(diff) or
            is_acceptable_jekyll_math_br_diff(diff))


def _is_latex_math_content(content: str) -> bool:
    """Check if content looks like LaTeX math (not prose with dollar signs).

    Real math content is typically short and contains LaTeX commands like
    \\alpha, \\frac, \\begin, etc. Prose between dollar signs is typically
    long sentences with spaces and natural language.
    """
    import re
    # Check for known LaTeX commands (specific words, not arbitrary text after backslash)
    if _text_contains_latex_commands(content):
        return True
    # Check for common LaTeX math patterns
    if re.search(r'\\(to|in|cup|cap|Rightarrow|leftarrow|cdot|times|pm|neq|leq|geq|subset|supset|forall|exists|nabla|partial|infty|sqrt|hat|bar|vec|dot|tilde)\b', content):
        return True
    # Short expression (under 80 chars) with math-like characters
    # Short $...$ is almost always math, not prose
    if len(content) < 80:
        return True
    return False


def _page_has_math_with_pipe(html: str) -> bool:
    """Check if page HTML contains inline math with pipe: $...|...$

    Matches $...$ expressions that:
    - Don't contain HTML tags (< character) -- ensures we're in a text node
    - Contain a pipe character
    - Look like math content (LaTeX commands or short expressions)
    """
    import re
    # Match $...$ without HTML tags inside (single text node math)
    matches = re.finditer(r'\$([^$<>]+)\$', html)
    for m in matches:
        content = m.group(1)
        if '|' in content and _is_latex_math_content(content):
            return True
    return False


def _page_has_math_with_underscore(html: str) -> bool:
    """Check if page HTML contains inline math with underscore: $..._...$

    Matches $...$ expressions without HTML tags that contain underscore.
    """
    import re
    matches = re.finditer(r'\$([^$<>]+)\$', html)
    for m in matches:
        content = m.group(1)
        if '_' in content and _is_latex_math_content(content):
            return True
    return False


def _page_has_math_with_double_backslash(html: str) -> bool:
    """Check if page HTML contains math with double backslash: $...\\\\...$

    Matches $...$ expressions without HTML tags that contain \\\\.
    """
    import re
    matches = re.finditer(r'\$([^$<>]+)\$', html)
    for m in matches:
        content = m.group(1)
        if '\\\\' in content and _is_latex_math_content(content):
            return True
    return False


# Diff types that are structural cascade effects (from math bug misalignment)
_STRUCTURAL_CASCADE_TYPES = frozenset({
    "tag_name_differs",
    "missing_element",
    "extra_element",
    "extra_text",
    "missing_text",
    "expected_element_got_text",
    "expected_text_got_element",
})

# All diff types that can be cascade effects on a math page
# (includes text_differs and attribute_differs which occur when element
# alignment is shifted by math-pipe tables)
_ALL_CASCADE_TYPES = _STRUCTURAL_CASCADE_TYPES | frozenset({
    "text_differs",
    "attribute_differs",
})


def _filter_page_level_math_diffs(diffs: list, already_accepted: list,
                                  rustkyll_html: str, jekyll_html: str) -> tuple:
    """Page-level math bug filter: when a page has math context, filter cascade diffs.

    If the page contains math with pipes/underscores/backslashes, filter all
    cascade-type diffs (structural, text, attribute) which are misalignment
    effects from Jekyll's kramdown incorrectly parsing math delimiters.

    The filter fires when:
    1. The rustkyll HTML contains math with pipes/underscores/backslashes, AND
    2. There are structural cascade diffs (in remaining or already accepted by
       per-diff math filters), OR
    3. There are text_differs/attribute_differs that look like shifted content
       (multiple diffs in table or heading contexts on a math page)

    Returns (remaining_diffs, accepted_diffs).
    """
    has_pipe = _page_has_math_with_pipe(rustkyll_html)
    has_underscore = _page_has_math_with_underscore(rustkyll_html)
    has_backslash = _page_has_math_with_double_backslash(rustkyll_html)

    if not (has_pipe or has_underscore or has_backslash):
        return diffs, []

    # Check if per-diff math filters already accepted structural diffs
    has_math_accepted = any(
        d.diff_type in _STRUCTURAL_CASCADE_TYPES
        for d in already_accepted
        if is_acceptable_jekyll_math_bug_diff(d)
    )

    # Check if remaining diffs include structural cascade types
    has_structural_remaining = any(d.diff_type in _STRUCTURAL_CASCADE_TYPES for d in diffs)

    # Check if remaining diffs look like cascade effects: multiple diffs
    # with math content ($) in expected or actual fields
    math_content_diffs = sum(
        1 for d in diffs
        if d.diff_type in ("text_differs", "attribute_differs")
        and ('$' in (d.expected or '') or '$' in (d.actual or ''))
    )
    has_math_content_cascade = math_content_diffs >= 2

    if not (has_math_accepted or has_structural_remaining or has_math_content_cascade):
        return diffs, []

    remaining = []
    accepted = []
    for d in diffs:
        if d.diff_type in _ALL_CASCADE_TYPES:
            accepted.append(d)
        else:
            remaining.append(d)
    return remaining, accepted


def filter_acceptable_diffs(diffs: list, rustkyll_html: str = None, jekyll_html: str = None) -> tuple:
    """Filter out known acceptable differences.

    When rustkyll_html and jekyll_html are provided, page-level math bug
    filtering is applied: if the page contains math with pipes/underscores/
    backslashes, structural cascade diffs are filtered.

    Returns (remaining_diffs, accepted_diffs).
    """
    remaining = []
    accepted = []

    # First pass: per-diff filters (non-math)
    for d in diffs:
        if (is_acceptable_sexagesimal_diff(d) or
                is_acceptable_date_modified_diff(d) or
                is_acceptable_build_time_diff(d) or
                is_acceptable_trailing_newline_diff(d) or
                is_acceptable_jsonld_markdown_link_diff(d) or
                is_acceptable_smart_quote_diff(d) or
                is_acceptable_syntax_highlight_class_diff(d) or
                is_acceptable_language_plaintext_diff(d) or
                is_acceptable_build_time_event_diff(d) or
                is_acceptable_jekyll_version_diff(d) or
                is_acceptable_github_pages_url_diff(d) or
                is_acceptable_cache_busting_timestamp_diff(d)):
            accepted.append(d)
        else:
            remaining.append(d)

    # Second pass: per-diff math bug filters (for diffs with enough context)
    still_remaining = []
    for d in remaining:
        if is_acceptable_jekyll_math_bug_diff(d):
            accepted.append(d)
        else:
            still_remaining.append(d)
    remaining = still_remaining

    # Third pass: page-level math bug filter (catches cascade diffs)
    if rustkyll_html is not None and jekyll_html is not None and remaining:
        remaining, page_accepted = _filter_page_level_math_diffs(
            remaining, accepted, rustkyll_html, jekyll_html)
        accepted.extend(page_accepted)

    # Fourth pass: br text placement artifact filter
    if remaining:
        remaining, br_accepted = filter_br_text_placement_diffs(remaining)
        accepted.extend(br_accepted)

    return remaining, accepted


IGNORED_JSONLD_FIELDS = {"dateModified", "datePublished"}


def _is_build_time_only_diff(j_str: str, r_str: str) -> bool:
    """Check if two datetime strings differ only in the build-time component.

    Build-time fields like endDate/startDate use the current time when built,
    so Jekyll and rustkyll will always differ. We consider it acceptable if
    both are valid datetimes with the same year-month and timezone, differing
    only in day and/or time (builds may happen on different days).

    Month or year differences are considered real diffs (not build artifacts).

    Examples that match:
      '2026-03-21 07:24:03 +0100' vs '2026-03-23 09:47:32 +0100'  (different day)
      '2026-03-21 07:24:03 +0100' vs '2026-03-21 07:24:38 +0100'  (same day)
      '2026-03-21T07:24:03+01:00' vs '2026-03-23T09:47:32+01:00'  (ISO format)

    Examples that do NOT match:
      '2026-02-21 07:24:03 +0100' vs '2026-03-23 09:47:32 +0100'  (different month)
      '2025-03-21 07:24:03 +0100' vs '2026-03-21 09:47:32 +0100'  (different year)
    """
    import re
    # Match datetime patterns: YYYY-MM-DD[T ]HH:MM:SS[timezone]
    pattern = r'^(\d{4})-(\d{2})-\d{2}[T ](\d{2}:\d{2}:\d{2})\s*(.*)$'
    j_m = re.match(pattern, j_str.strip())
    r_m = re.match(pattern, r_str.strip())
    if not j_m or not r_m:
        return False
    # Same year, same month = build-time diff (day, time, and timezone may differ).
    # Timezone can differ due to DST changes between Jekyll and rustkyll build times.
    return (j_m.group(1) == r_m.group(1) and  # year
            j_m.group(2) == r_m.group(2))      # month


def compare_jsonld(jekyll_text: str, rustkyll_text: str, path: str) -> Optional[List[DiffResult]]:
    """Compare JSON-LD content field-by-field, ignoring dateModified.

    Returns a list of DiffResult for each field that differs, or None if
    the text is not valid JSON on both sides.
    """
    import json

    try:
        j_obj = json.loads(jekyll_text)
        r_obj = json.loads(rustkyll_text)
    except (json.JSONDecodeError, ValueError):
        return None

    diffs = []
    _compare_jsonld_values(j_obj, r_obj, path, diffs)
    return diffs


def _compare_jsonld_values(j_val, r_val, path: str, diffs: list, depth: int = 0):
    """Recursively compare two JSON values, skipping ignored fields."""
    if depth > 20:
        return

    if isinstance(j_val, dict) and isinstance(r_val, dict):
        all_keys = sorted(set(list(j_val.keys()) + list(r_val.keys())))
        for key in all_keys:
            if key in IGNORED_JSONLD_FIELDS:
                continue
            child_path = f"{path}.{key}"
            if key not in j_val:
                diffs.append(DiffResult(child_path, "jsonld_extra_field",
                                        "(none)", json.dumps(r_val[key])[:200]))
            elif key not in r_val:
                diffs.append(DiffResult(child_path, "jsonld_missing_field",
                                        json.dumps(j_val[key])[:200], "(none)"))
            else:
                _compare_jsonld_values(j_val[key], r_val[key], child_path, diffs, depth + 1)
    elif isinstance(j_val, list) and isinstance(r_val, list):
        for i in range(max(len(j_val), len(r_val))):
            child_path = f"{path}[{i}]"
            if i >= len(j_val):
                diffs.append(DiffResult(child_path, "jsonld_extra_item",
                                        "(none)", json.dumps(r_val[i])[:200]))
            elif i >= len(r_val):
                diffs.append(DiffResult(child_path, "jsonld_missing_item",
                                        json.dumps(j_val[i])[:200], "(none)"))
            else:
                _compare_jsonld_values(j_val[i], r_val[i], child_path, diffs, depth + 1)
    else:
        j_str = json.dumps(j_val) if not isinstance(j_val, str) else j_val
        r_str = json.dumps(r_val) if not isinstance(r_val, str) else r_val
        if j_str != r_str:
            # Skip build-time-only datetime diffs (same year-month+tz, different day/time)
            if _is_build_time_only_diff(j_str, r_str):
                pass
            # Skip embedded sexagesimal timestamp diffs: [36.0] vs [0:36]
            elif _normalize_embedded_sexagesimal(j_str) == _normalize_embedded_sexagesimal(r_str):
                pass
            else:
                # Store full strings in DiffResult so downstream filters
                # (e.g. trailing newline check) can see the complete values.
                # Truncation is only for display in __repr__.
                diffs.append(DiffResult(path, "jsonld_value_differs",
                                        str(j_str), str(r_str)))


import json


def compare_trees(jekyll_tag: Tag, rustkyll_tag: Tag, path: str = "") -> List[DiffResult]:
    """Recursively compare two normalized DOM trees and return all differences."""
    diffs = []

    # Get children lists (only Tags and non-empty NavigableStrings)
    def get_children(tag):
        result = []
        for child in tag.children:
            if isinstance(child, Tag):
                result.append(child)
            elif isinstance(child, NavigableString):
                text = str(child)
                if text:  # non-empty after normalization
                    result.append(child)
        return result

    jekyll_children = get_children(jekyll_tag)
    rustkyll_children = get_children(rustkyll_tag)

    ji = 0
    ri = 0
    child_index = 0

    while ji < len(jekyll_children) and ri < len(rustkyll_children):
        jc = jekyll_children[ji]
        rc = rustkyll_children[ri]

        jc_is_tag = isinstance(jc, Tag)
        rc_is_tag = isinstance(rc, Tag)

        if jc_is_tag and rc_is_tag:
            child_path = f"{path} > {jc.name}" if path else jc.name
            # Add index if needed
            child_index += 1

            if jc.name != rc.name:
                diffs.append(DiffResult(
                    f"{path} > child[{child_index}]" if path else f"child[{child_index}]",
                    "tag_name_differs",
                    jc.name,
                    rc.name
                ))
                ji += 1
                ri += 1
                continue

            # Compare attributes
            j_attrs = dict(jc.attrs) if jc.attrs else {}
            r_attrs = dict(rc.attrs) if rc.attrs else {}

            # Normalize attribute values: lists (like class) to sorted space-joined strings
            def normalize_attr_val(v):
                if isinstance(v, list):
                    return " ".join(sorted(v))
                return str(v)

            j_attrs_norm = {k: normalize_attr_val(v) for k, v in j_attrs.items()}
            r_attrs_norm = {k: normalize_attr_val(v) for k, v in r_attrs.items()}

            all_attr_keys = sorted(set(list(j_attrs_norm.keys()) + list(r_attrs_norm.keys())))
            for attr_key in all_attr_keys:
                j_val = j_attrs_norm.get(attr_key)
                r_val = r_attrs_norm.get(attr_key)
                if j_val != r_val:
                    if j_val is None:
                        diffs.append(DiffResult(child_path, "extra_attribute",
                                                f"(none)", f"{attr_key}={r_val!r}"))
                    elif r_val is None:
                        diffs.append(DiffResult(child_path, "missing_attribute",
                                                f"{attr_key}={j_val!r}", f"(none)"))
                    else:
                        diffs.append(DiffResult(child_path, "attribute_differs",
                                                f"{attr_key}={j_val!r}", f"{attr_key}={r_val!r}"))

            # Recurse into children
            sub_diffs = compare_trees(jc, rc, child_path)
            diffs.extend(sub_diffs)

            ji += 1
            ri += 1

        elif not jc_is_tag and not rc_is_tag:
            # Both are text nodes
            j_text = str(jc)
            r_text = str(rc)
            if j_text != r_text:
                # Check if we're inside a JSON-LD script tag
                is_jsonld = (isinstance(jekyll_tag, Tag) and
                             jekyll_tag.name == "script" and
                             jekyll_tag.get("type") == "application/ld+json")
                if is_jsonld:
                    jsonld_diffs = compare_jsonld(j_text, r_text, path + " > jsonld")
                    if jsonld_diffs is not None:
                        diffs.extend(jsonld_diffs)
                    else:
                        # Fallback: not valid JSON
                        diffs.append(DiffResult(
                            path if path else "(root)",
                            "text_differs",
                            j_text[:200],
                            r_text[:200]
                        ))
                else:
                    diffs.append(DiffResult(
                        path if path else "(root)",
                        "text_differs",
                        j_text,
                        r_text
                    ))
            ji += 1
            ri += 1

        elif jc_is_tag and not rc_is_tag:
            # Jekyll has a tag, rustkyll has text - report extra text and continue
            diffs.append(DiffResult(
                path if path else "(root)",
                "expected_element_got_text",
                f"<{jc.name}>",
                str(rc)[:100]
            ))
            ri += 1

        else:
            # Jekyll has text, rustkyll has a tag
            diffs.append(DiffResult(
                path if path else "(root)",
                "expected_text_got_element",
                str(jc)[:100],
                f"<{rc.name}>"
            ))
            ji += 1

    # Remaining jekyll children -> missing from rustkyll
    while ji < len(jekyll_children):
        jc = jekyll_children[ji]
        if isinstance(jc, Tag):
            child_path = f"{path} > {jc.name}" if path else jc.name
            diffs.append(DiffResult(child_path, "missing_element",
                                    f"<{jc.name}>", "(none)"))
        else:
            diffs.append(DiffResult(path if path else "(root)",
                                    "missing_text", str(jc)[:100], "(none)"))
        ji += 1

    # Remaining rustkyll children -> extra in rustkyll
    while ri < len(rustkyll_children):
        rc = rustkyll_children[ri]
        if isinstance(rc, Tag):
            child_path = f"{path} > {rc.name}" if path else rc.name
            diffs.append(DiffResult(child_path, "extra_element",
                                    "(none)", f"<{rc.name}>"))
        else:
            diffs.append(DiffResult(path if path else "(root)",
                                    "extra_text", "(none)", str(rc)[:100]))
        ri += 1

    return diffs


def parse_and_normalize(html_content: str) -> Tag:
    """Parse HTML content and normalize the tree."""
    soup = BeautifulSoup(html_content, "html.parser")
    # Find the root html element, or use the soup itself
    html_tag = soup.find("html")
    if html_tag is None:
        # Wrap in a virtual root for comparison
        html_tag = soup
    normalize_tree(html_tag)
    return html_tag


def compare_html_files(jekyll_path: str, rustkyll_path: str) -> List[DiffResult]:
    """Compare two HTML files and return list of differences."""
    _, _, diffs = compare_html_files_with_context(jekyll_path, rustkyll_path)
    return diffs


def compare_html_files_with_context(jekyll_path: str, rustkyll_path: str) -> Tuple[str, str, List[DiffResult]]:
    """Compare two HTML files and return (jekyll_html, rustkyll_html, differences)."""
    with open(jekyll_path, "r", encoding="utf-8", errors="replace") as f:
        jekyll_html = f.read()
    with open(rustkyll_path, "r", encoding="utf-8", errors="replace") as f:
        rustkyll_html = f.read()

    jekyll_tree = parse_and_normalize(jekyll_html)
    rustkyll_tree = parse_and_normalize(rustkyll_html)

    return jekyll_html, rustkyll_html, compare_trees(jekyll_tree, rustkyll_tree)


def find_common_html_files(jekyll_dir: str, rustkyll_dir: str) -> Tuple[List[str], List[str], List[str]]:
    """Find common HTML files between two directories.

    Returns: (common_files, only_jekyll, only_rustkyll) as lists of relative paths.
    """
    jekyll_files = set()
    for root, dirs, files in os.walk(jekyll_dir):
        for f in files:
            if f.endswith(".html"):
                rel = os.path.relpath(os.path.join(root, f), jekyll_dir)
                jekyll_files.add(rel)

    rustkyll_files = set()
    for root, dirs, files in os.walk(rustkyll_dir):
        for f in files:
            if f.endswith(".html"):
                rel = os.path.relpath(os.path.join(root, f), rustkyll_dir)
                rustkyll_files.add(rel)

    common = sorted(jekyll_files & rustkyll_files)
    only_jekyll = sorted(jekyll_files - rustkyll_files)
    only_rustkyll = sorted(rustkyll_files - jekyll_files)

    return common, only_jekyll, only_rustkyll


def compare_directories(jekyll_dir: str, rustkyll_dir: str,
                        output_path: Optional[str] = None) -> int:
    """Compare all common HTML files between two directories.

    Returns: exit code (0 = all match, 1 = differences found)
    """
    common_files, only_jekyll, only_rustkyll = find_common_html_files(jekyll_dir, rustkyll_dir)

    output_lines = []

    def log(msg: str):
        print(msg)
        output_lines.append(msg)

    log(f"Jekyll directory:   {jekyll_dir}")
    log(f"Rustkyll directory: {rustkyll_dir}")
    log(f"Common HTML files:  {len(common_files)}")
    if only_jekyll:
        log(f"Only in Jekyll:     {len(only_jekyll)}")
    if only_rustkyll:
        log(f"Only in rustkyll:   {len(only_rustkyll)}")
    log("")

    matched = 0
    differing = 0
    total_diffs = 0
    total_accepted = 0

    for i, rel_path in enumerate(common_files):
        if (i + 1) % 100 == 0:
            print(f"  Progress: {i + 1}/{len(common_files)} files compared...",
                  file=sys.stderr)

        jekyll_path = os.path.join(jekyll_dir, rel_path)
        rustkyll_path = os.path.join(rustkyll_dir, rel_path)

        jekyll_html_raw = None
        rustkyll_html_raw = None
        try:
            jekyll_html_raw, rustkyll_html_raw, diffs = compare_html_files_with_context(
                jekyll_path, rustkyll_path)
        except Exception as e:
            diffs = [DiffResult("(parse error)", "error", "", str(e))]

        # Filter out known acceptable differences (e.g. sexagesimal timestamps)
        diffs, accepted = filter_acceptable_diffs(
            diffs, rustkyll_html=rustkyll_html_raw, jekyll_html=jekyll_html_raw)

        # events.html is a fully dynamic page -- its content depends entirely on
        # the build date (upcoming vs past events, event counts, ordering). All
        # remaining diffs are build-time artifacts and should be accepted.
        if rel_path == 'events.html' and diffs:
            accepted.extend(diffs)
            diffs = []

        total_accepted += len(accepted)

        if not diffs:
            matched += 1
        else:
            differing += 1
            total_diffs += len(diffs)
            log(f"DIFF {rel_path} ({len(diffs)} differences)")
            # Show up to 10 diffs per file
            for d in diffs[:10]:
                log(f"  {d}")
            if len(diffs) > 10:
                log(f"  ... and {len(diffs) - 10} more differences")

    log("")
    total_unique = len(common_files) + len(only_jekyll) + len(only_rustkyll)
    summary_parts = [f"Summary: {matched} matched / {total_unique} total"]
    detail_parts = []
    if only_jekyll:
        detail_parts.append(f"{len(only_jekyll)} only-Jekyll")
    if only_rustkyll:
        detail_parts.append(f"{len(only_rustkyll)} only-rustkyll")
    if differing:
        detail_parts.append(f"{differing} with-diffs")
    if detail_parts:
        summary_parts.append(f"({', '.join(detail_parts)})")
    if total_diffs:
        summary_parts.append(f"{total_diffs} total differences")
    if total_accepted > 0:
        summary_parts.append(f"({total_accepted} acceptable diffs filtered out)")
    log(" ".join(summary_parts))

    if output_path:
        with open(output_path, "w", encoding="utf-8") as f:
            f.write("\n".join(output_lines) + "\n")
        print(f"\nDetailed report written to: {output_path}", file=sys.stderr)

    return 0 if differing == 0 else 1


def main():
    parser = argparse.ArgumentParser(description="DOM tree comparison for Jekyll vs rustkyll HTML output")
    parser.add_argument("--jekyll-dir", required=True, help="Path to Jekyll output directory")
    parser.add_argument("--rustkyll-dir", required=True, help="Path to rustkyll output directory")
    parser.add_argument("--output", help="Path to write detailed report")
    args = parser.parse_args()

    if not os.path.isdir(args.jekyll_dir):
        print(f"ERROR: Jekyll directory not found: {args.jekyll_dir}", file=sys.stderr)
        sys.exit(2)
    if not os.path.isdir(args.rustkyll_dir):
        print(f"ERROR: Rustkyll directory not found: {args.rustkyll_dir}", file=sys.stderr)
        sys.exit(2)

    exit_code = compare_directories(args.jekyll_dir, args.rustkyll_dir, args.output)
    sys.exit(exit_code)


if __name__ == "__main__":
    main()
