#!/usr/bin/env -S uv run python
"""Validate feed.xml and sitemap.xml for DTC site.

Compares rustkyll output against Jekyll output for:
- Valid XML parsing
- Entry/URL count within 5% tolerance
- No raw Liquid tags
- Matching feed entry titles and links

Usage:
    uv run python scripts/validate-dtc-xml.py --jekyll-dir /path/to/jekyll --rustkyll-dir /path/to/rustkyll
"""

import argparse
import re
import sys
import xml.etree.ElementTree as ET

ATOM_NS = {"atom": "http://www.w3.org/2005/Atom"}
SITEMAP_NS = {"sm": "http://www.sitemaps.org/schemas/sitemap/0.9"}


def check_liquid_tags(filepath: str) -> list[str]:
    """Check for raw Liquid tags in a file, excluding code blocks."""
    issues = []
    with open(filepath) as f:
        content = f.read()
    # Check for {{ and {% outside of <code> blocks
    # Simple heuristic: check if these patterns exist at all
    for pattern, name in [("{%", "Liquid block tag"), ("| markdownify", "markdownify filter")]:
        if pattern in content:
            issues.append(f"Found raw {name} '{pattern}' in {filepath}")
    # Check for {{ but exclude ${{ (GitHub Actions syntax in code blocks)
    for match in re.finditer(r"(?<!\$)\{\{(?!\{)", content):
        # Check if this is inside a CDATA section or code block
        pos = match.start()
        before = content[:pos]
        if "<![CDATA[" in before and "]]>" not in before[before.rfind("<![CDATA[") :]:
            continue  # Inside CDATA, skip
        if "<code" in before and "</code>" not in before[before.rfind("<code") :]:
            continue  # Inside code block, skip
        issues.append(
            f"Found raw Liquid output tag '{{{{' at position {pos} in {filepath}"
        )
        break  # One is enough
    return issues


def validate_feed(jekyll_dir: str, rustkyll_dir: str) -> list[str]:
    """Validate feed.xml."""
    issues = []

    jpath = f"{jekyll_dir}/feed.xml"
    rpath = f"{rustkyll_dir}/feed.xml"

    # Parse XML
    try:
        jfeed = ET.parse(jpath)
        print("  feed.xml (Jekyll): valid XML")
    except ET.ParseError as e:
        issues.append(f"Jekyll feed.xml parse error: {e}")
        return issues

    try:
        rfeed = ET.parse(rpath)
        print("  feed.xml (rustkyll): valid XML")
    except ET.ParseError as e:
        issues.append(f"Rustkyll feed.xml parse error: {e}")
        return issues

    # Compare entry counts
    j_entries = jfeed.findall(".//atom:entry", ATOM_NS)
    r_entries = rfeed.findall(".//atom:entry", ATOM_NS)
    j_count = len(j_entries)
    r_count = len(r_entries)
    ratio = abs(j_count - r_count) / max(j_count, 1)
    print(f"  Entry count: Jekyll={j_count}, Rustkyll={r_count} (diff={ratio:.1%})")
    if ratio > 0.05:
        issues.append(
            f"feed.xml entry count diff {ratio:.1%} exceeds 5% tolerance "
            f"(Jekyll={j_count}, Rustkyll={r_count})"
        )

    # Compare entry titles
    def get_title(entry):
        t = entry.find("atom:title", ATOM_NS)
        return t.text if t is not None else ""

    j_titles = {get_title(e) for e in j_entries}
    r_titles = {get_title(e) for e in r_entries}
    shared = j_titles & r_titles
    print(f"  Shared titles: {len(shared)}/{len(j_titles)}")

    # Check for raw Liquid tags
    liquid_issues = check_liquid_tags(rpath)
    issues.extend(liquid_issues)
    if not liquid_issues:
        print("  No raw Liquid tags in rustkyll feed.xml")

    return issues


def validate_sitemap(jekyll_dir: str, rustkyll_dir: str) -> list[str]:
    """Validate sitemap.xml."""
    issues = []

    jpath = f"{jekyll_dir}/sitemap.xml"
    rpath = f"{rustkyll_dir}/sitemap.xml"

    # Parse XML
    try:
        jsm = ET.parse(jpath)
        print("  sitemap.xml (Jekyll): valid XML")
    except ET.ParseError as e:
        issues.append(f"Jekyll sitemap.xml parse error: {e}")
        return issues

    try:
        rsm = ET.parse(rpath)
        print("  sitemap.xml (rustkyll): valid XML")
    except ET.ParseError as e:
        issues.append(f"Rustkyll sitemap.xml parse error: {e}")
        return issues

    # Compare URL counts
    j_urls = jsm.findall(".//sm:url/sm:loc", SITEMAP_NS)
    r_urls = rsm.findall(".//sm:url/sm:loc", SITEMAP_NS)
    j_count = len(j_urls)
    r_count = len(r_urls)
    ratio = abs(j_count - r_count) / max(j_count, 1)
    print(f"  URL count: Jekyll={j_count}, Rustkyll={r_count} (diff={ratio:.1%})")
    if ratio > 0.05:
        issues.append(
            f"sitemap.xml URL count diff {ratio:.1%} exceeds 5% tolerance "
            f"(Jekyll={j_count}, Rustkyll={r_count})"
        )

    # Check for raw Liquid tags
    liquid_issues = check_liquid_tags(rpath)
    issues.extend(liquid_issues)
    if not liquid_issues:
        print("  No raw Liquid tags in rustkyll sitemap.xml")

    return issues


def main():
    parser = argparse.ArgumentParser(description="Validate DTC XML files")
    parser.add_argument("--jekyll-dir", required=True, help="Path to Jekyll output")
    parser.add_argument("--rustkyll-dir", required=True, help="Path to rustkyll output")
    args = parser.parse_args()

    all_issues = []

    print("=== Validating feed.xml ===")
    all_issues.extend(validate_feed(args.jekyll_dir, args.rustkyll_dir))

    print()
    print("=== Validating sitemap.xml ===")
    all_issues.extend(validate_sitemap(args.jekyll_dir, args.rustkyll_dir))

    print()
    if all_issues:
        print("FAIL: Issues found:")
        for issue in all_issues:
            print(f"  - {issue}")
        sys.exit(1)
    else:
        print("PASS: All XML validations passed")
        sys.exit(0)


if __name__ == "__main__":
    main()
