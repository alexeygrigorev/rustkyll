#!/usr/bin/env python3
"""Tests for dom_compare.py"""

import os
import subprocess
import sys
import tempfile
import unittest

# Add scripts directory to path so we can import dom_compare
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from dom_compare import (
    DiffResult,
    compare_html_files,
    compare_trees,
    find_common_html_files,
    normalize_text,
    parse_and_normalize,
    compare_directories,
)


class TestNormalizeText(unittest.TestCase):
    def test_strips_leading_trailing_whitespace(self):
        self.assertEqual(normalize_text("  hello  "), "hello")

    def test_collapses_internal_whitespace(self):
        self.assertEqual(normalize_text("hello   world"), "hello world")

    def test_preserves_nbsp(self):
        # \xa0 is the non-breaking space character
        result = normalize_text("\xa0")
        self.assertEqual(result, "\xa0")

    def test_empty_string(self):
        self.assertEqual(normalize_text(""), "")

    def test_tabs_and_newlines(self):
        self.assertEqual(normalize_text("hello\n\tworld"), "hello world")


class TestDOMNormalization(unittest.TestCase):
    def test_attribute_order_irrelevant(self):
        """Attributes in different order should normalize to the same result."""
        html1 = '<div class="a" id="b">text</div>'
        html2 = '<div id="b" class="a">text</div>'
        tree1 = parse_and_normalize(html1)
        tree2 = parse_and_normalize(html2)
        diffs = compare_trees(tree1, tree2)
        self.assertEqual(len(diffs), 0, f"Expected no diffs, got: {diffs}")

    def test_whitespace_between_tags_irrelevant(self):
        """Whitespace-only text nodes between tags should be ignored."""
        html1 = '<div>  <span>text</span>  </div>'
        html2 = '<div><span>text</span></div>'
        tree1 = parse_and_normalize(html1)
        tree2 = parse_and_normalize(html2)
        diffs = compare_trees(tree1, tree2)
        self.assertEqual(len(diffs), 0, f"Expected no diffs, got: {diffs}")

    def test_nbsp_is_meaningful(self):
        """&nbsp; (\\xa0) must be preserved and detected as different from empty."""
        html1 = '<p>\xa0</p>'
        html2 = '<p></p>'
        tree1 = parse_and_normalize(html1)
        tree2 = parse_and_normalize(html2)
        diffs = compare_trees(tree1, tree2)
        self.assertGreater(len(diffs), 0, "Expected nbsp vs empty to produce diffs")
        # Check that the diff is about text
        has_text_diff = any(d.diff_type in ("text_differs", "missing_text", "extra_text")
                           for d in diffs)
        self.assertTrue(has_text_diff, f"Expected text diff, got: {diffs}")

    def test_leading_trailing_whitespace_stripped(self):
        """Leading/trailing whitespace in text nodes should be stripped."""
        html1 = '<p>  hello  </p>'
        html2 = '<p>hello</p>'
        tree1 = parse_and_normalize(html1)
        tree2 = parse_and_normalize(html2)
        diffs = compare_trees(tree1, tree2)
        self.assertEqual(len(diffs), 0, f"Expected no diffs, got: {diffs}")

    def test_missing_attribute_detected(self):
        """Missing attribute (target) should be detected."""
        html1 = '<a href="/" target="_blank">link</a>'
        html2 = '<a href="/">link</a>'
        tree1 = parse_and_normalize(html1)
        tree2 = parse_and_normalize(html2)
        diffs = compare_trees(tree1, tree2)
        self.assertGreater(len(diffs), 0, "Expected attribute diff")
        attr_diffs = [d for d in diffs if "attribute" in d.diff_type]
        self.assertGreater(len(attr_diffs), 0, f"Expected attribute diff, got: {diffs}")


