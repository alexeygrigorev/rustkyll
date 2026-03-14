#!/usr/bin/env python3
"""Build platform-tagged wheels for rustkyll.

Usage:
    python scripts/build-wheels.py --binaries-dir ./artifacts --output-dir ./dist

The binaries-dir should contain the release binaries with names:
    rustkyll-linux-amd64
    rustkyll-linux-arm64
    rustkyll-darwin-amd64
    rustkyll-darwin-arm64
    rustkyll-windows-amd64.exe

Each binary is packaged into a platform-tagged wheel.
"""

import argparse
import base64
import csv
import hashlib
import io
import os
import re
import stat
import sys
import zipfile


# Wheel tag mapping: (asset_name, wheel_platform_tag, binary_name_inside_wheel)
TARGETS = [
    (
        "rustkyll-linux-amd64",
        "manylinux_2_17_x86_64.manylinux2014_x86_64",
        "rustkyll",
    ),
    (
        "rustkyll-linux-arm64",
        "manylinux_2_17_aarch64.manylinux2014_aarch64",
        "rustkyll",
    ),
    (
        "rustkyll-darwin-amd64",
        "macosx_10_12_x86_64",
        "rustkyll",
    ),
    (
        "rustkyll-darwin-arm64",
        "macosx_11_0_arm64",
        "rustkyll",
    ),
    (
        "rustkyll-windows-amd64.exe",
        "win_amd64",
        "rustkyll.exe",
    ),
]


def read_version():
    """Read version from python/pyproject.toml."""
    pyproject_path = os.path.join(
        os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
        "python",
        "pyproject.toml",
    )
    with open(pyproject_path, "r") as f:
        for line in f:
            m = re.match(r'^version\s*=\s*"([^"]+)"', line)
            if m:
                return m.group(1)
    raise RuntimeError("Could not find version in python/pyproject.toml")


def sha256_digest(data):
    """Return the SHA-256 digest of bytes data in urlsafe-base64 (no padding)."""
    h = hashlib.sha256(data)
    return base64.urlsafe_b64encode(h.digest()).rstrip(b"=").decode("ascii")


