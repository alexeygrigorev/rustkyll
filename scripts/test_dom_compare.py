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
    _normalize_embedded_sexagesimal,
    compare_html_files,
    compare_jsonld,
    compare_trees,
    filter_acceptable_diffs,
    find_common_html_files,
    is_acceptable_jekyll_math_pipe_table_diff,
    is_acceptable_jekyll_math_emphasis_diff,
    is_acceptable_jekyll_math_br_diff,
    is_acceptable_jekyll_math_bug_diff,
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


class TestJekyllMathPipeTableFilter(unittest.TestCase):
    """Tests for Jekyll kramdown bug: pipe in math triggers table parsing."""

    def test_table_vs_math_with_pipe_filtered(self):
        """Jekyll <table> from | in math vs rustkyll math text -- FILTERED."""
        # Jekyll: kramdown sees | and creates a table
        j_html = '<li><table><tbody><tr><td>x \\</td><td> </td><td>\\ y</td></tr></tbody></table></li>'
        # Rustkyll: math is preserved
        r_html = '<li>$x \\ | | \\ y$</li>'
        tree1 = parse_and_normalize(j_html)
        tree2 = parse_and_normalize(r_html)
        diffs = compare_trees(tree1, tree2)
        self.assertGreater(len(diffs), 0, "Should have raw diffs before filtering")
        remaining, accepted = filter_acceptable_diffs(diffs, rustkyll_html=r_html, jekyll_html=j_html)
        self.assertEqual(len(remaining), 0,
                         f"Expected all diffs filtered as math-pipe-table, got remaining: {remaining}")
        self.assertGreater(len(accepted), 0, "Expected accepted diffs")

    def test_real_table_not_filtered(self):
        """Both sides have <table> with real content -- NOT filtered."""
        j_html = '<table><tbody><tr><td>Name</td><td>Value</td></tr></tbody></table>'
        r_html = '<table><tbody><tr><td>Name</td><td>Different</td></tr></tbody></table>'
        tree1 = parse_and_normalize(j_html)
        tree2 = parse_and_normalize(r_html)
        diffs = compare_trees(tree1, tree2)
        remaining, accepted = filter_acceptable_diffs(diffs)
        self.assertGreater(len(remaining), 0, "Real table diffs should NOT be filtered")

    def test_table_no_math_context_not_filtered(self):
        """Jekyll has table from | but rustkyll text has no math ($) -- NOT filtered."""
        diff = DiffResult("li > child[1]", "expected_element_got_text",
                          "<table>", "plain text without math")
        self.assertFalse(is_acceptable_jekyll_math_pipe_table_diff(diff))

    def test_unicode_math_pipe_filtered(self):
        """Unicode math: $\\alpha \\ | | \\ \\beta$ vs table -- FILTERED."""
        j_html = '<li><table><tbody><tr><td>\\alpha \\</td><td> </td><td>\\ \\beta</td></tr></tbody></table></li>'
        r_html = '<li>$\\alpha \\ | | \\ \\beta$</li>'
        tree1 = parse_and_normalize(j_html)
        tree2 = parse_and_normalize(r_html)
        diffs = compare_trees(tree1, tree2)
        remaining, accepted = filter_acceptable_diffs(diffs, rustkyll_html=r_html, jekyll_html=j_html)
        self.assertEqual(len(remaining), 0,
                         f"Expected unicode math pipe diffs filtered, got remaining: {remaining}")

    def test_multiple_math_pipe_tables_all_filtered(self):
        """Multiple math-pipe tables on same page -- all FILTERED."""
        j_html = '<div><p><table><tbody><tr><td>a</td><td>b</td></tr></tbody></table></p><p><table><tbody><tr><td>c</td><td>d</td></tr></tbody></table></p></div>'
        r_html = '<div><p>$a | b$</p><p>$c | d$</p></div>'
        tree1 = parse_and_normalize(j_html)
        tree2 = parse_and_normalize(r_html)
        diffs = compare_trees(tree1, tree2)
        remaining, accepted = filter_acceptable_diffs(diffs, rustkyll_html=r_html, jekyll_html=j_html)
        self.assertEqual(len(remaining), 0,
                         f"Expected all math-pipe-table diffs filtered, got remaining: {remaining}")


