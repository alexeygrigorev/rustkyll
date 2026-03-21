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
        return f"{self.path}: {self.diff_type} - expected: {self.expected!r}, actual: {self.actual!r}"

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
    """Check if an attribute diff is only a trailing newline difference.

    Jekyll's strip_html sometimes preserves a trailing \\n while rustkyll strips it
    (or vice versa). These diffs are cosmetic and should be filtered.
    """
    if diff.diff_type != 'attribute_differs':
        return False
    expected = diff.expected or ''
    actual = diff.actual or ''
    # Try both directions: expected has trailing \n that actual doesn't, or vice versa
    if expected.rstrip('\n') == actual.rstrip('\n') and expected != actual:
        return True
    return False


def filter_acceptable_diffs(diffs: list) -> tuple:
    """Filter out known acceptable differences.

    Returns (remaining_diffs, accepted_diffs).
    """
    remaining = []
    accepted = []
    for d in diffs:
        if is_acceptable_sexagesimal_diff(d) or is_acceptable_date_modified_diff(d) or is_acceptable_build_time_diff(d) or is_acceptable_trailing_newline_diff(d):
            accepted.append(d)
        else:
            remaining.append(d)
    return remaining, accepted


IGNORED_JSONLD_FIELDS = {"dateModified"}


def _is_build_time_only_diff(j_str: str, r_str: str) -> bool:
    """Check if two datetime strings differ only in the time-of-day component.

    Build-time fields like endDate/startDate use the current time when built,
    so Jekyll and rustkyll will always differ by seconds. We consider it
    acceptable if the date and timezone match but the time differs.

    Examples that match:
      '2026-03-21 07:24:03 +0100' vs '2026-03-21 07:24:38 +0100'
      '2026-03-21T07:24:03+01:00' vs '2026-03-21T07:24:38+01:00'
    """
    import re
    # Match datetime patterns: YYYY-MM-DD[T ]HH:MM:SS[timezone]
    pattern = r'^(\d{4}-\d{2}-\d{2})[T ](\d{2}:\d{2}:\d{2})\s*(.*)$'
    j_m = re.match(pattern, j_str.strip())
    r_m = re.match(pattern, r_str.strip())
    if not j_m or not r_m:
        return False
    # Same date and timezone, different time = build-time diff
    return j_m.group(1) == r_m.group(1) and j_m.group(3) == r_m.group(3)


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
            # Skip build-time-only datetime diffs (same date+tz, different time)
            if _is_build_time_only_diff(j_str, r_str):
                pass
            else:
                diffs.append(DiffResult(path, "jsonld_value_differs",
                                        str(j_str)[:200], str(r_str)[:200]))


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
    with open(jekyll_path, "r", encoding="utf-8", errors="replace") as f:
        jekyll_html = f.read()
    with open(rustkyll_path, "r", encoding="utf-8", errors="replace") as f:
        rustkyll_html = f.read()

    jekyll_tree = parse_and_normalize(jekyll_html)
    rustkyll_tree = parse_and_normalize(rustkyll_html)

    return compare_trees(jekyll_tree, rustkyll_tree)


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

        try:
            diffs = compare_html_files(jekyll_path, rustkyll_path)
        except Exception as e:
            diffs = [DiffResult("(parse error)", "error", "", str(e))]

        # Filter out known acceptable differences (e.g. sexagesimal timestamps)
        diffs, accepted = filter_acceptable_diffs(diffs)
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
    summary = f"Summary: {matched} files matched, {differing} files with differences, {total_diffs} total differences"
    if total_accepted > 0:
        summary += f" ({total_accepted} acceptable diffs filtered out)"
    log(summary)

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