class TestDOMComparison(unittest.TestCase):
    def test_identical_documents(self):
        """Two identical documents should have 0 differences."""
        html = '<html><head><title>Test</title></head><body><p>Hello</p></body></html>'
        tree1 = parse_and_normalize(html)
        tree2 = parse_and_normalize(html)
        diffs = compare_trees(tree1, tree2)
        self.assertEqual(len(diffs), 0)

    def test_extra_element_detected(self):
        """Extra <p> element should be detected."""
        html1 = '<div><p>first</p></div>'
        html2 = '<div><p>first</p><p>second</p></div>'
        tree1 = parse_and_normalize(html1)
        tree2 = parse_and_normalize(html2)
        diffs = compare_trees(tree1, tree2)
        self.assertGreater(len(diffs), 0, "Expected extra element diff")
        has_extra = any("extra" in d.diff_type for d in diffs)
        self.assertTrue(has_extra, f"Expected extra element diff, got: {diffs}")

    def test_missing_element_detected(self):
        """Missing <p> element should be detected."""
        html1 = '<div><p>first</p><p>second</p></div>'
        html2 = '<div><p>first</p></div>'
        tree1 = parse_and_normalize(html1)
        tree2 = parse_and_normalize(html2)
        diffs = compare_trees(tree1, tree2)
        self.assertGreater(len(diffs), 0, "Expected missing element diff")
        has_missing = any("missing" in d.diff_type for d in diffs)
        self.assertTrue(has_missing, f"Expected missing element diff, got: {diffs}")

    def test_class_difference_detected(self):
        """CSS class difference should be detected."""
        html1 = '<div class="foo">text</div>'
        html2 = '<div>text</div>'
        tree1 = parse_and_normalize(html1)
        tree2 = parse_and_normalize(html2)
        diffs = compare_trees(tree1, tree2)
        self.assertGreater(len(diffs), 0, "Expected class diff")
        attr_diffs = [d for d in diffs if "attribute" in d.diff_type]
        self.assertGreater(len(attr_diffs), 0, f"Expected attribute diff, got: {diffs}")

    def test_deeply_nested_text_difference(self):
        """Text difference in deeply nested element should be detected with path."""
        html1 = '<html><body><div><section><p>hello</p></section></div></body></html>'
        html2 = '<html><body><div><section><p>world</p></section></div></body></html>'
        tree1 = parse_and_normalize(html1)
        tree2 = parse_and_normalize(html2)
        diffs = compare_trees(tree1, tree2)
        self.assertGreater(len(diffs), 0, "Expected text diff")
        has_text_diff = any(d.diff_type == "text_differs" for d in diffs)
        self.assertTrue(has_text_diff, f"Expected text_differs, got: {diffs}")
        # The path should contain the nesting
        text_diff = [d for d in diffs if d.diff_type == "text_differs"][0]
        self.assertIn("p", text_diff.path)

    def test_element_order_difference_detected(self):
        """Different element order (<p> then <ul> vs <ul> then <p>) should be detected."""
        html1 = '<div><p>text</p><ul><li>item</li></ul></div>'
        html2 = '<div><ul><li>item</li></ul><p>text</p></div>'
        tree1 = parse_and_normalize(html1)
        tree2 = parse_and_normalize(html2)
        diffs = compare_trees(tree1, tree2)
        self.assertGreater(len(diffs), 0, "Expected structural diff due to element order")

    def test_tag_name_difference_detected(self):
        """<div> vs <span> at the same position should be detected."""
        html1 = '<section><div>text</div></section>'
        html2 = '<section><span>text</span></section>'
        tree1 = parse_and_normalize(html1)
        tree2 = parse_and_normalize(html2)
        diffs = compare_trees(tree1, tree2)
        self.assertGreater(len(diffs), 0, "Expected tag name diff")
        has_tag_diff = any(d.diff_type == "tag_name_differs" for d in diffs)
        self.assertTrue(has_tag_diff, f"Expected tag_name_differs, got: {diffs}")


