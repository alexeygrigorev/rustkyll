#!/usr/bin/env python3
"""
Parse dom-diff-full-report.txt and categorize all differences.
Outputs a structured summary of diff categories with counts and examples.
"""
import re
import sys
import json
from collections import defaultdict


def categorize_diff(diff_type, expected, actual, path):
    """Determine the high-level category for a diff."""

    # JSON-LD / schema.org script diffs
    if diff_type == "text_differs" and "script" in path:
        exp = expected.strip("'")
        act = actual.strip("'")

        # Timezone differences in JSON-LD dates
        tz_pattern = r'\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\+\d{2}:\d{2}'
        exp_dates = re.findall(tz_pattern, exp)
        act_dates = re.findall(tz_pattern, act)
        if exp_dates and act_dates:
            # Check if only the timezone offset differs
            exp_no_tz = re.sub(r'\+\d{2}:\d{2}', '', exp)
            act_no_tz = re.sub(r'\+\d{2}:\d{2}', '', act)
            if exp_no_tz == act_no_tz:
                return "jsonld-timezone-offset"

        # null vs empty string in JSON-LD
        if '"datePublished": null' in exp and '"datePublished": ""' in act:
            return "jsonld-null-vs-empty"

        # Trailing newlines in descriptions (\n vs \n\n)
        if '\\n"' in exp and '\\n\\n"' in act:
            exp_norm = exp.replace('\\n\\n', '\\n')
            if exp_norm == act.replace('\\n\\n', '\\n'):
                return "jsonld-trailing-newline"
            # Check if only the \n difference matters
            return "jsonld-trailing-newline"

        # Check if it's only trailing newline difference
        if exp.replace('\\n"', '\\n') != act.replace('\\n"', '\\n'):
            # Something else differs too
            pass

        return "jsonld-text-content-diff"

    # Paragraph wrapping differences
    if diff_type == "tag_name_differs":
        if expected == "'p'" or actual == "'p'":
            return "paragraph-wrapping"
        if expected == "'div'" or actual == "'div'":
            return "element-type-mismatch"
        if expected == "'script'" or actual == "'script'":
            return "script-element-mismatch"
        return "tag-name-differs-other"

    # Missing/extra elements
    if diff_type == "missing_element":
        tag = expected.strip("'")
        if tag == "<p>" or tag == "<br>":
            return "missing-paragraph-or-br"
        if tag == "<script>":
            return "missing-script-element"
        if tag == "<div>":
            return "missing-div-element"
        if tag == "<a>":
            return "missing-anchor-element"
        return f"missing-element-{tag.strip('<>/')}"

    if diff_type == "extra_element":
        tag = actual.strip("'")
        if tag == "<p>" or tag == "<br>":
            return "extra-paragraph-or-br"
        if tag == "<script>":
            return "extra-script-element"
        return f"extra-element-{tag.strip('<>/')}"

    # Attribute differences
    if diff_type == "attribute_differs":
        attr_match = re.match(r"(\w+)=", expected.strip("'"))
        if attr_match:
            attr_name = attr_match.group(1)
            if attr_name == "class":
                return "class-attribute-differs"
            if attr_name == "href":
                return "href-attribute-differs"
            if attr_name == "src":
                return "src-attribute-differs"
            if attr_name == "id":
                return "id-attribute-differs"
            return f"attribute-differs-{attr_name}"
        return "attribute-differs-unknown"

    if diff_type == "missing_attribute":
        attr_match = re.match(r"(\w+)=", expected.strip("'"))
        if attr_match:
            attr_name = attr_match.group(1)
            if attr_name == "class":
                return "missing-class-attribute"
            if attr_name == "src":
                return "missing-src-attribute"
            return f"missing-attribute-{attr_name}"
        return "missing-attribute-unknown"

    if diff_type == "extra_attribute":
        attr_match = re.match(r"(\w+)=", actual.strip("'"))
        if attr_match:
            attr_name = attr_match.group(1)
            if attr_name == "class":
                return "extra-class-attribute"
            if attr_name == "type":
                return "extra-type-attribute"
            return f"extra-attribute-{attr_name}"
        return "extra-attribute-unknown"

    # Text differences
    if diff_type == "text_differs":
        return "text-content-differs"

    if diff_type == "missing_text":
        return "missing-text"

    if diff_type == "extra_text":
        return "extra-text"

    if diff_type == "expected_element_got_text":
        return "expected-element-got-text"

    if diff_type == "expected_text_got_element":
        return "expected-text-got-element"

    return f"other-{diff_type}"


def main():
    report_path = sys.argv[1] if len(sys.argv) > 1 else "docs/comparison/dom-diff-full-report.txt"

    with open(report_path, "r") as f:
        lines = f.readlines()

    # Parse diff lines
    categories = defaultdict(lambda: {"count": 0, "examples": [], "files": set()})
    current_file = None

    # More detailed subcategorization for JSON-LD
    jsonld_subcats = defaultdict(int)

    diff_line_re = re.compile(r'^  (.+?): (\w+) - expected: (.+), actual: (.+)$')
    file_re = re.compile(r'^DIFF (.+) \((\d+) differences\)$')

    for line in lines:
        line = line.rstrip('\n')

        file_match = file_re.match(line)
        if file_match:
            current_file = file_match.group(1)
            continue

        diff_match = diff_line_re.match(line)
        if diff_match:
            path = diff_match.group(1)
            diff_type = diff_match.group(2)
            expected = diff_match.group(3)
            actual = diff_match.group(4)

            category = categorize_diff(diff_type, expected, actual, path)

            categories[category]["count"] += 1
            categories[category]["files"].add(current_file)
            if len(categories[category]["examples"]) < 3:
                categories[category]["examples"].append({
                    "file": current_file,
                    "path": path,
                    "expected": expected[:200],
                    "actual": actual[:200],
                })

    # Print summary sorted by count
    print(f"{'Category':<45} {'Count':>6} {'Files':>6}")
    print("-" * 60)

    sorted_cats = sorted(categories.items(), key=lambda x: -x[1]["count"])
    total = 0
    for cat, info in sorted_cats:
        print(f"{cat:<45} {info['count']:>6} {len(info['files']):>6}")
        total += info["count"]

    print("-" * 60)
    print(f"{'TOTAL':<45} {total:>6}")

    # Output JSON for further processing
    output = {}
    for cat, info in sorted_cats:
        output[cat] = {
            "count": info["count"],
            "file_count": len(info["files"]),
            "examples": info["examples"],
        }

    json_path = report_path.replace(".txt", "-categories.json")
    with open(json_path, "w") as f:
        json.dump(output, f, indent=2)
    print(f"\nJSON output: {json_path}", file=sys.stderr)


if __name__ == "__main__":
    main()