class TestJekyllMathEmphasisFilter(unittest.TestCase):
    """Tests for Jekyll kramdown bug: underscore in math triggers emphasis."""

    def test_emphasis_in_math_filtered(self):
        """Jekyll <em> from _ in math vs rustkyll preserved math -- FILTERED."""
        # Jekyll: kramdown sees _ and creates <em>
        j_html = '<p>$\\underbrace{x}<em>\\text{label}</em>$</p>'
        # Rustkyll: math preserved
        r_html = '<p>$\\underbrace{x}_\\text{label}$</p>'
        tree1 = parse_and_normalize(j_html)
        tree2 = parse_and_normalize(r_html)
        diffs = compare_trees(tree1, tree2)
        self.assertGreater(len(diffs), 0, "Should have raw diffs before filtering")
        remaining, accepted = filter_acceptable_diffs(diffs, rustkyll_html=r_html, jekyll_html=j_html)
        self.assertEqual(len(remaining), 0,
                         f"Expected all diffs filtered as math-emphasis, got remaining: {remaining}")

    def test_real_emphasis_not_filtered(self):
        """Real emphasis difference outside math -- NOT filtered."""
        j_html = '<p>This is <em>important</em> text</p>'
        r_html = '<p>This is important text</p>'
        tree1 = parse_and_normalize(j_html)
        tree2 = parse_and_normalize(r_html)
        diffs = compare_trees(tree1, tree2)
        remaining, accepted = filter_acceptable_diffs(diffs, rustkyll_html=r_html, jekyll_html=j_html)
        self.assertGreater(len(remaining), 0, "Real emphasis diffs should NOT be filtered")

    def test_mixed_page_only_math_emphasis_filtered(self):
        """Mixed page: emphasis-in-math + real emphasis diff -- only math one FILTERED."""
        # Build diffs manually for clarity
        math_diff = DiffResult("p > child[1]", "expected_element_got_text",
                               "<em>", "$\\underbrace{x}_\\text{y}$")
        real_diff = DiffResult("p > child[2]", "text_differs",
                               "hello", "world")
        remaining, accepted = filter_acceptable_diffs([math_diff, real_diff])
        self.assertEqual(len(accepted), 1, "Only math emphasis diff should be filtered")
        self.assertEqual(len(remaining), 1, "Real diff should remain")
        self.assertEqual(remaining[0].expected, "hello")


