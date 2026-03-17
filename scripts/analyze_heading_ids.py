#!/usr/bin/env -S uv run python
"""Analyze heading ID attribute differences (double-dash vs single-dash)."""
import re
import sys
from collections import defaultdict


def main():
    with open("docs/comparison/dom-diff-full-report.txt", "r") as f:
        content = f.read()

    # Find attribute_differs lines with id=
    pattern = re.compile(r'  .+?: attribute_differs - expected: "(id=\'(.+?)\')", actual: "(id=\'(.+?)\')"')

    cats = defaultdict(int)
    examples = defaultdict(list)

    for match in pattern.finditer(content):
        j_id = match.group(2)
        r_id = match.group(4)

        # Check if it's double-dash vs single-dash
        if j_id.replace('--', '-') == r_id:
            cats["double-dash-collapsed"] += 1
            if len(examples["double-dash-collapsed"]) < 3:
                examples["double-dash-collapsed"].append(f"{j_id} -> {r_id}")
        else:
            cats["other-id-diff"] += 1
            if len(examples["other-id-diff"]) < 3:
                examples["other-id-diff"].append(f"{j_id} -> {r_id}")

    print("Heading ID attribute diffs:")
    for cat, count in sorted(cats.items(), key=lambda x: -x[1]):
        print(f"  {cat}: {count}")
        for ex in examples[cat]:
            print(f"    e.g. {ex}")


if __name__ == "__main__":
    main()
