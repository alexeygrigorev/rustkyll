#!/usr/bin/env -S uv run python
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
    _is_build_time_only_diff,
    compare_html_files,
    compare_jsonld,
    compare_trees,
    filter_acceptable_diffs,
    find_common_html_files,
    is_acceptable_sexagesimal_diff,
    is_acceptable_trailing_newline_diff,
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


class TestHTMLFragments(unittest.TestCase):
    """Tests for HTML fragments without <html> root element.

    Theme sites (e.g. just-the-docs) may produce HTML without proper
    <html> root. DOM comparison must handle these gracefully.
    """

    def test_fragment_identical(self):
        """Two identical HTML fragments (no <html> root) should have 0 diffs."""
        html = '<div class="container"><h1>Hello</h1><p>World</p></div>'
        tree1 = parse_and_normalize(html)
        tree2 = parse_and_normalize(html)
        diffs = compare_trees(tree1, tree2)
        self.assertEqual(len(diffs), 0, f"Expected no diffs, got: {diffs}")

    def test_fragment_with_differences(self):
        """HTML fragments with differences should detect them."""
        html1 = '<div><h1>Jekyll</h1><p>Content A</p></div>'
        html2 = '<div><h1>Rustkyll</h1><p>Content B</p></div>'
        tree1 = parse_and_normalize(html1)
        tree2 = parse_and_normalize(html2)
        diffs = compare_trees(tree1, tree2)
        self.assertGreater(len(diffs), 0, "Expected diffs in fragments")
        has_text_diff = any(d.diff_type == "text_differs" for d in diffs)
        self.assertTrue(has_text_diff, f"Expected text_differs, got: {diffs}")

    def test_fragment_multiple_roots(self):
        """Multiple root-level elements (no single <html> root) should work."""
        html = '<header>Nav</header><main>Content</main><footer>Foot</footer>'
        tree1 = parse_and_normalize(html)
        tree2 = parse_and_normalize(html)
        diffs = compare_trees(tree1, tree2)
        self.assertEqual(len(diffs), 0, f"Expected no diffs, got: {diffs}")

    def test_fragment_vs_fragment_missing_element(self):
        """Missing element in a fragment should be detected."""
        html1 = '<header>Nav</header><main>Content</main><footer>Foot</footer>'
        html2 = '<header>Nav</header><main>Content</main>'
        tree1 = parse_and_normalize(html1)
        tree2 = parse_and_normalize(html2)
        diffs = compare_trees(tree1, tree2)
        self.assertGreater(len(diffs), 0, "Expected missing element diff")
        has_missing = any("missing" in d.diff_type for d in diffs)
        self.assertTrue(has_missing, f"Expected missing element, got: {diffs}")

    def test_bare_text_fragment(self):
        """Bare text content (no tags at all) should be handled."""
        html = 'Just some plain text'
        tree1 = parse_and_normalize(html)
        tree2 = parse_and_normalize(html)
        diffs = compare_trees(tree1, tree2)
        self.assertEqual(len(diffs), 0, f"Expected no diffs, got: {diffs}")

    def test_fragment_with_doctype_only(self):
        """Fragment with doctype but no <html> tag should work."""
        html1 = '<!DOCTYPE html><div>Content</div>'
        html2 = '<!DOCTYPE html><div>Content</div>'
        tree1 = parse_and_normalize(html1)
        tree2 = parse_and_normalize(html2)
        diffs = compare_trees(tree1, tree2)
        self.assertEqual(len(diffs), 0, f"Expected no diffs, got: {diffs}")

    def test_fragment_file_comparison(self):
        """compare_html_files should work with fragment HTML files."""
        import tempfile
        import shutil
        tmpdir = tempfile.mkdtemp()
        try:
            j_path = os.path.join(tmpdir, "j.html")
            r_path = os.path.join(tmpdir, "r.html")
            with open(j_path, "w") as f:
                f.write('<div class="content"><p>Hello</p></div>')
            with open(r_path, "w") as f:
                f.write('<div class="content"><p>Hello</p></div>')
            diffs = compare_html_files(j_path, r_path)
            self.assertEqual(len(diffs), 0, f"Expected no diffs, got: {diffs}")
        finally:
            shutil.rmtree(tmpdir)

    def test_fragment_directory_comparison(self):
        """compare_directories should work when all files are HTML fragments."""
        import shutil
        tmpdir = tempfile.mkdtemp()
        try:
            j_dir = os.path.join(tmpdir, "jekyll")
            r_dir = os.path.join(tmpdir, "rustkyll")
            os.makedirs(j_dir)
            os.makedirs(r_dir)
            fragment = '<nav>Menu</nav><main><h1>Page</h1></main>'
            with open(os.path.join(j_dir, "index.html"), "w") as f:
                f.write(fragment)
            with open(os.path.join(r_dir, "index.html"), "w") as f:
                f.write(fragment)
            exit_code = compare_directories(j_dir, r_dir)
            self.assertEqual(exit_code, 0)
        finally:
            shutil.rmtree(tmpdir)


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