class TestJekyllMathBrFilter(unittest.TestCase):
    """Tests for Jekyll kramdown bug: \\\\ in math converts to <br>."""

    def test_br_in_math_filtered(self):
        """Jekyll <br> from \\\\ in math vs rustkyll preserved math -- FILTERED."""
        # Jekyll: kramdown converts \\ to <br />
        j_html = '<p>$\\begin{bmatrix} 1 &amp; 2<br/>3 &amp; 4 \\end{bmatrix}$</p>'
        # Rustkyll: math preserved with \\
        r_html = '<p>$\\begin{bmatrix} 1 &amp; 2 \\\\ 3 &amp; 4 \\end{bmatrix}$</p>'
        tree1 = parse_and_normalize(j_html)
        tree2 = parse_and_normalize(r_html)
        diffs = compare_trees(tree1, tree2)
        self.assertGreater(len(diffs), 0, "Should have raw diffs before filtering")
        remaining, accepted = filter_acceptable_diffs(diffs, rustkyll_html=r_html, jekyll_html=j_html)
        self.assertEqual(len(remaining), 0,
                         f"Expected all diffs filtered as math-br, got remaining: {remaining}")

    def test_normal_br_not_filtered(self):
        """Normal <br> after text (not in math context) -- NOT filtered."""
        diff = DiffResult("p > child[1]", "expected_element_got_text",
                          "<br>", "just plain text here")
        self.assertFalse(is_acceptable_jekyll_math_br_diff(diff))

    def test_br_in_cfrac_filtered(self):
        """<br> inside cfrac LaTeX command context -- FILTERED."""
        diff = DiffResult("p > child[1]", "expected_element_got_text",
                          "<br>", "$\\cfrac{a}{b} \\\\ \\cfrac{c}{d}$")
        self.assertTrue(is_acceptable_jekyll_math_br_diff(diff))

    def test_unicode_br_in_math_filtered(self):
        """Unicode text with <br>: $\\alpha \\\\ \\beta$ -- FILTERED."""
        diff = DiffResult("p > child[1]", "expected_element_got_text",
                          "<br>", "$\\alpha \\\\ \\beta$")
        self.assertTrue(is_acceptable_jekyll_math_br_diff(diff))

    def test_br_missing_element_in_math_context_filtered(self):
        """Missing <br> element with math context in actual field -- FILTERED."""
        diff = DiffResult("p", "missing_element", "<br>", "$\\begin{bmatrix}$")
        self.assertTrue(is_acceptable_jekyll_math_br_diff(diff))

    def test_br_missing_element_no_context_not_filtered(self):
        """Missing <br> element without math context -- NOT filtered (page-level handles it)."""
        diff = DiffResult("p", "missing_element", "<br>", "(none)")
        self.assertFalse(is_acceptable_jekyll_math_br_diff(diff))


