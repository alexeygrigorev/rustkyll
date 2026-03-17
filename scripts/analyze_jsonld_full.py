#!/usr/bin/env -S uv run python
"""Full JSON-LD diff analysis with proper subcategorization."""
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


def categorize_json_diff(j_json, r_json, path=""):
    """Recursively find and categorize all differences."""
    cats = []
    if type(j_json) != type(r_json):
        if j_json is None and r_json == "":
            cats.append("null-vs-empty")
        elif isinstance(j_json, str) and isinstance(r_json, (int, float)):
            cats.append("string-vs-number")
        elif isinstance(j_json, (int, float)) and isinstance(r_json, str):
            cats.append("number-vs-string")
        else:
            cats.append(f"type-mismatch({type(j_json).__name__}-vs-{type(r_json).__name__})")
        return cats

    if isinstance(j_json, dict):
        for k in set(list(j_json.keys()) + list(r_json.keys())):
            if k not in j_json:
                cats.append(f"extra-key:{k}")
            elif k not in r_json:
                cats.append(f"missing-key:{k}")
            else:
                cats.extend(categorize_json_diff(j_json[k], r_json[k], f"{path}.{k}"))
    elif isinstance(j_json, list):
        for i in range(max(len(j_json), len(r_json))):
            if i >= len(j_json):
                cats.append("extra-array-item")
            elif i >= len(r_json):
                cats.append("missing-array-item")
            else:
                cats.extend(categorize_json_diff(j_json[i], r_json[i], f"{path}[{i}]"))
    elif j_json != r_json:
        j_str = str(j_json)
        r_str = str(r_json)
        # Check timezone offset
        tz = re.compile(r'^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})\+(\d{2}:\d{2})$')
        jm = tz.match(j_str)
        rm = tz.match(r_str)
        if jm and rm and jm.group(1) == rm.group(1):
            cats.append("timezone-offset")
        elif isinstance(j_json, str) and isinstance(r_json, str):
            if j_json.rstrip('\n') == r_json.rstrip('\n'):
                cats.append("trailing-newline")
            elif j_json.rstrip() == r_json.rstrip():
                cats.append("trailing-whitespace")
            else:
                cats.append(f"string-value-diff")
        else:
            cats.append("value-diff")
    return cats


def main():
    jekyll_dir = sys.argv[1] if len(sys.argv) > 1 else "datatalksclub.github.io/_site"
    rustkyll_dir = sys.argv[2] if len(sys.argv) > 2 else "_site"

    sys.path.insert(0, os.path.dirname(__file__))
    from dom_compare import find_common_html_files
    common_files, _, _ = find_common_html_files(jekyll_dir, rustkyll_dir)

    cats = defaultdict(int)
    cat_examples = defaultdict(list)
    script_count_mismatches = 0

    for i, rel_path in enumerate(common_files):
        if (i + 1) % 200 == 0:
            print(f"  Progress: {i + 1}/{len(common_files)}...", file=sys.stderr)

        j_path = os.path.join(jekyll_dir, rel_path)
        r_path = os.path.join(rustkyll_dir, rel_path)

        with open(j_path, "r", errors="replace") as f:
            j_scripts = extract_scripts(f.read())
        with open(r_path, "r", errors="replace") as f:
            r_scripts = extract_scripts(f.read())

        if len(j_scripts) != len(r_scripts):
            diff = len(r_scripts) - len(j_scripts)
            if diff > 0:
                cats["extra-jsonld-script-tag"] += diff
            else:
                cats["missing-jsonld-script-tag"] += abs(diff)
            script_count_mismatches += 1

        for idx in range(min(len(j_scripts), len(r_scripts))):
            if j_scripts[idx] == r_scripts[idx]:
                continue
            try:
                j_json = json.loads(j_scripts[idx])
                r_json = json.loads(r_scripts[idx])
                diff_cats = categorize_json_diff(j_json, r_json)
                for c in diff_cats:
                    cats[c] += 1
                    if len(cat_examples[c]) < 2:
                        cat_examples[c].append(rel_path)
            except json.JSONDecodeError:
                cats["json-parse-error"] += 1

    print(f"\nJSON-LD subcategory analysis:")
    print(f"{'Category':<40} {'Count':>6}")
    print("-" * 50)
    total = 0
    for cat, count in sorted(cats.items(), key=lambda x: -x[1]):
        print(f"{cat:<40} {count:>6}")
        for ex in cat_examples.get(cat, []):
            print(f"    e.g. {ex}")
        total += count
    print("-" * 50)
    print(f"{'TOTAL':<40} {total:>6}")
    print(f"Files with script count mismatch: {script_count_mismatches}")


if __name__ == "__main__":
    main()
