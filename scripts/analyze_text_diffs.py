#!/usr/bin/env -S uv run python
"""Categorize text_differs entries (excluding JSON-LD which are in script tags)."""
import re
import sys
from collections import defaultdict


def main():
    with open("docs/comparison/dom-diff-full-report.txt", "r") as f:
        lines = f.readlines()

    diff_line_re = re.compile(r'^  (.+?): (text_differs) - expected: (.+), actual: (.+)$')

    cats = defaultdict(int)
    examples = defaultdict(list)
    current_file = None

    for line in lines:
        line = line.rstrip('\n')
        if line.startswith("DIFF "):
            current_file = line.split(" ")[1]
            continue

        m = diff_line_re.match(line)
        if not m:
            continue

        path = m.group(1)
        expected = m.group(3)
        actual = m.group(4)

        # Skip JSON-LD script text diffs
        if "script" in path:
            cats["jsonld-script-text"] += 1
            continue

        # Check if it's in code/pre (syntax highlighting)
        if "pre > code" in path:
            cats["syntax-highlight-text"] += 1
            continue

        # Check if it's in a span within pre>code
        if "code > span" in path:
            cats["syntax-highlight-span-text"] += 1
            continue

        # Check common patterns
        exp_stripped = expected.strip("'\"")
        act_stripped = actual.strip("'\"")

        # Very different text (completely different content = structural mismatch)
        if len(exp_stripped) > 20 and len(act_stripped) > 20:
            # Check similarity
            common_prefix = 0
            for a, b in zip(exp_stripped, act_stripped):
                if a == b:
                    common_prefix += 1
                else:
                    break
            similarity = common_prefix / max(len(exp_stripped), len(act_stripped))
            if similarity < 0.3:
                cats["completely-different-text"] += 1
                if len(examples["completely-different-text"]) < 3:
                    examples["completely-different-text"].append(
                        f"{current_file}: path={path}\n      J: {exp_stripped[:80]}\n      R: {act_stripped[:80]}")
                continue

        # Entity encoding differences (& vs &amp;)
        if "&amp;" in exp_stripped or "&amp;" in act_stripped:
            cats["entity-encoding"] += 1
            if len(examples["entity-encoding"]) < 3:
                examples["entity-encoding"].append(f"{current_file}: {exp_stripped[:80]} vs {act_stripped[:80]}")
            continue

        # Whitespace/newline differences
        if exp_stripped.replace(' ', '') == act_stripped.replace(' ', ''):
            cats["whitespace-diff"] += 1
            continue

        # Non-breaking space
        if '\xa0' in exp_stripped or '\xa0' in act_stripped:
            cats["nbsp-diff"] += 1
            continue

        # Trailing space
        if exp_stripped.rstrip() == act_stripped.rstrip():
            cats["trailing-space"] += 1
            continue

        cats["other-text-diff"] += 1
        if len(examples["other-text-diff"]) < 5:
            examples["other-text-diff"].append(
                f"{current_file}: path={path}\n      J: {exp_stripped[:80]}\n      R: {act_stripped[:80]}")

    print("Text content diff categories:")
    print(f"{'Category':<35} {'Count':>6}")
    print("-" * 45)
    total = 0
    for cat, count in sorted(cats.items(), key=lambda x: -x[1]):
        print(f"{cat:<35} {count:>6}")
        for ex in examples.get(cat, []):
            print(f"    {ex}")
        total += count
    print(f"\nTotal: {total}")


if __name__ == "__main__":
    main()
