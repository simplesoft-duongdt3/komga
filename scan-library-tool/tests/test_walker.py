"""Tests for the filesystem walker (PDF-only)."""

import tempfile
import os
import unittest
from pathlib import Path
from walker import walk_library


class TestWalker(unittest.TestCase):
    def _make_file(self, dir_path: Path, name: str, content: str = "mock"):
        f = dir_path / name
        f.write_text(content)
        return f

    def test_empty_directory(self):
        with tempfile.TemporaryDirectory() as tmp:
            result = walk_library(tmp)
            self.assertEqual(result, {"series": {}, "oneshots": []})

    def test_series_with_pdf_books(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            series_a = root / "Series A"
            series_a.mkdir()
            self._make_file(series_a, "Chapter 01.pdf")
            self._make_file(series_a, "Chapter 02.pdf")

            result = walk_library(str(root))

            self.assertEqual(len(result["series"]), 1)
            s_url = series_a.as_uri()
            self.assertIn(s_url, result["series"])
            s_data = result["series"][s_url]
            self.assertEqual(s_data["name"], "Series A")
            self.assertEqual(len(s_data["books"]), 2)
            names = {b["name"] for b in s_data["books"]}
            self.assertEqual(names, {"Chapter 01", "Chapter 02"})

    def test_ignores_non_pdf_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            series_a = root / "Series A"
            series_a.mkdir()
            self._make_file(series_a, "Chapter 01.cbz")
            self._make_file(series_a, "Chapter 02.epub")
            self._make_file(series_a, "Chapter 03.pdf")

            result = walk_library(str(root))

            self.assertEqual(len(result["series"]), 1)
            books = list(result["series"].values())[0]["books"]
            self.assertEqual(len(books), 1)
            self.assertEqual(books[0]["name"], "Chapter 03")

    def test_oneshot_pdf_at_root(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self._make_file(root, "OneShot.pdf")

            result = walk_library(str(root))

            self.assertEqual(len(result["series"]), 0)
            self.assertEqual(len(result["oneshots"]), 1)
            self.assertEqual(result["oneshots"][0]["name"], "OneShot")

    def test_oneshots_in_dedicated_directory(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            oneshots = root / "Oneshots"
            oneshots.mkdir()
            self._make_file(oneshots, "OneShot.pdf")

            result = walk_library(str(root), oneshots_dir="Oneshots")

            self.assertEqual(len(result["oneshots"]), 1)
            self.assertEqual(result["oneshots"][0]["name"], "OneShot")

    def test_directory_without_pdf_books_excluded(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            series_a = root / "No PDFs"
            series_a.mkdir()
            self._make_file(series_a, "info.txt")

            result = walk_library(str(root))
            self.assertEqual(len(result["series"]), 0)

    def test_url_encoding(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            series_a = root / "Series With Spaces"
            series_a.mkdir()
            self._make_file(series_a, "Chapter 01.pdf")

            result = walk_library(str(root))

            s_data = list(result["series"].values())[0]
            self.assertIn("Series%20With%20Spaces", s_data["url"])

    def test_file_has_required_fields(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            series_a = root / "Series A"
            series_a.mkdir()
            self._make_file(series_a, "Chapter 01.pdf")

            result = walk_library(str(root))

            books = list(result["series"].values())[0]["books"]
            b = books[0]
            self.assertIn("url", b)
            self.assertTrue(b["url"].startswith("file://"))
            self.assertIn("name", b)
            self.assertIn("file_size", b)
            self.assertGreater(b["file_size"], 0)
            self.assertIn("file_last_modified", b)
            self.assertIn("+00:00", b["file_last_modified"])

    def test_hash_files_enabled(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            series_a = root / "Series A"
            series_a.mkdir()
            self._make_file(series_a, "Chapter 01.pdf", "hello pdf")

            result = walk_library(str(root), hash_files=True)

            books = list(result["series"].values())[0]["books"]
            b = books[0]
            self.assertIn("file_hash", b)
            # SHA-256 of "hello pdf"
            self.assertEqual(len(b["file_hash"]), 64)
            self.assertEqual(
                b["file_hash"],
                "9f275d73a74baf528734b92128a320df66ae66dab4935c842d8c3879d498e3f4",
            )

    def test_hash_files_disabled(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            series_a = root / "Series A"
            series_a.mkdir()
            self._make_file(series_a, "Chapter 01.pdf", "hello")

            result = walk_library(str(root), hash_files=False)

            books = list(result["series"].values())[0]["books"]
            b = books[0]
            self.assertNotIn("file_hash", b)