class TestSexagesimalAllowlist(unittest.TestCase):
    """Tests for the sexagesimal timestamp allowlist in DOM comparison."""

    def test_sexagesimal_float_vs_time_is_acceptable(self):
        """A diff where Jekyll shows '36.0' and rustkyll shows '0:36' is acceptable."""
        diff = DiffResult("some > path", "text_differs", "36.0", "0:36")
        self.assertTrue(is_acceptable_sexagesimal_diff(diff))

    def test_sexagesimal_large_float_vs_time_is_acceptable(self):
        """5400.0 vs 1:30:00 is acceptable."""
        diff = DiffResult("some > path", "text_differs", "5400.0", "1:30:00")
        self.assertTrue(is_acceptable_sexagesimal_diff(diff))

    def test_non_sexagesimal_text_diff_not_acceptable(self):
        """Regular text differences should not be filtered."""
        diff = DiffResult("some > path", "text_differs", "hello", "world")
        self.assertFalse(is_acceptable_sexagesimal_diff(diff))

    def test_attribute_diff_not_acceptable(self):
        """Attribute diffs should not be filtered even if values look sexagesimal."""
        diff = DiffResult("some > path", "attribute_differs", "36.0", "0:36")
        self.assertFalse(is_acceptable_sexagesimal_diff(diff))

    def test_filter_acceptable_diffs_separates_correctly(self):
        """filter_acceptable_diffs should separate acceptable from real diffs."""
        diffs = [
            DiffResult("p1", "text_differs", "36.0", "0:36"),     # acceptable
            DiffResult("p2", "text_differs", "hello", "world"),    # real diff
            DiffResult("p3", "text_differs", "5400.0", "1:30:00"), # acceptable
            DiffResult("p4", "missing_element", "<p>", "(none)"),  # real diff
        ]
        remaining, accepted = filter_acceptable_diffs(diffs)
        self.assertEqual(len(accepted), 2)
        self.assertEqual(len(remaining), 2)
        self.assertEqual(remaining[0].expected, "hello")
        self.assertEqual(remaining[1].diff_type, "missing_element")

    def test_zero_timestamp_acceptable(self):
        """0.0 vs 0:00 is acceptable."""
        diff = DiffResult("path", "text_differs", "0.0", "0:00")
        self.assertTrue(is_acceptable_sexagesimal_diff(diff))

    def test_integer_float_vs_time_acceptable(self):
        """12.0 vs 0:12 is acceptable."""
        diff = DiffResult("path", "text_differs", "12.0", "0:12")
        self.assertTrue(is_acceptable_sexagesimal_diff(diff))

    def test_url_not_treated_as_sexagesimal(self):
        """URLs should not be treated as sexagesimal."""
        diff = DiffResult("path", "text_differs", "https://example.com", "0:36")
        self.assertFalse(is_acceptable_sexagesimal_diff(diff))

    def test_directory_comparison_filters_sexagesimal(self):
        """Directory comparison should filter sexagesimal diffs and still pass."""
        import shutil
        tmpdir = tempfile.mkdtemp()
        try:
            j_dir = os.path.join(tmpdir, "jekyll")
            r_dir = os.path.join(tmpdir, "rustkyll")
            os.makedirs(j_dir)
            os.makedirs(r_dir)
            # Jekyll shows float, rustkyll shows time string
            j_html = '<html><body><p>36.0</p></body></html>'
            r_html = '<html><body><p>0:36</p></body></html>'
            with open(os.path.join(j_dir, "index.html"), "w") as f:
                f.write(j_html)
            with open(os.path.join(r_dir, "index.html"), "w") as f:
                f.write(r_html)
            exit_code = compare_directories(j_dir, r_dir)
            # Should pass because the only diff is an acceptable sexagesimal diff
            self.assertEqual(exit_code, 0)
        finally:
            shutil.rmtree(tmpdir)


