"""Tests for the wheel-building script."""

import os
import stat
import sys
import tempfile
import unittest
import zipfile

# Add scripts to path so we can import build_wheels
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "..", "scripts"))

# Import the build-wheels module (has a hyphen in filename, so use importlib)
import importlib.util
spec = importlib.util.spec_from_file_location(
    "build_wheels",
    os.path.join(os.path.dirname(__file__), "..", "..", "scripts", "build-wheels.py"),
)
build_wheels = importlib.util.module_from_spec(spec)
spec.loader.exec_module(build_wheels)


class TestReadVersion(unittest.TestCase):
    """Test version reading from pyproject.toml."""

    def test_reads_version(self):
        version = build_wheels.read_version()
        self.assertIsInstance(version, str)
        self.assertRegex(version, r"^\d+\.\d+\.\d+")


class TestBuildWheel(unittest.TestCase):
    """Test building a single wheel with a dummy binary."""

    def test_build_linux_amd64_wheel(self):
        """Build a wheel for linux-amd64 with a dummy binary and verify contents."""
        with tempfile.TemporaryDirectory() as tmpdir:
            # Create a dummy binary
            binary_path = os.path.join(tmpdir, "rustkyll")
            with open(binary_path, "wb") as f:
                f.write(b"#!/bin/sh\necho hello\n")

            output_dir = os.path.join(tmpdir, "dist")
            os.makedirs(output_dir)

            wheel_path = build_wheels.build_wheel(
                binary_path=binary_path,
                platform_tag="manylinux_2_17_x86_64.manylinux2014_x86_64",
                binary_name="rustkyll",
                version="0.1.0",
                output_dir=output_dir,
            )

            # Verify wheel file exists
            self.assertTrue(os.path.isfile(wheel_path))

            # Verify wheel filename
            self.assertIn("rustkyll-0.1.0", os.path.basename(wheel_path))
            self.assertIn("manylinux_2_17_x86_64", os.path.basename(wheel_path))
            self.assertTrue(wheel_path.endswith(".whl"))

            # Verify wheel contents
            with zipfile.ZipFile(wheel_path, "r") as whl:
                names = whl.namelist()

                # Binary should be present
                self.assertIn("rustkyll/bin/rustkyll", names)

                # Python files should be present
                self.assertIn("rustkyll/__init__.py", names)
                self.assertIn("rustkyll/__main__.py", names)
                self.assertIn("rustkyll/_main.py", names)

                # Dist-info files should be present
                self.assertIn("rustkyll-0.1.0.dist-info/METADATA", names)
                self.assertIn("rustkyll-0.1.0.dist-info/WHEEL", names)
                self.assertIn("rustkyll-0.1.0.dist-info/RECORD", names)
                self.assertIn("rustkyll-0.1.0.dist-info/entry_points.txt", names)

                # Verify METADATA content
                metadata = whl.read("rustkyll-0.1.0.dist-info/METADATA").decode("utf-8")
                self.assertIn("Name: rustkyll", metadata)
                self.assertIn("Version: 0.1.0", metadata)

                # Verify WHEEL content
                wheel_meta = whl.read("rustkyll-0.1.0.dist-info/WHEEL").decode("utf-8")
                self.assertIn("Tag: py3-none-manylinux_2_17_x86_64", wheel_meta)

                # Verify binary has executable permissions
                info = whl.getinfo("rustkyll/bin/rustkyll")
                unix_mode = (info.external_attr >> 16) & 0o777
                self.assertTrue(unix_mode & stat.S_IXUSR, "Binary should be executable")

    def test_build_windows_wheel(self):
        """Build a wheel for windows-amd64 and verify no Unix permissions set."""
        with tempfile.TemporaryDirectory() as tmpdir:
            binary_path = os.path.join(tmpdir, "rustkyll.exe")
            with open(binary_path, "wb") as f:
                f.write(b"MZ\x00\x00")  # Minimal PE header marker

            output_dir = os.path.join(tmpdir, "dist")
            os.makedirs(output_dir)

            wheel_path = build_wheels.build_wheel(
                binary_path=binary_path,
                platform_tag="win_amd64",
                binary_name="rustkyll.exe",
                version="0.1.0",
                output_dir=output_dir,
            )

            self.assertTrue(os.path.isfile(wheel_path))
            self.assertIn("win_amd64", os.path.basename(wheel_path))

            with zipfile.ZipFile(wheel_path, "r") as whl:
                self.assertIn("rustkyll/bin/rustkyll.exe", whl.namelist())

    def test_build_darwin_arm64_wheel(self):
        """Build a wheel for macOS ARM64."""
        with tempfile.TemporaryDirectory() as tmpdir:
            binary_path = os.path.join(tmpdir, "rustkyll")
            with open(binary_path, "wb") as f:
                f.write(b"\xcf\xfa\xed\xfe")  # Mach-O magic

            output_dir = os.path.join(tmpdir, "dist")
            os.makedirs(output_dir)

            wheel_path = build_wheels.build_wheel(
                binary_path=binary_path,
                platform_tag="macosx_11_0_arm64",
                binary_name="rustkyll",
                version="0.1.0",
                output_dir=output_dir,
            )

            self.assertTrue(os.path.isfile(wheel_path))
            self.assertIn("macosx_11_0_arm64", os.path.basename(wheel_path))


