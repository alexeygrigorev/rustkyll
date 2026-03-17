#!/usr/bin/env -S uv run python
# /// script
# dependencies = ["beautifulsoup4"]
# ///
"""Analyze what causes 'string-value-diff' in JSON-LD."""
import os
import sys
import json
import re
from collections import defaultdict
from bs4 import BeautifulSoup


def extract_scripts(html):
    soup = BeautifulSoup(html, "html.parser")
    results = []
    for tag in soup.find_all("script"):
        text = tag.get_text().strip()
        if text and '"@context"' in text:
            results.append(text)
    return results


def find_string_diffs(j_json, r_json, path=""):
    """Find all string value differences."""
    diffs = []
    if type(j_json) != type(r_json):
        return diffs
    if isinstance(j_json, dict):
        for k in set(list(j_json.keys()) + list(r_json.keys())):
            if k in j_json and k in r_json:
                diffs.extend(find_string_diffs(j_json[k], r_json[k], f"{path}.{k}"))
    elif isinstance(j_json, list):
        for i in range(min(len(j_json), len(r_json))):
            diffs.extend(find_string_diffs(j_json[i], r_json[i], f"{path}[{i}]"))
    elif isinstance(j_json, str) and isinstance(r_json, str):
        if j_json != r_json and j_json.rstrip('\n') != r_json.rstrip('\n'):
            # Not a trailing newline diff - it's a real string diff
            tz = re.compile(r'^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})\+(\d{2}:\d{2})$')
            if not (tz.match(j_json) and tz.match(r_json)):
                diffs.append((path, j_json[:150], r_json[:150]))
    return diffs


def main():
    jekyll_dir = "datatalksclub.github.io/_site"
    rustkyll_dir = "_site"

    sys.path.insert(0, os.path.dirname(__file__))
    from dom_compare import find_common_html_files
    common_files, _, _ = find_common_html_files(jekyll_dir, rustkyll_dir)

    subcats = defaultdict(int)
    subcat_examples = defaultdict(list)

    for i, rel_path in enumerate(common_files):
        j_path = os.path.join(jekyll_dir, rel_path)
        r_path = os.path.join(rustkyll_dir, rel_path)

        with open(j_path, "r", errors="replace") as f:
            j_scripts = extract_scripts(f.read())
        with open(r_path, "r", errors="replace") as f:
            r_scripts = extract_scripts(f.read())

        for idx in range(min(len(j_scripts), len(r_scripts))):
            if j_scripts[idx] == r_scripts[idx]:
                continue
            try:
                j_json = json.loads(j_scripts[idx])
                r_json = json.loads(r_scripts[idx])
                diffs = find_string_diffs(j_json, r_json)
                for path, j_val, r_val in diffs:
                    # Categorize
                    field = path.split(".")[-1] if "." in path else path
                    subcats[field] += 1
                    if len(subcat_examples[field]) < 3:
                        subcat_examples[field].append({
                            "file": rel_path,
                            "path": path,
                            "jekyll": j_val[:100],
                            "rustkyll": r_val[:100],
                        })
            except:
                pass

    print(f"\nJSON-LD string-value-diff by field:")
    print(f"{'Field':<30} {'Count':>6}")
    print("-" * 40)
    for field, count in sorted(subcats.items(), key=lambda x: -x[1]):
        print(f"{field:<30} {count:>6}")
        for ex in subcat_examples.get(field, [])[:1]:
            print(f"    {ex['file']}")
            print(f"    Jekyll:   {ex['jekyll'][:100]}")
            print(f"    Rustkyll: {ex['rustkyll'][:100]}")


if __name__ == "__main__":
    main()