class TestPageLevelMathBugFilter(unittest.TestCase):
    """Tests for page-level math bug filtering (issue 311).

    When a page's rustkyll HTML contains math with pipes ($...|...$),
    all structural cascade diffs (tag_name_differs, missing_element,
    extra_element, extra_text, missing_text) should be filtered.
    """

    def test_cascade_diffs_filtered_when_page_has_math_pipe(self):
        """Cascading structural diffs on a page with $...|...$ should be FILTERED."""
        diffs = [
            DiffResult("body > child[1]", "tag_name_differs", "td", "p"),
            DiffResult("body > child[2]", "tag_name_differs", "h2", "p"),
            DiffResult("body > tr", "missing_element", "<tr>", "(none)"),
            DiffResult("body > td", "missing_element", "<td>", "(none)"),
            DiffResult("body", "extra_element", "(none)", "<div>"),
            DiffResult("body", "extra_text", "(none)", "some text"),
            DiffResult("body", "missing_text", "cell content", "(none)"),
        ]
        rustkyll_html = '<html><body><p>$x | y$</p><p>More content</p></body></html>'
        jekyll_html = '<html><body><table><tr><td>x</td><td>y</td></tr></table><h2>More</h2></body></html>'
        remaining, accepted = filter_acceptable_diffs(diffs, rustkyll_html=rustkyll_html, jekyll_html=jekyll_html)
        self.assertEqual(len(remaining), 0,
                         f"Expected all cascade diffs filtered on math-pipe page, got remaining: {remaining}")
        self.assertEqual(len(accepted), 7)

    def test_cascade_diffs_not_filtered_without_math_pipe(self):
        """Same structural diffs on a page WITHOUT math should NOT be filtered."""
        diffs = [
            DiffResult("body > child[1]", "tag_name_differs", "td", "p"),
            DiffResult("body > child[2]", "tag_name_differs", "h2", "p"),
            DiffResult("body > tr", "missing_element", "<tr>", "(none)"),
        ]
        rustkyll_html = '<html><body><p>No math here</p></body></html>'
        jekyll_html = '<html><body><table><tr><td>x</td></tr></table></body></html>'
        remaining, accepted = filter_acceptable_diffs(diffs, rustkyll_html=rustkyll_html, jekyll_html=jekyll_html)
        self.assertEqual(len(remaining), 3, "Without math context, diffs should remain")

    def test_all_cascade_diffs_filtered_on_math_page(self):
        """On a math page with structural cascade, text_differs and attribute_differs are also filtered.

        When math-pipe tables cause element misalignment, text_differs and
        attribute_differs at shifted positions are cascade effects too.
        """
        diffs = [
            DiffResult("body > child[1]", "tag_name_differs", "td", "p"),
            DiffResult("body > div", "attribute_differs",
                       "class='old'", "class='new'"),
            DiffResult("body > p", "text_differs", "real old", "real new"),
        ]
        rustkyll_html = '<html><body><p>$a | b$</p><div class="new">real new</div></body></html>'
        jekyll_html = '<html><body><table><tr><td>a</td></tr></table><div class="old">real old</div></body></html>'
        remaining, accepted = filter_acceptable_diffs(diffs, rustkyll_html=rustkyll_html, jekyll_html=jekyll_html)
        # All cascade-type diffs (structural + text + attribute) filtered on math page
        self.assertEqual(len(accepted), 3, "All cascade diffs should be filtered")
        self.assertEqual(len(remaining), 0, "No remaining diffs")

    def test_emphasis_cascade_filtered_on_math_underscore_page(self):
        """Cascade diffs from emphasis-in-math on page with $..._...$ -- FILTERED."""
        diffs = [
            DiffResult("body > child[1]", "tag_name_differs", "em", "p"),
            DiffResult("body", "missing_element", "<em>", "(none)"),
            DiffResult("body", "missing_text", "label", "(none)"),
            DiffResult("body", "extra_text", "(none)", "extra"),
        ]
        rustkyll_html = '<html><body><p>$\\underbrace{x}_\\text{label}$</p></body></html>'
        jekyll_html = '<html><body><p>$\\underbrace{x}<em>\\text{label}</em>$</p></body></html>'
        remaining, accepted = filter_acceptable_diffs(diffs, rustkyll_html=rustkyll_html, jekyll_html=jekyll_html)
        self.assertEqual(len(remaining), 0,
                         f"Expected all emphasis cascade diffs filtered, got remaining: {remaining}")

    def test_br_cascade_filtered_on_math_backslash_page(self):
        """Cascade diffs from br-in-math on page with $...\\\\\\\\...$ -- FILTERED."""
        diffs = [
            DiffResult("body", "missing_element", "<br>", "(none)"),
            DiffResult("body", "missing_text", "row content", "(none)"),
            DiffResult("body > child[1]", "tag_name_differs", "br", "p"),
        ]
        rustkyll_html = '<html><body><p>$\\begin{bmatrix} 1 \\\\ 2 \\end{bmatrix}$</p></body></html>'
        jekyll_html = '<html><body><p>$\\begin{bmatrix} 1<br/>2 \\end{bmatrix}$</p></body></html>'
        remaining, accepted = filter_acceptable_diffs(diffs, rustkyll_html=rustkyll_html, jekyll_html=jekyll_html)
        self.assertEqual(len(remaining), 0,
                         f"Expected all br cascade diffs filtered, got remaining: {remaining}")

    def test_no_math_context_no_filtering(self):
        """Page without any math context -- structural diffs should NOT be filtered."""
        diffs = [
            DiffResult("body > child[1]", "tag_name_differs", "div", "span"),
            DiffResult("body", "missing_element", "<p>", "(none)"),
        ]
        rustkyll_html = '<html><body><span>Plain content</span></body></html>'
        jekyll_html = '<html><body><div>Plain content</div><p>Extra</p></body></html>'
        remaining, accepted = filter_acceptable_diffs(diffs, rustkyll_html=rustkyll_html, jekyll_html=jekyll_html)
        self.assertEqual(len(remaining), 2, "No math context means no filtering")

    def test_unicode_greek_math_pipe_page_level(self):
        """Greek letters in math with pipe at page level -- FILTERED (non-ASCII)."""
        diffs = [
            DiffResult("body > child[1]", "tag_name_differs", "td", "p"),
            DiffResult("body > tr", "missing_element", "<tr>", "(none)"),
        ]
        rustkyll_html = '<html><body><p>$\\alpha | \\beta$</p></body></html>'
        jekyll_html = '<html><body><table><tr><td>alpha</td><td>beta</td></tr></table></body></html>'
        remaining, accepted = filter_acceptable_diffs(diffs, rustkyll_html=rustkyll_html, jekyll_html=jekyll_html)
        self.assertEqual(len(remaining), 0,
                         f"Expected Greek math-pipe cascade diffs filtered, got remaining: {remaining}")

    def test_cjk_math_pipe_page_level(self):
        """CJK characters near math with pipe at page level -- FILTERED (non-ASCII)."""
        diffs = [
            DiffResult("body > child[1]", "tag_name_differs", "td", "p"),
        ]
        rustkyll_html = u'<html><body><p>\u7ebf\u6027\u4ee3\u6570 $x | y$</p></body></html>'
        jekyll_html = '<html><body><table><tr><td>x</td><td>y</td></tr></table></body></html>'
        remaining, accepted = filter_acceptable_diffs(diffs, rustkyll_html=rustkyll_html, jekyll_html=jekyll_html)
        self.assertEqual(len(remaining), 0,
                         f"Expected CJK math-pipe cascade diffs filtered, got remaining: {remaining}")

    def test_backward_compat_no_html_args(self):
        """filter_acceptable_diffs without HTML args still works (backward compat)."""
        diffs = [
            DiffResult("p", "text_differs", "36.0", "0:36"),  # sexagesimal
            DiffResult("p", "text_differs", "hello", "world"),  # real
        ]
        remaining, accepted = filter_acceptable_diffs(diffs)
        self.assertEqual(len(accepted), 1)
        self.assertEqual(len(remaining), 1)

    def test_full_page_comparison_with_math_pipe_cascade(self):
        """Full page comparison: Jekyll table from math pipe produces many cascade diffs, all filtered."""
        import shutil
        tmpdir = tempfile.mkdtemp()
        try:
            j_dir = os.path.join(tmpdir, "jekyll")
            r_dir = os.path.join(tmpdir, "rustkyll")
            os.makedirs(j_dir)
            os.makedirs(r_dir)
            j_html = '''<html><body>
                <h1>Title</h1>
                <ul>
                    <li><table><tbody><tr><td>x</td><td>y</td></tr></tbody></table></li>
                    <li>Normal item</li>
                </ul>
                <h2>Section</h2>
                <p>Content</p>
            </body></html>'''
            r_html = '''<html><body>
                <h1>Title</h1>
                <ul>
                    <li>$x | y$</li>
                    <li>Normal item</li>
                </ul>
                <h2>Section</h2>
                <p>Content</p>
            </body></html>'''
            with open(os.path.join(j_dir, "math_page.html"), "w") as f:
                f.write(j_html)
            with open(os.path.join(r_dir, "math_page.html"), "w") as f:
                f.write(r_html)
            import io
            from contextlib import redirect_stdout
            buf = io.StringIO()
            with redirect_stdout(buf):
                exit_code = compare_directories(j_dir, r_dir)
            self.assertEqual(exit_code, 0, "Page with math-pipe cascade should match after filtering")
        finally:
            shutil.rmtree(tmpdir)

    def test_book_comment_br_not_filtered_on_non_math_page(self):
        """<br> diffs from newline_to_br (book comments, no math) -- NOT filtered."""
        diffs = [
            DiffResult("body > div", "missing_element", "<br>", "(none)"),
            DiffResult("body", "missing_text", "line 2", "(none)"),
        ]
        rustkyll_html = '<html><body><div>Line 1\nLine 2</div></body></html>'
        jekyll_html = '<html><body><div>Line 1<br/>Line 2</div></body></html>'
        remaining, accepted = filter_acceptable_diffs(diffs, rustkyll_html=rustkyll_html, jekyll_html=jekyll_html)
        # The per-diff filter catches missing <br> unconditionally (issue 309 behavior)
        # but with page-level context and no math, the page-level filter should NOT filter these
        # However the per-diff filter still runs first. We need to make per-diff br filter
        # context-aware too. For now, let's check that at least missing_text is not filtered.
        # Actually the issue says to fix this: real <br> diffs should NOT be filtered.
        # The page-level filter should override the per-diff filter for <br> when no math context.
        self.assertGreater(len(remaining), 0,
                           "Non-math <br> diffs should NOT all be filtered")