class TestBuildTimeDiffFiltering(unittest.TestCase):
    """Tests for Bug A: endDate/startDate build-time filtering."""

    def test_same_date_different_time_is_build_diff(self):
        """Same date, different time = build-time diff (existing behavior)."""
        self.assertTrue(_is_build_time_only_diff(
            '2026-03-21 07:24:03 +0100', '2026-03-21 07:24:38 +0100'))

    def test_different_date_same_tz_is_build_diff(self):
        """Different date (2 days apart), same timezone = build-time diff.
        Builds on different days should still be filtered."""
        self.assertTrue(_is_build_time_only_diff(
            '2026-03-21 07:24:03 +0100', '2026-03-23 09:47:32 +0100'))

    def test_different_month_is_not_build_diff(self):
        """Different month = real diff, not a build-time artifact."""
        self.assertFalse(_is_build_time_only_diff(
            '2026-02-21 07:24:03 +0100', '2026-03-23 09:47:32 +0100'))

    def test_different_year_is_not_build_diff(self):
        """Different year = real diff."""
        self.assertFalse(_is_build_time_only_diff(
            '2025-03-21 07:24:03 +0100', '2026-03-21 09:47:32 +0100'))

    def test_different_tz_is_not_build_diff(self):
        """Different timezone = real diff."""
        self.assertFalse(_is_build_time_only_diff(
            '2026-03-21 07:24:03 +0100', '2026-03-23 09:47:32 +0200'))

    def test_iso_format_different_date_is_build_diff(self):
        """ISO 8601 format with T separator, different date, same tz = build diff."""
        self.assertTrue(_is_build_time_only_diff(
            '2026-03-21T07:24:03+01:00', '2026-03-23T09:47:32+01:00'))

    def test_iso_format_different_month_is_not_build_diff(self):
        """ISO 8601 format, different month = real diff."""
        self.assertFalse(_is_build_time_only_diff(
            '2026-02-21T07:24:03+01:00', '2026-03-23T09:47:32+01:00'))

    def test_non_datetime_strings_not_build_diff(self):
        """Non-datetime strings should not match."""
        self.assertFalse(_is_build_time_only_diff('hello', 'world'))

    def test_jsonld_enddate_filtered_in_comparison(self):
        """JSON-LD comparison should filter endDate build-time diffs (different days)."""
        import json
        j_obj = {"@type": "Event", "name": "Podcast", "endDate": "2026-03-21 07:24:03 +0100"}
        r_obj = {"@type": "Event", "name": "Podcast", "endDate": "2026-03-23 09:47:32 +0100"}
        j_text = json.dumps(j_obj)
        r_text = json.dumps(r_obj)
        diffs = compare_jsonld(j_text, r_text, "script > jsonld")
        self.assertEqual(len(diffs), 0, f"Expected no diffs (build-time filtered), got: {diffs}")

    def test_jsonld_startdate_filtered_in_comparison(self):
        """JSON-LD comparison should filter startDate build-time diffs."""
        import json
        j_obj = {"@type": "Event", "startDate": "2026-03-21 07:24:03 +0100",
                 "endDate": "2026-03-21 07:24:03 +0100"}
        r_obj = {"@type": "Event", "startDate": "2026-03-23 09:47:32 +0100",
                 "endDate": "2026-03-23 09:47:32 +0100"}
        j_text = json.dumps(j_obj)
        r_text = json.dumps(r_obj)
        diffs = compare_jsonld(j_text, r_text, "script > jsonld")
        self.assertEqual(len(diffs), 0, f"Expected no diffs, got: {diffs}")

    def test_jsonld_enddate_real_diff_not_filtered(self):
        """JSON-LD endDate with month difference should NOT be filtered."""
        import json
        j_obj = {"@type": "Event", "endDate": "2026-02-21 07:24:03 +0100"}
        r_obj = {"@type": "Event", "endDate": "2026-03-23 09:47:32 +0100"}
        j_text = json.dumps(j_obj)
        r_text = json.dumps(r_obj)
        diffs = compare_jsonld(j_text, r_text, "script > jsonld")
        self.assertGreater(len(diffs), 0, "Expected real diff for month difference")

    def test_full_html_enddate_only_diff_filtered(self):
        """Full HTML page with only endDate build-time diff should match after filtering."""
        import json
        j_jsonld = json.dumps({"@type": "Event", "name": "Podcast",
                               "endDate": "2026-03-21 07:24:03 +0100"})
        r_jsonld = json.dumps({"@type": "Event", "name": "Podcast",
                               "endDate": "2026-03-23 09:47:32 +0100"})
        j_html = f'<html><head><script type="application/ld+json">{j_jsonld}</script></head><body><p>Content</p></body></html>'
        r_html = f'<html><head><script type="application/ld+json">{r_jsonld}</script></head><body><p>Content</p></body></html>'
        tree1 = parse_and_normalize(j_html)
        tree2 = parse_and_normalize(r_html)
        diffs = compare_trees(tree1, tree2)
        remaining, accepted = filter_acceptable_diffs(diffs)
        self.assertEqual(len(remaining), 0,
                         f"Expected no remaining diffs after filtering, got: {remaining}")


