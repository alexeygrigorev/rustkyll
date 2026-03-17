#!/usr/bin/env -S uv run python
"""Check if text diffs are concentrated in files that also have structural diffs."""
import re
from collections import defaultdict


def main():
    with open("docs/comparison/dom-diff-full-report.txt", "r") as f:
        lines = f.readlines()

    file_diffs = defaultdict(lambda: defaultdict(int))
    current_file = None

    diff_line_re = re.compile(r'^  (.+?): (\w+) - expected: (.+), actual: (.+)$')
    file_re = re.compile(r'^DIFF (.+) \((\d+) differences\)$')

    for line in lines:
        line = line.rstrip('\n')
        fm = file_re.match(line)
        if fm:
            current_file = fm.group(1)
            continue

        dm = diff_line_re.match(line)
        if dm:
            diff_type = dm.group(2)
            path = dm.group(1)
            if diff_type == "text_differs" and "script" not in path and "pre > code" not in path and "code > span" not in path:
                file_diffs[current_file]["text"] += 1
            elif diff_type in ("tag_name_differs", "missing_element", "extra_element",
                              "expected_element_got_text", "expected_text_got_element"):
                file_diffs[current_file]["structural"] += 1

    # Files with text diffs but NO structural diffs
    text_only = {f: d for f, d in file_diffs.items() if d.get("text", 0) > 0 and d.get("structural", 0) == 0}
    text_and_structural = {f: d for f, d in file_diffs.items() if d.get("text", 0) > 0 and d.get("structural", 0) > 0}

    text_only_count = sum(d["text"] for d in text_only.values())
    text_and_struct_count = sum(d["text"] for d in text_and_structural.values())

    print(f"Files with ONLY text diffs (no structural): {len(text_only)} files, {text_only_count} text diffs")
    print(f"Files with text AND structural diffs: {len(text_and_structural)} files, {text_and_struct_count} text diffs")

    print(f"\nTop text-only files:")
    for f, d in sorted(text_only.items(), key=lambda x: -x[1]["text"])[:10]:
        print(f"  {f}: {d['text']} text diffs")

    print(f"\nTop text+structural files:")
    for f, d in sorted(text_and_structural.items(), key=lambda x: -x[1]["text"])[:10]:
        print(f"  {f}: {d['text']} text, {d['structural']} structural")


if __name__ == "__main__":
    main()