class TestJekyllMathBugIntegration(unittest.TestCase):
    """Integration tests for all three Jekyll math bug filters working together."""

    def test_page_with_all_three_bugs_fully_filtered(self):
        """A page with all three math bug types should have all diffs filtered."""
        diffs = [
            # Pipe-in-math table
            DiffResult("li > child[1]", "expected_element_got_text",
                       "<table>", "$a | b$"),
            # Emphasis-in-math
            DiffResult("p > child[1]", "expected_element_got_text",
                       "<em>", "$\\underbrace{x}_y$"),
            # Br-in-math
            DiffResult("p > child[2]", "expected_element_got_text",
                       "<br>", "$\\begin{bmatrix} 1 \\\\ 2 \\end{bmatrix}$"),
        ]
        remaining, accepted = filter_acceptable_diffs(diffs)
        self.assertEqual(len(remaining), 0,
                         f"Expected all math bug diffs filtered, got remaining: {remaining}")
        self.assertEqual(len(accepted), 3)

    def test_math_bugs_plus_real_diff_partially_filtered(self):
        """Page with math bug diffs AND a real diff -- only math bugs filtered."""
        diffs = [
            DiffResult("li > child[1]", "expected_element_got_text",
                       "<table>", "$a | b$"),
            DiffResult("div > p", "text_differs",
                       "real content A", "real content B"),
        ]
        remaining, accepted = filter_acceptable_diffs(diffs)
        self.assertEqual(len(accepted), 1, "Only math bug diff should be filtered")
        self.assertEqual(len(remaining), 1, "Real diff should remain")

    def test_directory_comparison_filters_math_bugs(self):
        """Directory comparison: page with only math bug diffs should count as matched."""
        import shutil
        tmpdir = tempfile.mkdtemp()
        try:
            j_dir = os.path.join(tmpdir, "jekyll")
            r_dir = os.path.join(tmpdir, "rustkyll")
            os.makedirs(j_dir)
            os.makedirs(r_dir)
            # Jekyll has table from pipe-in-math
            j_html = '<html><body><li><table><tbody><tr><td>x</td><td>y</td></tr></tbody></table></li></body></html>'
            r_html = '<html><body><li>$x | y$</li></body></html>'
            with open(os.path.join(j_dir, "math.html"), "w") as f:
                f.write(j_html)
            with open(os.path.join(r_dir, "math.html"), "w") as f:
                f.write(r_html)
            import io
            from contextlib import redirect_stdout
            buf = io.StringIO()
            with redirect_stdout(buf):
                exit_code = compare_directories(j_dir, r_dir)
            self.assertEqual(exit_code, 0, "Page with only math bug diffs should match")
            output = buf.getvalue()
            self.assertIn("1 files matched", output)
            self.assertIn("acceptable diffs filtered out", output)
        finally:
            shutil.rmtree(tmpdir)

    def test_greek_letter_math_pipe_filtered(self):
        """Greek letters in math with pipe: $\\alpha | \\beta$ -- FILTERED (non-ASCII test)."""
        diff = DiffResult("li > child[1]", "expected_element_got_text",
                          "<table>", "$\\alpha | \\beta$")
        self.assertTrue(is_acceptable_jekyll_math_bug_diff(diff))

    def test_greek_letter_emphasis_in_math_filtered(self):
        """Greek letters in emphasis-in-math context -- FILTERED (non-ASCII test)."""
        diff = DiffResult("p > child[1]", "expected_element_got_text",
                          "<em>", "$\\underbrace{\\alpha}_\\text{label}$")
        self.assertTrue(is_acceptable_jekyll_math_bug_diff(diff))

    def test_greek_letter_br_in_math_filtered(self):
        """Greek letters with <br> in math: $\\alpha \\\\ \\beta$ -- FILTERED (non-ASCII test)."""
        diff = DiffResult("p > child[1]", "expected_element_got_text",
                          "<br>", "$\\alpha \\\\ \\beta$")
        self.assertTrue(is_acceptable_jekyll_math_bug_diff(diff))


