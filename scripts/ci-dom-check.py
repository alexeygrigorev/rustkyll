#!/usr/bin/env -S uv run python
# /// script
# dependencies = ["beautifulsoup4"]
# ///
"""
CI DOM baseline checker.

Builds each site with rustkyll, runs dom_compare.py, and asserts
that the matched file count does not drop below the baseline stored
in docs/dom-baselines.json.

Usage:
    uv run scripts/ci-dom-check.py \
        --rustkyll-bin target/release/rustkyll \
        --websites-dir websites \
        --baselines docs/dom-baselines.json \
        [--output dom-nightly-report.txt] \
        [--sites site1,site2]  # optional: only check these sites

Exit codes:
    0 = all sites at or above baseline
    1 = at least one site regressed below baseline
"""

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path


def find_site_dir(websites_dir: Path, site_key: str) -> Path | None:
    """Map a baseline key (e.g. 'DataTalksClub/docs') to its directory under websites/."""
    candidate = websites_dir / site_key
    if candidate.is_dir() and (candidate / "_site_jekyll_cached").is_dir():
        return candidate
    return None


# File extensions whose first bytes can be safely read as UTF-8 text.
# Binary asset extensions are skipped to avoid false positives.
_TEXTY_EXTENSIONS = {
    ".html", ".htm", ".xml", ".json", ".txt", ".js", ".css", ".scss", ".sass",
    ".md", ".rss", ".atom", ".csv", ".tsv", ".svg",
}


def find_frontmatter_leaks(output_dir: Path) -> list[Path]:
    """Return any output file that begins with `---\\n` -- a leaked YAML
    frontmatter delimiter that should have been stripped during rendering.

    DOM comparison only inspects HTML files. JS/CSS/JSON pages with frontmatter
    that didn't get processed leave the literal `---\\n...\\n---\\n` at the top,
    breaking JS execution and CSS parsing. This catches that class of bug.
    """
    leaks = []
    for p in output_dir.rglob("*"):
        if not p.is_file():
            continue
        if p.suffix.lower() not in _TEXTY_EXTENSIONS:
            continue
        try:
            with open(p, "rb") as f:
                head = f.read(8)
        except OSError:
            continue
        text = head.decode("utf-8", errors="ignore").lstrip("﻿")
        if text.startswith("---\n") or text.startswith("---\r\n"):
            leaks.append(p)
    return leaks


def build_with_rustkyll(rustkyll_bin: Path, site_dir: Path, output_dir: Path) -> bool:
    """Build a site with rustkyll, returning True on success."""
    try:
        result = subprocess.run(
            [str(rustkyll_bin), "build", "--destination", str(output_dir)],
            cwd=str(site_dir),
            capture_output=True,
            text=True,
            timeout=300,
        )
        if result.returncode != 0:
            print(f"  rustkyll build failed (exit {result.returncode})")
            if result.stderr:
                # Print last 10 lines of stderr to avoid flooding
                lines = result.stderr.strip().split("\n")
                for line in lines[-10:]:
                    print(f"    {line}")
            return False
        return True
    except subprocess.TimeoutExpired:
        print("  rustkyll build timed out (300s)")
        return False


def run_dom_compare(
    dom_compare_script: Path,
    jekyll_dir: Path,
    rustkyll_dir: Path,
    report_file: Path | None = None,
) -> tuple[int, str]:
    """Run dom_compare.py and return (matched_count, summary_line).

    Returns (-1, error_message) on failure.
    """
    cmd = [
        "uv", "run", str(dom_compare_script),
        "--jekyll-dir", str(jekyll_dir),
        "--rustkyll-dir", str(rustkyll_dir),
    ]
    if report_file:
        cmd.extend(["--output", str(report_file)])

    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=300,
        )
    except subprocess.TimeoutExpired:
        return (-1, "dom_compare.py timed out (300s)")

    output = result.stdout + result.stderr

    # Extract summary line: "Summary: N matched / M total ..." (new format)
    # or legacy: "Summary: N files matched, ..."
    for line in output.split("\n"):
        if line.startswith("Summary:"):
            # New union-based format: "Summary: N matched / M total ..."
            match = re.match(r"Summary:\s+(\d+)\s+matched\s*/\s*\d+\s+total", line)
            if match:
                return (int(match.group(1)), line.strip())
            # Legacy format: "Summary: N files matched, ..."
            match = re.match(r"Summary:\s+(\d+)\s+files matched", line)
            if match:
                return (int(match.group(1)), line.strip())

    return (-1, f"Could not parse dom_compare output (exit {result.returncode})")