class TestTruncatedDescriptionTrailingNewline(unittest.TestCase):
    """Tests for Bug B: truncated description trailing newline filtering."""

    def test_long_description_trailing_newline_filtered(self):
        """350-char description where rustkyll adds trailing newline should be filtered."""
        import json
        desc = "A" * 350
        j_obj = {"@type": "Article", "description": desc}
        r_obj = {"@type": "Article", "description": desc + "\n"}
        j_text = json.dumps(j_obj)
        r_text = json.dumps(r_obj)
        diffs = compare_jsonld(j_text, r_text, "script > jsonld")
        remaining, accepted = filter_acceptable_diffs(diffs)
        self.assertEqual(len(remaining), 0,
                         f"Expected trailing newline diff to be filtered, got remaining: {remaining}")
        self.assertGreater(len(accepted), 0, "Expected at least one accepted diff")

    def test_long_description_real_diff_not_filtered(self):
        """350-char description with text difference at position 250 should NOT be filtered."""
        import json
        desc_j = "A" * 250 + "B" * 100
        desc_r = "A" * 250 + "C" * 100
        j_obj = {"@type": "Article", "description": desc_j}
        r_obj = {"@type": "Article", "description": desc_r}
        j_text = json.dumps(j_obj)
        r_text = json.dumps(r_obj)
        diffs = compare_jsonld(j_text, r_text, "script > jsonld")
        remaining, accepted = filter_acceptable_diffs(diffs)
        self.assertGreater(len(remaining), 0,
                           "Expected real diff to remain unfiltered")

    def test_short_description_trailing_newline_filtered(self):
        """199-char description + trailing newline (under truncation) still filtered."""
        import json
        desc = "B" * 199
        j_obj = {"@type": "Article", "description": desc}
        r_obj = {"@type": "Article", "description": desc + "\n"}
        j_text = json.dumps(j_obj)
        r_text = json.dumps(r_obj)
        diffs = compare_jsonld(j_text, r_text, "script > jsonld")
        remaining, accepted = filter_acceptable_diffs(diffs)
        self.assertEqual(len(remaining), 0,
                         f"Expected trailing newline diff to be filtered, got: {remaining}")

    def test_exactly_200_char_description_trailing_newline_filtered(self):
        """Exactly 200-char description + trailing newline should be filtered."""
        import json
        desc = "C" * 200
        j_obj = {"@type": "Article", "description": desc}
        r_obj = {"@type": "Article", "description": desc + "\n"}
        j_text = json.dumps(j_obj)
        r_text = json.dumps(r_obj)
        diffs = compare_jsonld(j_text, r_text, "script > jsonld")
        remaining, accepted = filter_acceptable_diffs(diffs)
        self.assertEqual(len(remaining), 0,
                         f"Expected trailing newline diff to be filtered, got: {remaining}")

    def test_unicode_description_trailing_newline_filtered(self):
        """Long description with non-ASCII/Unicode content + trailing newline should be filtered."""
        import json
        # Mix of emoji, CJK, accented chars
        desc = "Beschreibung mit Umlauten: " + "\u00e4\u00f6\u00fc\u00df" * 50 + " \u4e16\u754c \U0001f600"
        j_obj = {"@type": "Article", "description": desc}
        r_obj = {"@type": "Article", "description": desc + "\n"}
        j_text = json.dumps(j_obj)
        r_text = json.dumps(r_obj)
        diffs = compare_jsonld(j_text, r_text, "script > jsonld")
        remaining, accepted = filter_acceptable_diffs(diffs)
        self.assertEqual(len(remaining), 0,
                         f"Expected unicode trailing newline diff to be filtered, got: {remaining}")

    def test_full_html_long_description_trailing_newline_filtered(self):
        """Full HTML with long description trailing newline in JSON-LD should be filtered."""
        import json
        desc = "D" * 350
        j_jsonld = json.dumps({"@type": "Article", "description": desc})
        r_jsonld = json.dumps({"@type": "Article", "description": desc + "\n"})
        j_html = f'<html><head><script type="application/ld+json">{j_jsonld}</script></head><body><p>Content</p></body></html>'
        r_html = f'<html><head><script type="application/ld+json">{r_jsonld}</script></head><body><p>Content</p></body></html>'
        tree1 = parse_and_normalize(j_html)
        tree2 = parse_and_normalize(r_html)
        diffs = compare_trees(tree1, tree2)
        remaining, accepted = filter_acceptable_diffs(diffs)
        self.assertEqual(len(remaining), 0,
                         f"Expected no remaining diffs, got: {remaining}")
        self.assertGreater(len(accepted), 0, "Expected at least one accepted diff")


if __name__ == "__main__":
    unittest.main()
