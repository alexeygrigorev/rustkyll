#!/usr/bin/env -S uv run python
"""Analyze JSON-LD diffs by comparing actual JSON content of script tags."""
import os
import sys
import json
import re
from collections import defaultdict

sys.path.insert(0, os.path.dirname(__file__))
from dom_compare import parse_and_normalize, find_common_html_files

from bs4 import Tag, NavigableString


def extract_jsonld_scripts(tree):
    """Extract JSON-LD script tag contents."""
    results = []
    if isinstance(tree, Tag):
        for tag in tree.find_all("script"):
            if tag.get("type") == "application/ld+json":
                text = tag.get_text()
                if text.strip():
                    try:
                        data = json.loads(text)
                        results.append(("ld+json", data, text))
                    except:
                        results.append(("ld+json-invalid", None, text))
            else:
                text = tag.get_text()
                if text.strip() and '"@context"' in text:
                    try:
                        data = json.loads(text)
                        results.append(("inline-json", data, text))
                    except:
                        results.append(("inline-json-invalid", None, text))
    return results


def diff_json(j_data, r_data, path=""):
    """Find differences between two JSON objects, returning categories."""
    diffs = []
    if type(j_data) != type(r_data):
        diffs.append(f"type-mismatch at {path}: {type(j_data).__name__} vs {type(r_data).__name__}")
        return diffs

    if isinstance(j_data, dict):
        all_keys = set(list(j_data.keys()) + list(r_data.keys()))
        for key in sorted(all_keys):
            sub_path = f"{path}.{key}" if path else key
            if key not in j_data:
                diffs.append(f"extra-key at {sub_path}")
            elif key not in r_data:
                diffs.append(f"missing-key at {sub_path}")
            else:
                diffs.extend(diff_json(j_data[key], r_data[key], sub_path))
    elif isinstance(j_data, list):
        for i in range(max(len(j_data), len(r_data))):
            sub_path = f"{path}[{i}]"
            if i >= len(j_data):
                diffs.append(f"extra-item at {sub_path}")
            elif i >= len(r_data):
                diffs.append(f"missing-item at {sub_path}")
            else:
                diffs.extend(diff_json(j_data[i], r_data[i], sub_path))
    elif j_data != r_data:
        # Categorize the value diff
        j_str = str(j_data)
        r_str = str(r_data)

        # Timezone offset
        tz_re = re.compile(r'(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})\+(\d{2}:\d{2})')
        j_tz = tz_re.match(j_str)
        r_tz = tz_re.match(r_str)
        if j_tz and r_tz:
            if j_tz.group(1) == r_tz.group(1) and j_tz.group(2) != r_tz.group(2):
                diffs.append(f"timezone-offset at {path}: {j_tz.group(2)} vs {r_tz.group(2)}")
                return diffs

        # null vs empty
        if j_data is None and r_data == "":
            diffs.append(f"null-vs-empty at {path}")
            return diffs

        # Trailing newline
        if isinstance(j_data, str) and isinstance(r_data, str):
            if j_data.rstrip('\n') == r_data.rstrip('\n'):
                diffs.append(f"trailing-newline at {path}")
                return diffs
            if j_data.rstrip() == r_data.rstrip():
                diffs.append(f"trailing-whitespace at {path}")
                return diffs

        diffs.append(f"value-diff at {path}: {j_str[:80]} vs {r_str[:80]}")

    return diffs


def main():
    jekyll_dir = sys.argv[1] if len(sys.argv) > 1 else "datatalksclub.github.io/_site"
    rustkyll_dir = sys.argv[2] if len(sys.argv) > 2 else "_site"

    common_files, _, _ = find_common_html_files(jekyll_dir, rustkyll_dir)

    categories = defaultdict(int)
    examples = defaultdict(list)
    files_with_jsonld_diffs = 0
    total_jsonld_diffs = 0

    for i, rel_path in enumerate(common_files):
        if (i + 1) % 100 == 0:
            print(f"  Progress: {i + 1}/{len(common_files)}...", file=sys.stderr)

        j_path = os.path.join(jekyll_dir, rel_path)
        r_path = os.path.join(rustkyll_dir, rel_path)

        with open(j_path, "r", errors="replace") as f:
            j_html = f.read()
        with open(r_path, "r", errors="replace") as f:
            r_html = f.read()

        j_scripts = extract_jsonld_scripts(parse_and_normalize(j_html))
        r_scripts = extract_jsonld_scripts(parse_and_normalize(r_html))

        if not j_scripts and not r_scripts:
            continue

        file_diffs = []

        # Match scripts by position
        for idx in range(max(len(j_scripts), len(r_scripts))):
            if idx >= len(j_scripts):
                file_diffs.append("extra-script")
                categories["extra-jsonld-script"] += 1
                continue
            if idx >= len(r_scripts):
                file_diffs.append("missing-script")
                categories["missing-jsonld-script"] += 1
                continue

            j_type, j_data, j_text = j_scripts[idx]
            r_type, r_data, r_text = r_scripts[idx]

            if j_data is None or r_data is None:
                categories["parse-error"] += 1
                continue

            diffs = diff_json(j_data, r_data)
            for d in diffs:
                cat = d.split(" at ")[0]
                categories[cat] += 1
                total_jsonld_diffs += 1
                if len(examples[cat]) < 2:
                    examples[cat].append(f"{rel_path}: {d}")

        if file_diffs:
            files_with_jsonld_diffs += 1

    print(f"\nJSON-LD difference subcategories:")
    print(f"{'Category':<30} {'Count':>6}")
    print("-" * 40)
    for cat, count in sorted(categories.items(), key=lambda x: -x[1]):
        print(f"{cat:<30} {count:>6}")
        for ex in examples.get(cat, []):
            print(f"  e.g. {ex[:120]}")
    print(f"\nTotal JSON-LD diffs: {total_jsonld_diffs}")


if __name__ == "__main__":
    main()