def main():
    parser = argparse.ArgumentParser(description="CI DOM baseline checker")
    parser.add_argument(
        "--rustkyll-bin",
        required=True,
        help="Path to rustkyll binary",
    )
    parser.add_argument(
        "--websites-dir",
        required=True,
        help="Path to websites/ directory",
    )
    parser.add_argument(
        "--baselines",
        required=True,
        help="Path to docs/dom-baselines.json",
    )
    parser.add_argument(
        "--output",
        default=None,
        help="Path to write full report",
    )
    parser.add_argument(
        "--sites",
        default=None,
        help="Comma-separated list of site keys to check (default: all available)",
    )
    args = parser.parse_args()

    rustkyll_bin = Path(args.rustkyll_bin).resolve()
    websites_dir = Path(args.websites_dir).resolve()
    baselines_path = Path(args.baselines).resolve()

    if not rustkyll_bin.is_file():
        print(f"ERROR: rustkyll binary not found at {rustkyll_bin}")
        sys.exit(1)

    if not websites_dir.is_dir():
        print(f"ERROR: websites directory not found at {websites_dir}")
        sys.exit(1)

    if not baselines_path.is_file():
        print(f"ERROR: baselines file not found at {baselines_path}")
        sys.exit(1)

    with open(baselines_path) as f:
        baselines = json.load(f)

    # Locate dom_compare.py relative to this script
    scripts_dir = Path(__file__).resolve().parent
    dom_compare_script = scripts_dir / "dom_compare.py"
    if not dom_compare_script.is_file():
        print(f"ERROR: dom_compare.py not found at {dom_compare_script}")
        sys.exit(1)

    # Filter sites if --sites is provided
    if args.sites:
        site_keys = [s.strip() for s in args.sites.split(",")]
    else:
        site_keys = list(baselines.keys())

    results = []
    regressions = []
    skipped = []

    for site_key in site_keys:
        baseline = baselines.get(site_key)
        if baseline is None:
            print(f"WARNING: {site_key} not found in baselines, skipping")
            skipped.append(site_key)
            continue

        site_dir = find_site_dir(websites_dir, site_key)
        if site_dir is None:
            print(f"SKIP: {site_key} (not found or no _site_jekyll_cached)")
            skipped.append(site_key)
            continue

        print(f"\n{'='*60}")
        print(f"Checking: {site_key} (baseline: {baseline})")
        print(f"{'='*60}")

        # Build with rustkyll into a temp directory
        with tempfile.TemporaryDirectory(prefix=f"rustkyll_{site_key.replace('/', '_')}_") as tmp:
            output_dir = Path(tmp) / "_site"

            if not build_with_rustkyll(rustkyll_bin, site_dir, output_dir):
                print(f"  FAIL: build failed")
                results.append((site_key, baseline, -1, "build failed"))
                regressions.append(site_key)
                continue

            leaks = find_frontmatter_leaks(output_dir)
            if leaks:
                rels = [str(p.relative_to(output_dir)) for p in leaks]
                msg = "frontmatter leak in: " + ", ".join(rels[:5])
                if len(rels) > 5:
                    msg += f" (+{len(rels) - 5} more)"
                print(f"  FAIL: {msg}")
                results.append((site_key, baseline, -1, msg))
                regressions.append(site_key)
                continue

            jekyll_dir = site_dir / "_site_jekyll_cached"
            report_file = None
            if args.output:
                report_dir = Path(args.output).parent
                report_dir.mkdir(parents=True, exist_ok=True)
                report_file = report_dir / f"dom-report-{site_key.replace('/', '_')}.txt"

            matched, summary = run_dom_compare(
                dom_compare_script, jekyll_dir, output_dir, report_file
            )

            if matched < 0:
                print(f"  FAIL: {summary}")
                results.append((site_key, baseline, -1, summary))
                regressions.append(site_key)
                continue

            status = "PASS" if matched >= baseline else "REGRESSED"
            print(f"  {status}: {matched}/{baseline} (matched/baseline)")
            print(f"  {summary}")
            results.append((site_key, baseline, matched, summary))

            if matched < baseline:
                regressions.append(site_key)

    # Print summary
    print(f"\n{'='*60}")
    print("SUMMARY")
    print(f"{'='*60}")
    print(f"Sites checked: {len(results)}")
    print(f"Sites skipped: {len(skipped)}")
    print(f"Regressions:   {len(regressions)}")
    print()

    for site_key, baseline, matched, summary in results:
        if matched < 0:
            status = "FAIL"
        elif matched < baseline:
            status = "REGRESSED"
        elif matched > baseline:
            status = "IMPROVED"
        else:
            status = "OK"
        print(f"  [{status:>9}] {site_key}: {matched}/{baseline}")

    if skipped:
        print()
        print("Skipped sites:")
        for s in skipped:
            print(f"  - {s}")

    # Write full report if requested
    if args.output:
        output_path = Path(args.output)
        with open(output_path, "w") as f:
            f.write("DOM Baseline Check Report\n")
            f.write(f"{'='*60}\n\n")
            f.write(f"Sites checked: {len(results)}\n")
            f.write(f"Sites skipped: {len(skipped)}\n")
            f.write(f"Regressions:   {len(regressions)}\n\n")
            for site_key, baseline, matched, summary in results:
                if matched < 0:
                    status = "FAIL"
                elif matched < baseline:
                    status = "REGRESSED"
                elif matched > baseline:
                    status = "IMPROVED"
                else:
                    status = "OK"
                f.write(f"[{status:>9}] {site_key}: {matched}/{baseline}\n")
                f.write(f"            {summary}\n\n")
            if skipped:
                f.write("\nSkipped sites:\n")
                for s in skipped:
                    f.write(f"  - {s}\n")
        print(f"\nFull report written to: {output_path}")

    if regressions:
        print(f"\nFAILED: {len(regressions)} site(s) regressed below baseline:")
        for r in regressions:
            print(f"  - {r}")
        sys.exit(1)
    else:
        print("\nPASSED: All sites at or above baseline.")
        sys.exit(0)


if __name__ == "__main__":
    main()
