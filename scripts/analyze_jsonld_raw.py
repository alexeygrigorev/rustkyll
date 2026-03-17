#!/usr/bin/env -S uv run python
# /// script
# dependencies = ["beautifulsoup4"]
# ///
"""Compare raw text of JSON-LD script tags to understand actual differences."""
import os
import sys
import re
from collections import defaultdict

from bs4 import BeautifulSoup, Tag


def extract_script_texts(html):
    """Extract all script tag text content."""
    soup = BeautifulSoup(html, "html.parser")
    results = []
    for tag in soup.find_all("script"):
        text = tag.get_text().strip()
        if text and '"@context"' in text:
            results.append(text)
    return results


def categorize_text_diff(expected, actual):
    """Categorize the difference between two script text contents."""
    cats = []

    # Normalize whitespace to compare
    e_norm = re.sub(r'\s+', ' ', expected).strip()
    a_norm = re.sub(r'\s+', ' ', actual).strip()

    if e_norm == a_norm:
        return ["whitespace-only"]

    # Check timezone
    tz_re = r'(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})\+(\d{2}:\d{2})'
    e_no_tz = re.sub(tz_re, r'\1+00:00', e_norm)
    a_no_tz = re.sub(tz_re, r'\1+00:00', a_norm)

    if e_no_tz != a_no_tz:
        # Trailing newlines
        e_no_tz2 = e_no_tz.replace('\\n\\n"', '\\n"')
        a_no_tz2 = a_no_tz.replace('\\n\\n"', '\\n"')
        if e_no_tz2 != a_no_tz2:
            # null vs empty
            e_no_null = e_no_tz2.replace('"datePublished": null', '"datePublished": ""').replace('"dateModified": null', '"dateModified": ""')
            a_no_null = a_no_tz2
            if e_no_null != a_no_null:
                cats.append("other-content-diff")
            else:
                cats.append("null-vs-empty-date")
        else:
            cats.append("trailing-newline")
        if e_no_tz != e_norm or a_no_tz != a_norm:
            cats.append("timezone-offset")
    else:
        cats.append("timezone-offset-only")

    return cats if cats else ["unknown"]


def main():
    jekyll_dir = sys.argv[1] if len(sys.argv) > 1 else "datatalksclub.github.io/_site"
    rustkyll_dir = sys.argv[2] if len(sys.argv) > 2 else "_site"

    from dom_compare import find_common_html_files
    sys.path.insert(0, os.path.dirname(__file__))

    common_files, _, _ = find_common_html_files(jekyll_dir, rustkyll_dir)

    categories = defaultdict(int)
    cat_files = defaultdict(list)
    total_script_diffs = 0

    for i, rel_path in enumerate(common_files):
        if (i + 1) % 200 == 0:
            print(f"  Progress: {i + 1}/{len(common_files)}...", file=sys.stderr)

        j_path = os.path.join(jekyll_dir, rel_path)
        r_path = os.path.join(rustkyll_dir, rel_path)

        with open(j_path, "r", errors="replace") as f:
            j_scripts = extract_script_texts(f.read())
        with open(r_path, "r", errors="replace") as f:
            r_scripts = extract_script_texts(f.read())

        for idx in range(min(len(j_scripts), len(r_scripts))):
            if j_scripts[idx] != r_scripts[idx]:
                total_script_diffs += 1
                cats = categorize_text_diff(j_scripts[idx], r_scripts[idx])
                key = "+".join(sorted(set(cats)))
                categories[key] += 1
                if len(cat_files[key]) < 2:
                    cat_files[key].append(rel_path)

        if len(j_scripts) != len(r_scripts):
            categories["script-count-mismatch"] += 1

    print(f"\nJSON-LD raw text comparison:")
    print(f"{'Category':<45} {'Count':>6}")
    print("-" * 55)
    for cat, count in sorted(categories.items(), key=lambda x: -x[1]):
        print(f"{cat:<45} {count:>6}")
        for f in cat_files.get(cat, []):
            print(f"    e.g. {f}")
    print(f"\nTotal script text diffs: {total_script_diffs}")


if __name__ == "__main__":
    main()