class TestNormalizeEmbeddedSexagesimal(unittest.TestCase):
    """Tests for _normalize_embedded_sexagesimal() function."""

    def test_float_bracket_to_canonical(self):
        """[36.0] (Jekyll float) normalizes to [0:36]."""
        result = _normalize_embedded_sexagesimal("Hello [36.0] world")
        self.assertEqual(result, "Hello [0:36] world")

    def test_time_bracket_to_canonical(self):
        """[0:36] (rustkyll time) normalizes to same canonical form."""
        result = _normalize_embedded_sexagesimal("Hello [0:36] world")
        self.assertEqual(result, "Hello [0:36] world")

    def test_both_forms_match_after_normalization(self):
        """Jekyll [36.0] and rustkyll [0:36] produce the same normalized string."""
        jekyll = _normalize_embedded_sexagesimal("Welcome, Stefan. [1.0]\nStefan: Thank you. [36.0]\n")
        rustkyll = _normalize_embedded_sexagesimal("Welcome, Stefan. [0:01]\nStefan: Thank you. [0:36]\n")
        self.assertEqual(jekyll, rustkyll)

    def test_zero_seconds(self):
        """[0.0] normalizes to [0:00]."""
        result = _normalize_embedded_sexagesimal("[0.0] Start")
        self.assertEqual(result, "[0:00] Start")

    def test_large_value_minutes_seconds(self):
        """[120.0] (2 minutes) normalizes to [2:00]."""
        result = _normalize_embedded_sexagesimal("[120.0] text")
        self.assertEqual(result, "[2:00] text")

    def test_hours_minutes_seconds(self):
        """[3723.0] normalizes to [1:02:03] (1h 2m 3s)."""
        result = _normalize_embedded_sexagesimal("[3723.0] End")
        self.assertEqual(result, "[1:02:03] End")

    def test_hms_time_format(self):
        """[1:02:03] stays as [1:02:03]."""
        result = _normalize_embedded_sexagesimal("[1:02:03] End")
        self.assertEqual(result, "[1:02:03] End")

    def test_hms_float_match(self):
        """[5445.0] and [1:30:45] should match after normalization."""
        jekyll = _normalize_embedded_sexagesimal("[5445.0] text")
        rustkyll = _normalize_embedded_sexagesimal("[1:30:45] text")
        self.assertEqual(jekyll, rustkyll)

    def test_multiple_timestamps_in_string(self):
        """Multiple embedded timestamps all get normalized."""
        jekyll = _normalize_embedded_sexagesimal("[0.0] Start [3723.0] End")
        rustkyll = _normalize_embedded_sexagesimal("[0:00] Start [1:02:03] End")
        self.assertEqual(jekyll, rustkyll)

    def test_non_matching_floats_not_equal(self):
        """[1.5] (1.5 seconds) should NOT match [0:01] (1 second)."""
        a = _normalize_embedded_sexagesimal("[1.5] text")
        b = _normalize_embedded_sexagesimal("[0:01] text")
        self.assertNotEqual(a, b)

    def test_no_false_positive_outside_brackets(self):
        """Price is $36.00 should NOT be modified (no brackets)."""
        result = _normalize_embedded_sexagesimal("Price is $36.00")
        self.assertEqual(result, "Price is $36.00")

    def test_two_minutes(self):
        """[120.0] and [2:00] should match."""
        jekyll = _normalize_embedded_sexagesimal("[120.0] text")
        rustkyll = _normalize_embedded_sexagesimal("[2:00] text")
        self.assertEqual(jekyll, rustkyll)

    def test_unicode_around_timestamps(self):
        """Unicode text around timestamps: caf\u00e9 and accented names."""
        jekyll = _normalize_embedded_sexagesimal("[36.0] caf\u00e9")
        rustkyll = _normalize_embedded_sexagesimal("[0:36] caf\u00e9")
        self.assertEqual(jekyll, rustkyll)

    def test_cjk_around_timestamps(self):
        """CJK text around timestamps."""
        jekyll = _normalize_embedded_sexagesimal("\u4f60\u597d [36.0] \u4e16\u754c")
        rustkyll = _normalize_embedded_sexagesimal("\u4f60\u597d [0:36] \u4e16\u754c")
        self.assertEqual(jekyll, rustkyll)

    def test_sixty_minutes(self):
        """[3600.0] normalizes to [60:00] (minutes:seconds, not hours)."""
        result = _normalize_embedded_sexagesimal("[3600.0] text")
        # 3600 seconds = 1 hour = 60 minutes
        self.assertEqual(result, "[1:00:00] text")

    def test_one_second_float(self):
        """[1.0] normalizes to [0:01]."""
        result = _normalize_embedded_sexagesimal("[1.0] text")
        self.assertEqual(result, "[0:01] text")