def build_wheel(binary_path, platform_tag, binary_name, version, output_dir):
    """Build a single platform-tagged wheel."""
    wheel_name = "rustkyll-{version}-py3-none-{platform}.whl".format(
        version=version, platform=platform_tag
    )
    wheel_path = os.path.join(output_dir, wheel_name)

    dist_info = "rustkyll-{version}.dist-info".format(version=version)

    with open(binary_path, "rb") as f:
        binary_data = f.read()

    # Build file records for RECORD
    records = []

    with zipfile.ZipFile(wheel_path, "w", zipfile.ZIP_DEFLATED) as whl:
        # Write the binary into rustkyll/bin/
        bin_path_in_wheel = "rustkyll/bin/{name}".format(name=binary_name)

        # Set executable permissions for Unix binaries
        info = zipfile.ZipInfo(bin_path_in_wheel)
        if not binary_name.endswith(".exe"):
            # Set Unix executable permissions: rwxr-xr-x
            info.external_attr = (
                stat.S_IRWXU | stat.S_IRGRP | stat.S_IXGRP | stat.S_IROTH | stat.S_IXOTH
            ) << 16
        info.compress_type = zipfile.ZIP_DEFLATED
        whl.writestr(info, binary_data)
        records.append(
            (bin_path_in_wheel, "sha256=" + sha256_digest(binary_data), str(len(binary_data)))
        )

        # Write __init__.py
        init_content = '"""rustkyll - A fast static site generator compatible with Jekyll, written in Rust."""\n\n__version__ = "{version}"\n'.format(
            version=version
        )
        init_data = init_content.encode("utf-8")
        whl.writestr("rustkyll/__init__.py", init_data)
        records.append(
            ("rustkyll/__init__.py", "sha256=" + sha256_digest(init_data), str(len(init_data)))
        )

        # Write __main__.py
        main_content = '"""Allow running rustkyll as `python -m rustkyll`."""\n\nfrom rustkyll._main import main\n\nif __name__ == "__main__":\n    main()\n'
        main_data = main_content.encode("utf-8")
        whl.writestr("rustkyll/__main__.py", main_data)
        records.append(
            ("rustkyll/__main__.py", "sha256=" + sha256_digest(main_data), str(len(main_data)))
        )

        # Write _main.py
        main_py_path = os.path.join(
            os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
            "python",
            "rustkyll",
            "_main.py",
        )
        with open(main_py_path, "rb") as f:
            main_py_data = f.read()
        whl.writestr("rustkyll/_main.py", main_py_data)
        records.append(
            ("rustkyll/_main.py", "sha256=" + sha256_digest(main_py_data), str(len(main_py_data)))
        )

        # Write METADATA
        metadata = """\
Metadata-Version: 2.1
Name: rustkyll
Version: {version}
Summary: A fast static site generator compatible with Jekyll, written in Rust
License: MIT
Requires-Python: >=3.8
""".format(
            version=version
        )
        metadata_data = metadata.encode("utf-8")
        metadata_path = "{dist_info}/METADATA".format(dist_info=dist_info)
        whl.writestr(metadata_path, metadata_data)
        records.append(
            (metadata_path, "sha256=" + sha256_digest(metadata_data), str(len(metadata_data)))
        )

        # Write WHEEL
        wheel_meta = """\
Wheel-Version: 1.0
Generator: rustkyll-build-wheels
Root-Is-Purelib: false
Tag: py3-none-{platform}
""".format(
            platform=platform_tag
        )
        wheel_meta_data = wheel_meta.encode("utf-8")
        wheel_meta_path = "{dist_info}/WHEEL".format(dist_info=dist_info)
        whl.writestr(wheel_meta_path, wheel_meta_data)
        records.append(
            (
                wheel_meta_path,
                "sha256=" + sha256_digest(wheel_meta_data),
                str(len(wheel_meta_data)),
            )
        )

        # Write entry_points.txt (console_scripts)
        entry_points = "[console_scripts]\nrustkyll = rustkyll._main:main\n"
        entry_points_data = entry_points.encode("utf-8")
        ep_path = "{dist_info}/entry_points.txt".format(dist_info=dist_info)
        whl.writestr(ep_path, entry_points_data)
        records.append(
            (ep_path, "sha256=" + sha256_digest(entry_points_data), str(len(entry_points_data)))
        )

        # Write top_level.txt
        top_level = "rustkyll\n"
        top_level_data = top_level.encode("utf-8")
        tl_path = "{dist_info}/top_level.txt".format(dist_info=dist_info)
        whl.writestr(tl_path, top_level_data)
        records.append(
            (tl_path, "sha256=" + sha256_digest(top_level_data), str(len(top_level_data)))
        )

        # Write RECORD (must be last, and its own entry has no hash)
        record_path = "{dist_info}/RECORD".format(dist_info=dist_info)
        records.append((record_path, "", ""))

        record_buf = io.StringIO()
        writer = csv.writer(record_buf, lineterminator="\n")
        for row in records:
            writer.writerow(row)
        record_data = record_buf.getvalue().encode("utf-8")
        whl.writestr(record_path, record_data)

    return wheel_path


def main():
    parser = argparse.ArgumentParser(description="Build platform-tagged wheels for rustkyll")
    parser.add_argument(
        "--binaries-dir",
        required=True,
        help="Directory containing the release binaries",
    )
    parser.add_argument(
        "--output-dir",
        default="dist",
        help="Directory to write wheels to (default: dist)",
    )
    parser.add_argument(
        "--version",
        default=None,
        help="Version override (default: read from pyproject.toml)",
    )
    args = parser.parse_args()

    version = args.version or read_version()
    os.makedirs(args.output_dir, exist_ok=True)

    built = []
    skipped = []

    for asset_name, platform_tag, binary_name in TARGETS:
        # The download-artifact action creates subdirectories per artifact name
        # Try both flat and nested layouts
        binary_path = os.path.join(args.binaries_dir, asset_name)
        if not os.path.isfile(binary_path):
            binary_path = os.path.join(args.binaries_dir, asset_name, asset_name)
        if not os.path.isfile(binary_path):
            print("WARNING: Binary not found for {asset}, skipping".format(asset=asset_name))
            skipped.append(asset_name)
            continue

        wheel_path = build_wheel(binary_path, platform_tag, binary_name, version, args.output_dir)
        built.append(wheel_path)
        print("Built: {path}".format(path=wheel_path))

    print("\nSummary: {built} wheels built, {skipped} skipped".format(
        built=len(built), skipped=len(skipped)
    ))

    if not built:
        print("ERROR: No wheels were built!", file=sys.stderr)
        sys.exit(1)

    return 0


if __name__ == "__main__":
    sys.exit(main() or 0)