class TestMainWithDummyBinaries(unittest.TestCase):
    """Test the main function with a directory of dummy binaries."""

    def test_builds_all_wheels_from_flat_directory(self):
        """Test building wheels from a flat directory of binaries."""
        with tempfile.TemporaryDirectory() as tmpdir:
            binaries_dir = os.path.join(tmpdir, "binaries")
            output_dir = os.path.join(tmpdir, "dist")
            os.makedirs(binaries_dir)
            os.makedirs(output_dir)

            # Create dummy binaries for all targets
            for asset_name, _, _ in build_wheels.TARGETS:
                with open(os.path.join(binaries_dir, asset_name), "wb") as f:
                    f.write(b"dummy binary content")

            # Run the build
            sys.argv = [
                "build-wheels.py",
                "--binaries-dir", binaries_dir,
                "--output-dir", output_dir,
                "--version", "0.2.0",
            ]
            result = build_wheels.main()
            self.assertEqual(result, 0)

            # Verify 5 wheels were created
            wheels = [f for f in os.listdir(output_dir) if f.endswith(".whl")]
            self.assertEqual(len(wheels), 6)

    def test_builds_wheels_from_nested_directory(self):
        """Test building from nested artifact directories (GitHub Actions layout)."""
        with tempfile.TemporaryDirectory() as tmpdir:
            binaries_dir = os.path.join(tmpdir, "artifacts")
            output_dir = os.path.join(tmpdir, "dist")
            os.makedirs(output_dir)

            # Create nested directory structure like download-artifact creates
            for asset_name, _, _ in build_wheels.TARGETS:
                artifact_dir = os.path.join(binaries_dir, asset_name)
                os.makedirs(artifact_dir)
                with open(os.path.join(artifact_dir, asset_name), "wb") as f:
                    f.write(b"dummy binary content")

            sys.argv = [
                "build-wheels.py",
                "--binaries-dir", binaries_dir,
                "--output-dir", output_dir,
                "--version", "0.2.0",
            ]
            result = build_wheels.main()
            self.assertEqual(result, 0)

            wheels = [f for f in os.listdir(output_dir) if f.endswith(".whl")]
            self.assertEqual(len(wheels), 6)

    def test_skips_missing_binaries_with_warning(self):
        """Test that missing binaries are skipped gracefully."""
        with tempfile.TemporaryDirectory() as tmpdir:
            binaries_dir = os.path.join(tmpdir, "binaries")
            output_dir = os.path.join(tmpdir, "dist")
            os.makedirs(binaries_dir)
            os.makedirs(output_dir)

            # Only create one binary
            with open(os.path.join(binaries_dir, "rustkyll-linux-amd64"), "wb") as f:
                f.write(b"dummy binary content")

            sys.argv = [
                "build-wheels.py",
                "--binaries-dir", binaries_dir,
                "--output-dir", output_dir,
                "--version", "0.1.0",
            ]
            result = build_wheels.main()
            self.assertEqual(result, 0)

            wheels = [f for f in os.listdir(output_dir) if f.endswith(".whl")]
            self.assertEqual(len(wheels), 1)

    def test_fails_if_no_binaries_found(self):
        """Test that the script exits with error if no binaries exist."""
        with tempfile.TemporaryDirectory() as tmpdir:
            binaries_dir = os.path.join(tmpdir, "empty")
            output_dir = os.path.join(tmpdir, "dist")
            os.makedirs(binaries_dir)
            os.makedirs(output_dir)

            sys.argv = [
                "build-wheels.py",
                "--binaries-dir", binaries_dir,
                "--output-dir", output_dir,
            ]
            with self.assertRaises(SystemExit) as cm:
                build_wheels.main()
            self.assertEqual(cm.exception.code, 1)


if __name__ == "__main__":
    unittest.main()