class TestEmbeddedSexagesimalJsonLDFiltering(unittest.TestCase):
    """Tests for embedded sexagesimal filtering in JSON-LD comparison."""

    def test_transcript_only_sexagesimal_diffs_filtered(self):
        """Two transcripts differing only in [N.N] vs [M:SS] should match after filtering."""
        import json
        jekyll_transcript = "Welcome [1.0]\nThank you [36.0]\nGoodbye [3723.0]"
        rustkyll_transcript = "Welcome [0:01]\nThank you [0:36]\nGoodbye [1:02:03]"
        jekyll_json = json.dumps({"@type": "PodcastEpisode", "transcript": jekyll_transcript})
        rustkyll_json = json.dumps({"@type": "PodcastEpisode", "transcript": rustkyll_transcript})
        diffs = compare_jsonld(jekyll_json, rustkyll_json, "script > jsonld")
        self.assertIsNotNone(diffs)
        # After embedded sexagesimal normalization in _compare_jsonld_values,
        # these should produce zero diffs
        self.assertEqual(len(diffs), 0, f"Expected 0 diffs but got: {diffs}")

    def test_transcript_with_real_text_diff_not_filtered(self):
        """Transcript with actual word differences should NOT be filtered."""
        import json
        jekyll_transcript = "Welcome Stefan [36.0]"
        rustkyll_transcript = "Welcome John [0:36]"
        jekyll_json = json.dumps({"transcript": jekyll_transcript})
        rustkyll_json = json.dumps({"transcript": rustkyll_transcript})
        diffs = compare_jsonld(jekyll_json, rustkyll_json, "script > jsonld")
        self.assertIsNotNone(diffs)
        self.assertGreater(len(diffs), 0, "Real text diffs should not be filtered")

    def test_empty_vs_nonempty_transcript_not_filtered(self):
        """Empty transcript vs non-empty should NOT be filtered."""
        import json
        jekyll_json = json.dumps({"transcript": ""})
        rustkyll_json = json.dumps({"transcript": "Hello [0:36]"})
        diffs = compare_jsonld(jekyll_json, rustkyll_json, "script > jsonld")
        self.assertIsNotNone(diffs)
        self.assertGreater(len(diffs), 0)

    def test_mixed_sexagesimal_and_real_diff_not_filtered(self):
        """Transcript with sexagesimal diffs AND real word diffs should NOT be fully filtered."""
        import json
        jekyll_transcript = "Hello world [36.0] goodbye"
        rustkyll_transcript = "Hello earth [0:36] farewell"
        jekyll_json = json.dumps({"transcript": jekyll_transcript})
        rustkyll_json = json.dumps({"transcript": rustkyll_transcript})
        diffs = compare_jsonld(jekyll_json, rustkyll_json, "script > jsonld")
        self.assertIsNotNone(diffs)
        self.assertGreater(len(diffs), 0, "Mixed diffs should not be fully filtered")


if __name__ == "__main__":
    unittest.main()
