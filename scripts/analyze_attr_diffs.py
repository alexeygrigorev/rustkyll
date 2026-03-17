#!/usr/bin/env -S uv run python
"""Analyze attribute_differs in detail."""
import re
import sys
from collections import defaultdict


def main():
    with open("docs/comparison/dom-diff-full-report.txt", "r") as f:
        lines = f.readlines()

    diff_line_re = re.compile(r'^  (.+?): (\w+) - expected: (.+), actual: (.+)$')

    attr_cats = defaultdict(int)
    attr_examples = defaultdict(list)

    for line in lines:
        line = line.rstrip('\n')
        m = diff_line_re.match(line)
        if not m:
            continue

        path = m.group(1)
        diff_type = m.group(2)
        expected = m.group(3)
        actual = m.group(4)

        if diff_type == "attribute_differs":
            # Extract attr name
            attr_match = re.match(r'"?(\w[\w-]*)=', expected.strip("'\""))
            attr_name = attr_match.group(1) if attr_match else "unknown"
            attr_cats[attr_name] += 1
            if len(attr_examples[attr_name]) < 3:
                attr_examples[attr_name].append(f"  {expected} vs {actual}")

        elif diff_type == "missing_attribute":
            attr_match = re.match(r'"?(\w[\w-]*)=', expected.strip("'\""))
            attr_name = attr_match.group(1) if attr_match else "unknown"
            attr_cats[f"missing:{attr_name}"] += 1
            if len(attr_examples[f"missing:{attr_name}"]) < 3:
                attr_examples[f"missing:{attr_name}"].append(f"  {expected}")

        elif diff_type == "extra_attribute":
            attr_match = re.match(r'"?(\w[\w-]*)=', actual.strip("'\""))
            attr_name = attr_match.group(1) if attr_match else "unknown"
            attr_cats[f"extra:{attr_name}"] += 1
            if len(attr_examples[f"extra:{attr_name}"]) < 3:
                attr_examples[f"extra:{attr_name}"].append(f"  {actual}")

    print("Attribute diff categories:")
    print(f"{'Category':<40} {'Count':>6}")
    print("-" * 50)
    for cat, count in sorted(attr_cats.items(), key=lambda x: -x[1]):
        print(f"{cat:<40} {count:>6}")
        for ex in attr_examples[cat][:2]:
            print(f"    {ex}")


if __name__ == "__main__":
    main()