class TestDirectoryComparison(unittest.TestCase):
    def setUp(self):
        self.tmpdir = tempfile.mkdtemp()
        self.jekyll_dir = os.path.join(self.tmpdir, "jekyll")
        self.rustkyll_dir = os.path.join(self.tmpdir, "rustkyll")
        os.makedirs(self.jekyll_dir)
        os.makedirs(self.rustkyll_dir)

    def tearDown(self):
        import shutil
        shutil.rmtree(self.tmpdir)

    def _write_file(self, base_dir, rel_path, content):
        full_path = os.path.join(base_dir, rel_path)
        os.makedirs(os.path.dirname(full_path), exist_ok=True)
        with open(full_path, "w") as f:
            f.write(content)

    def test_matching_files_exit_0(self):
        """All identical files should exit 0."""
        html = '<html><body><p>Hello</p></body></html>'
        self._write_file(self.jekyll_dir, "index.html", html)
        self._write_file(self.rustkyll_dir, "index.html", html)
        self._write_file(self.jekyll_dir, "about.html", html)
        self._write_file(self.rustkyll_dir, "about.html", html)

        exit_code = compare_directories(self.jekyll_dir, self.rustkyll_dir)
        self.assertEqual(exit_code, 0)

    def test_differing_files_exit_1(self):
        """Directories with differences should exit 1."""
        html_match = '<html><body><p>Hello</p></body></html>'
        html_j = '<html><body><p>Jekyll version</p></body></html>'
        html_r = '<html><body><p>Rustkyll version</p></body></html>'

        # 2 matching, 1 different
        self._write_file(self.jekyll_dir, "index.html", html_match)
        self._write_file(self.rustkyll_dir, "index.html", html_match)
        self._write_file(self.jekyll_dir, "about.html", html_match)
        self._write_file(self.rustkyll_dir, "about.html", html_match)
        self._write_file(self.jekyll_dir, "diff.html", html_j)
        self._write_file(self.rustkyll_dir, "diff.html", html_r)

        exit_code = compare_directories(self.jekyll_dir, self.rustkyll_dir)
        self.assertEqual(exit_code, 1)

    def test_only_common_files_compared(self):
        """Files only in one directory should not cause comparison failure on their own."""
        html = '<html><body><p>Hello</p></body></html>'
        self._write_file(self.jekyll_dir, "index.html", html)
        self._write_file(self.rustkyll_dir, "index.html", html)
        # Extra file only in jekyll
        self._write_file(self.jekyll_dir, "extra.html", html)

        exit_code = compare_directories(self.jekyll_dir, self.rustkyll_dir)
        self.assertEqual(exit_code, 0)  # Common files match

    def test_find_common_files(self):
        """find_common_html_files should correctly categorize files."""
        html = '<html><body><p>Hello</p></body></html>'
        self._write_file(self.jekyll_dir, "common.html", html)
        self._write_file(self.rustkyll_dir, "common.html", html)
        self._write_file(self.jekyll_dir, "only_j.html", html)
        self._write_file(self.rustkyll_dir, "only_r.html", html)

        common, only_j, only_r = find_common_html_files(self.jekyll_dir, self.rustkyll_dir)
        self.assertEqual(common, ["common.html"])
        self.assertEqual(only_j, ["only_j.html"])
        self.assertEqual(only_r, ["only_r.html"])

    def test_summary_output(self):
        """Summary should report correct counts."""
        html_match = '<html><body><p>Hello</p></body></html>'
        html_j = '<html><body><p class="x">Hello</p></body></html>'
        html_r = '<html><body><p>Hello</p></body></html>'

        self._write_file(self.jekyll_dir, "match1.html", html_match)
        self._write_file(self.rustkyll_dir, "match1.html", html_match)
        self._write_file(self.jekyll_dir, "match2.html", html_match)
        self._write_file(self.rustkyll_dir, "match2.html", html_match)
        self._write_file(self.jekyll_dir, "diff1.html", html_j)
        self._write_file(self.rustkyll_dir, "diff1.html", html_r)

        # Capture stdout
        import io
        from contextlib import redirect_stdout
        f = io.StringIO()
        with redirect_stdout(f):
            compare_directories(self.jekyll_dir, self.rustkyll_dir)
        output = f.getvalue()
        self.assertIn("2 files matched", output)
        self.assertIn("1 files with differences", output)


