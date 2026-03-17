#!/usr/bin/env -S uv run python
"""
Full DOM comparison - shows ALL diffs (not limited to 10 per file).
Wraps dom_compare.py logic but removes the per-file limit.
"""
import os
import sys

# Add parent to path so we can import
sys.path.insert(0, os.path.dirname(__file__))
from dom_compare import (
    find_common_html_files,
    compare_html_files,
    filter_acceptable_diffs,
    DiffResult,
)


def main():
    jekyll_dir = sys.argv[1]
    rustkyll_dir = sys.argv[2]
    output_path = sys.argv[3]

    common_files, only_jekyll, only_rustkyll = find_common_html_files(jekyll_dir, rustkyll_dir)

    output_lines = []

    def log(msg: str):
        output_lines.append(msg)

    log(f"Jekyll directory:   {jekyll_dir}")
    log(f"Rustkyll directory: {rustkyll_dir}")
    log(f"Common HTML files:  {len(common_files)}")
    if only_jekyll:
        log(f"Only in Jekyll ({len(only_jekyll)}):")
        for f in only_jekyll:
            log(f"  {f}")
    if only_rustkyll:
        log(f"Only in rustkyll ({len(only_rustkyll)}):")
        for f in only_rustkyll:
            log(f"  {f}")
    log("")

    matched = 0
    differing = 0
    total_diffs = 0
    total_accepted = 0

    for i, rel_path in enumerate(common_files):
        if (i + 1) % 100 == 0:
            print(f"  Progress: {i + 1}/{len(common_files)} files compared...",
                  file=sys.stderr)

        jekyll_path = os.path.join(jekyll_dir, rel_path)
        rustkyll_path = os.path.join(rustkyll_dir, rel_path)

        try:
            diffs = compare_html_files(jekyll_path, rustkyll_path)
        except Exception as e:
            diffs = [DiffResult("(parse error)", "error", "", str(e))]

        # Filter out known acceptable differences (e.g. sexagesimal timestamps)
        diffs, accepted = filter_acceptable_diffs(diffs)
        total_accepted += len(accepted)

        if not diffs:
            matched += 1
        else:
            differing += 1
            total_diffs += len(diffs)
            log(f"DIFF {rel_path} ({len(diffs)} differences)")
            # Show ALL diffs - no limit
            for d in diffs:
                log(f"  {d}")

    log("")
    summary = f"Summary: {matched} files matched, {differing} files with differences, {total_diffs} total differences"
    if total_accepted > 0:
        summary += f" ({total_accepted} acceptable diffs filtered out)"
    log(summary)

    with open(output_path, "w", encoding="utf-8") as f:
        f.write("\n".join(output_lines) + "\n")
    print(f"Report written to: {output_path} ({total_diffs} total diffs across {differing} files)", file=sys.stderr)


if __name__ == "__main__":
    main()
