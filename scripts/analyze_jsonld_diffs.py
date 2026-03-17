#!/usr/bin/env -S uv run python
"""Analyze JSON-LD specific differences in detail."""
import re
import sys
import json
from collections import defaultdict


def main():
    report_path = sys.argv[1] if len(sys.argv) > 1 else "docs/comparison/dom-diff-full-report.txt"

    with open(report_path, "r") as f:
        content = f.read()

    # Find all text_differs lines for script elements
    # Pattern: body > script: text_differs - expected: '...', actual: '...'
    pattern = re.compile(
        r"  (body > script|.*?script): text_differs - expected: '(.*?)', actual: '(.*?)'$",
        re.MULTILINE
    )

    subcategories = defaultdict(int)

    for match in pattern.finditer(content):
        path = match.group(1)
        expected = match.group(2)
        actual = match.group(3)

        diffs_found = []

        # Check timezone offset
        tz_exp = re.findall(r'(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})\+(\d{2}:\d{2})', expected)
        tz_act = re.findall(r'(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})\+(\d{2}:\d{2})', actual)

        if tz_exp and tz_act:
            exp_offsets = set(tz[1] for tz in tz_exp)
            act_offsets = set(tz[1] for tz in tz_act)
            if exp_offsets != act_offsets:
                diffs_found.append("timezone-offset")

        # Check null vs empty string
        if '"datePublished": null' in expected and '"datePublished": ""' in actual:
            diffs_found.append("null-vs-empty-date")

        # Check trailing newline in description
        if '\\n"' in expected or '\\n\\n"' in actual:
            # Check if \n vs \n\n
            exp_norm = expected.replace('\\n\\n', '\\n')
            act_norm = actual.replace('\\n\\n', '\\n')
            if exp_norm != act_norm:
                # There are other diffs too
                pass
            else:
                diffs_found.append("trailing-newline-only")

            if '\\n\\n"' in actual and '\\n"' in expected:
                diffs_found.append("trailing-newline")

        if not diffs_found:
            # Normalize both and see what remains
            exp_norm = re.sub(r'\+\d{2}:\d{2}', '+00:00', expected)
            act_norm = re.sub(r'\+\d{2}:\d{2}', '+00:00', actual)
            exp_norm = exp_norm.replace('\\n\\n', '\\n')
            act_norm = act_norm.replace('\\n\\n', '\\n')

            if exp_norm == act_norm:
                diffs_found.append("tz+newline-only")
            else:
                diffs_found.append("other-jsonld-diff")

        for d in diffs_found:
            subcategories[d] += 1

    print("JSON-LD subcategories:")
    for cat, count in sorted(subcategories.items(), key=lambda x: -x[1]):
        print(f"  {cat}: {count}")


if __name__ == "__main__":
    main()