class TestRegressionDetection(unittest.TestCase):
    """Regression tests for specific known difference patterns."""

    def test_nbsp_spacer_detected(self):
        """<p>&nbsp;</p> present in one but absent in other should be detected."""
        html1 = '<div><p>\xa0</p><p>text</p></div>'
        html2 = '<div><p>text</p></div>'
        tree1 = parse_and_normalize(html1)
        tree2 = parse_and_normalize(html2)
        diffs = compare_trees(tree1, tree2)
        self.assertGreater(len(diffs), 0, "Expected nbsp spacer diff")

    def test_script_content_detected(self):
        """Script content differences should be detected."""
        html1 = '<html><body><script>var a = 1;</script></body></html>'
        html2 = '<html><body><script>var b = 2;</script></body></html>'
        tree1 = parse_and_normalize(html1)
        tree2 = parse_and_normalize(html2)
        diffs = compare_trees(tree1, tree2)
        self.assertGreater(len(diffs), 0, "Expected script content diff")

    def test_data_attribute_detected(self):
        """data-* attribute differences should be detected."""
        html1 = '<div data-id="123">text</div>'
        html2 = '<div data-id="456">text</div>'
        tree1 = parse_and_normalize(html1)
        tree2 = parse_and_normalize(html2)
        diffs = compare_trees(tree1, tree2)
        self.assertGreater(len(diffs), 0, "Expected data attribute diff")
        attr_diffs = [d for d in diffs if "attribute" in d.diff_type]
        self.assertGreater(len(attr_diffs), 0)

    def test_data_attribute_missing_detected(self):
        """Missing data-* attribute should be detected."""
        html1 = '<div data-value="x">text</div>'
        html2 = '<div>text</div>'
        tree1 = parse_and_normalize(html1)
        tree2 = parse_and_normalize(html2)
        diffs = compare_trees(tree1, tree2)
        self.assertGreater(len(diffs), 0, "Expected missing data attribute diff")


class TestCLI(unittest.TestCase):
    """Test the command-line interface."""

    def setUp(self):
        self.tmpdir = tempfile.mkdtemp()
        self.jekyll_dir = os.path.join(self.tmpdir, "jekyll")
        self.rustkyll_dir = os.path.join(self.tmpdir, "rustkyll")
        os.makedirs(self.jekyll_dir)
        os.makedirs(self.rustkyll_dir)
        self.script = os.path.join(os.path.dirname(os.path.abspath(__file__)), "dom_compare.py")

    def tearDown(self):
        import shutil
        shutil.rmtree(self.tmpdir)

    def _write_file(self, base_dir, rel_path, content):
        full_path = os.path.join(base_dir, rel_path)
        os.makedirs(os.path.dirname(full_path), exist_ok=True)
        with open(full_path, "w") as f:
            f.write(content)

    def test_exit_0_on_match(self):
        html = '<html><body><p>Hello</p></body></html>'
        self._write_file(self.jekyll_dir, "index.html", html)
        self._write_file(self.rustkyll_dir, "index.html", html)
        result = subprocess.run(
            [sys.executable, self.script,
             "--jekyll-dir", self.jekyll_dir,
             "--rustkyll-dir", self.rustkyll_dir],
            capture_output=True, text=True
        )
        self.assertEqual(result.returncode, 0)

    def test_exit_1_on_diff(self):
        self._write_file(self.jekyll_dir, "index.html",
                         '<html><body><p>Jekyll</p></body></html>')
        self._write_file(self.rustkyll_dir, "index.html",
                         '<html><body><p>Rustkyll</p></body></html>')
        result = subprocess.run(
            [sys.executable, self.script,
             "--jekyll-dir", self.jekyll_dir,
             "--rustkyll-dir", self.rustkyll_dir],
            capture_output=True, text=True
        )
        self.assertEqual(result.returncode, 1)

    def test_output_file(self):
        html = '<html><body><p>Hello</p></body></html>'
        self._write_file(self.jekyll_dir, "index.html", html)
        self._write_file(self.rustkyll_dir, "index.html", html)
        output_path = os.path.join(self.tmpdir, "report.txt")
        result = subprocess.run(
            [sys.executable, self.script,
             "--jekyll-dir", self.jekyll_dir,
             "--rustkyll-dir", self.rustkyll_dir,
             "--output", output_path],
            capture_output=True, text=True
        )
        self.assertEqual(result.returncode, 0)
        self.assertTrue(os.path.exists(output_path))
        with open(output_path) as f:
            content = f.read()
        self.assertIn("Summary:", content)

    def test_missing_dir_exit_2(self):
        result = subprocess.run(
            [sys.executable, self.script,
             "--jekyll-dir", "/nonexistent",
             "--rustkyll-dir", "/also-nonexistent"],
            capture_output=True, text=True
        )
        self.assertEqual(result.returncode, 2)


if __name__ == "__main__":
    unittest.main()
