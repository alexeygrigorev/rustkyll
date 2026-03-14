"""Tests for the rustkyll Python wrapper entry point."""

import os
import subprocess
import sys
import unittest
from unittest import mock

# Add the parent directory to the path so we can import rustkyll
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

import rustkyll
from rustkyll import _main


class TestVersion(unittest.TestCase):
    """Test that __version__ is set correctly."""

    def test_version_exists(self):
        self.assertTrue(hasattr(rustkyll, "__version__"))

    def test_version_is_string(self):
        self.assertIsInstance(rustkyll.__version__, str)

    def test_version_matches_pyproject(self):
        """Verify __version__ matches the version in pyproject.toml."""
        pyproject_path = os.path.join(
            os.path.dirname(os.path.dirname(__file__)), "pyproject.toml"
        )
        version = None
        with open(pyproject_path, "r") as f:
            for line in f:
                if line.startswith("version"):
                    version = line.split('"')[1]
                    break
        self.assertIsNotNone(version, "Could not find version in pyproject.toml")
        self.assertEqual(rustkyll.__version__, version)


class TestMainFunction(unittest.TestCase):
    """Test that main() function exists and is callable."""

    def test_main_exists(self):
        self.assertTrue(hasattr(_main, "main"))

    def test_main_is_callable(self):
        self.assertTrue(callable(_main.main))


class TestPlatformDetection(unittest.TestCase):
    """Test platform detection logic for all supported targets."""

    def test_linux_x86_64(self):
        with mock.patch("sys.platform", "linux"), \
             mock.patch("platform.machine", return_value="x86_64"):
            path = _main._get_binary_path()
            self.assertIsNotNone(path)
            self.assertTrue(path.endswith(os.path.join("bin", "rustkyll")))

    def test_linux_aarch64(self):
        with mock.patch("sys.platform", "linux"), \
             mock.patch("platform.machine", return_value="aarch64"):
            path = _main._get_binary_path()
            self.assertIsNotNone(path)
            self.assertTrue(path.endswith(os.path.join("bin", "rustkyll")))

    def test_darwin_x86_64(self):
        with mock.patch("sys.platform", "darwin"), \
             mock.patch("platform.machine", return_value="x86_64"):
            path = _main._get_binary_path()
            self.assertIsNotNone(path)
            self.assertTrue(path.endswith(os.path.join("bin", "rustkyll")))

    def test_darwin_arm64(self):
        with mock.patch("sys.platform", "darwin"), \
             mock.patch("platform.machine", return_value="arm64"):
            path = _main._get_binary_path()
            self.assertIsNotNone(path)
            self.assertTrue(path.endswith(os.path.join("bin", "rustkyll")))

    def test_windows_amd64(self):
        with mock.patch("sys.platform", "win32"), \
             mock.patch("platform.machine", return_value="AMD64"):
            path = _main._get_binary_path()
            self.assertIsNotNone(path)
            self.assertTrue(path.endswith(os.path.join("bin", "rustkyll.exe")))

    def test_unsupported_platform(self):
        with mock.patch("sys.platform", "freebsd"), \
             mock.patch("platform.machine", return_value="armv7l"):
            path = _main._get_binary_path()
            self.assertIsNone(path)


class TestUnsupportedPlatformError(unittest.TestCase):
    """Test that unsupported platforms produce a clear error message."""

    def test_unsupported_platform_exits_with_error(self):
        with mock.patch("sys.platform", "freebsd"), \
             mock.patch("platform.machine", return_value="armv7l"), \
             mock.patch("sys.stderr") as mock_stderr, \
             self.assertRaises(SystemExit) as cm:
            _main.main()
        self.assertEqual(cm.exception.code, 1)

    def test_missing_binary_exits_with_error(self):
        """Even on a supported platform, if binary is missing, exit with error."""
        with mock.patch("sys.platform", "linux"), \
             mock.patch("platform.machine", return_value="x86_64"), \
             mock.patch("sys.stderr") as mock_stderr, \
             self.assertRaises(SystemExit) as cm:
            _main.main()
        self.assertEqual(cm.exception.code, 1)


class TestPythonModuleInvocation(unittest.TestCase):
    """Test that python -m rustkyll works."""

    def test_python_m_rustkyll_invokes_entry_point(self):
        """Running python -m rustkyll should invoke the entry point.

        Since no binary is bundled in the source tree, it should print
        an error message about the unsupported platform or missing binary.
        """
        result = subprocess.run(
            [sys.executable, "-m", "rustkyll"],
            capture_output=True,
            text=True,
            cwd=os.path.join(os.path.dirname(__file__), ".."),
        )
        # Should exit with code 1 (no binary bundled in source tree)
        self.assertEqual(result.returncode, 1)
        self.assertIn("rustkyll", result.stderr.lower())


class TestArgumentForwarding(unittest.TestCase):
    """Test that arguments are forwarded correctly to the binary."""

    @mock.patch("os.execvp")
    def test_unix_args_forwarded(self, mock_execvp):
        """On Unix, verify args are passed through to os.execvp."""
        # Create a fake binary file so the path check passes
        import tempfile
        with tempfile.NamedTemporaryFile(suffix="rustkyll", delete=False) as f:
            fake_binary = f.name

        try:
            with mock.patch.object(_main, "_get_binary_path", return_value=fake_binary), \
                 mock.patch("sys.platform", "linux"), \
                 mock.patch("sys.argv", ["rustkyll", "build", "--source", "/tmp"]):
                _main.main()
                mock_execvp.assert_called_once_with(
                    fake_binary,
                    [fake_binary, "build", "--source", "/tmp"],
                )
        finally:
            os.unlink(fake_binary)

    @mock.patch("subprocess.run")
    def test_windows_args_forwarded(self, mock_run):
        """On Windows, verify args are passed through to subprocess.run."""
        import tempfile
        with tempfile.NamedTemporaryFile(suffix="rustkyll.exe", delete=False) as f:
            fake_binary = f.name

        try:
            mock_run.return_value = mock.Mock(returncode=0)
            with mock.patch.object(_main, "_get_binary_path", return_value=fake_binary), \
                 mock.patch("sys.platform", "win32"), \
                 mock.patch("sys.argv", ["rustkyll", "serve", "--port", "8080"]), \
                 self.assertRaises(SystemExit) as cm:
                _main.main()
            self.assertEqual(cm.exception.code, 0)
            mock_run.assert_called_once_with(
                [fake_binary, "serve", "--port", "8080"],
            )
        finally:
            os.unlink(fake_binary)

    @mock.patch("subprocess.run")
    def test_windows_exit_code_forwarded(self, mock_run):
        """On Windows, verify the binary's exit code is forwarded."""
        import tempfile
        with tempfile.NamedTemporaryFile(suffix="rustkyll.exe", delete=False) as f:
            fake_binary = f.name

        try:
            mock_run.return_value = mock.Mock(returncode=42)
            with mock.patch.object(_main, "_get_binary_path", return_value=fake_binary), \
                 mock.patch("sys.platform", "win32"), \
                 mock.patch("sys.argv", ["rustkyll", "build"]), \
                 self.assertRaises(SystemExit) as cm:
                _main.main()
            self.assertEqual(cm.exception.code, 42)
        finally:
            os.unlink(fake_binary)


if __name__ == "__main__":
    unittest.main()
