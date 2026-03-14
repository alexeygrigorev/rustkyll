"""Entry point that locates and executes the bundled rustkyll binary."""

import os
import platform
import subprocess
import sys


# Map (sys.platform, platform.machine()) to binary name inside the package.
# platform.machine() returns values like 'x86_64', 'AMD64', 'aarch64', 'arm64'.
_PLATFORM_MAP = {
    ("linux", "x86_64"): "rustkyll",
    ("linux", "aarch64"): "rustkyll",
    ("darwin", "x86_64"): "rustkyll",
    ("darwin", "arm64"): "rustkyll",
    ("win32", "AMD64"): "rustkyll.exe",
    ("win32", "x86_64"): "rustkyll.exe",
}

_SUPPORTED_PLATFORMS = [
    "linux x86_64 (amd64)",
    "linux aarch64 (arm64)",
    "macOS x86_64 (amd64)",
    "macOS arm64 (Apple Silicon)",
    "Windows AMD64",
]


def _get_binary_path():
    """Return the path to the bundled rustkyll binary for the current platform."""
    plat = sys.platform
    machine = platform.machine()

    binary_name = _PLATFORM_MAP.get((plat, machine))
    if binary_name is None:
        return None

    # The binary is bundled at rustkyll/bin/<binary_name> inside the package
    package_dir = os.path.dirname(os.path.abspath(__file__))
    return os.path.join(package_dir, "bin", binary_name)


def main():
    """Locate the bundled binary and execute it, forwarding all arguments."""
    binary_path = _get_binary_path()

    if binary_path is None or not os.path.isfile(binary_path):
        plat = sys.platform
        machine = platform.machine()
        print(
            "Error: No rustkyll binary found for this platform "
            "({platform} {machine}).\n"
            "\n"
            "Supported platforms:\n"
            "{platforms}\n"
            "\n"
            "You can build from source with: cargo install --path .".format(
                platform=plat,
                machine=machine,
                platforms="\n".join("  - " + p for p in _SUPPORTED_PLATFORMS),
            ),
            file=sys.stderr,
        )
        sys.exit(1)

    args = [binary_path] + sys.argv[1:]

    if sys.platform == "win32":
        # On Windows, os.execvp is not reliable; use subprocess instead
        result = subprocess.run(args)
        sys.exit(result.returncode)
    else:
        # On Unix, replace the current process with the binary
        os.execvp(binary_path, args)
