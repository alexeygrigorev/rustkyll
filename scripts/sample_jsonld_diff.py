#!/usr/bin/env -S uv run python
# /// script
# dependencies = ["beautifulsoup4"]
# ///
"""Sample a specific file's JSON-LD to see exact differences."""
import os
import sys
import json
import re
import difflib
from bs4 import BeautifulSoup


def extract_script_texts(html):
    soup = BeautifulSoup(html, "html.parser")
    results = []
    for tag in soup.find_all("script"):
        text = tag.get_text().strip()
        if text and '"@context"' in text:
            results.append(text)
    return results


def main():
    jekyll_dir = "datatalksclub.github.io/_site"
    rustkyll_dir = "_site"
    # Sample a few files
    files = [
        "blog/ai-tools-for-personal-productivity.html",
        "courses/data-engineering-zoomcamp.html",
        "blog/8-newsletters-for-data-science-ai-and-ml-enthusiasts.html",
    ]

    for rel_path in files:
        j_path = os.path.join(jekyll_dir, rel_path)
        r_path = os.path.join(rustkyll_dir, rel_path)
        if not os.path.exists(j_path) or not os.path.exists(r_path):
            continue

        with open(j_path, "r", errors="replace") as f:
            j_scripts = extract_script_texts(f.read())
        with open(r_path, "r", errors="replace") as f:
            r_scripts = extract_script_texts(f.read())

        print(f"\n=== {rel_path} ===")
        print(f"Jekyll scripts: {len(j_scripts)}, Rustkyll scripts: {len(r_scripts)}")

        for idx in range(min(len(j_scripts), len(r_scripts))):
            if j_scripts[idx] != r_scripts[idx]:
                # Try to parse as JSON and diff
                try:
                    j_json = json.loads(j_scripts[idx])
                    r_json = json.loads(r_scripts[idx])
                    j_pretty = json.dumps(j_json, indent=2, sort_keys=True)
                    r_pretty = json.dumps(r_json, indent=2, sort_keys=True)
                    diff = list(difflib.unified_diff(
                        j_pretty.splitlines(), r_pretty.splitlines(),
                        fromfile="jekyll", tofile="rustkyll", lineterm=""
                    ))
                    print(f"\nScript {idx} JSON diff:")
                    for line in diff[:40]:
                        print(f"  {line}")
                    if len(diff) > 40:
                        print(f"  ... and {len(diff) - 40} more lines")
                except json.JSONDecodeError as e:
                    print(f"\nScript {idx} - JSON parse error: {e}")
                    # Show raw text diff
                    j_norm = re.sub(r'\s+', ' ', j_scripts[idx])
                    r_norm = re.sub(r'\s+', ' ', r_scripts[idx])
                    print(f"  Jekyll (normalized, first 200): {j_norm[:200]}")
                    print(f"  Rustkyll (normalized, first 200): {r_norm[:200]}")


if __name__ == "__main__":
    main()
