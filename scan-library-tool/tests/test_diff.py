"""Tests for the diff engine."""

import unittest
from diff import Diff, compute, group_new_books_by_series, _normalize_file_url


class TestNormalizeFileUrl(unittest.TestCase):
    """Test URL normalization with real-world data from Komga DB vs Python FS."""

    def test_java_db_single_slash(self):
        """Java URL.toString() on JDK 21 produces 'file:/path'."""
        self.assertEqual(
            _normalize_file_url("file:/data/library-sample/sample-series/"),
            "file:///data/library-sample/sample-series",
        )

    def test_java_db_single_slash_no_trailing(self):
        self.assertEqual(
            _normalize_file_url("file:/data/library-sample/sample-series"),
            "file:///data/library-sample/sample-series",
        )

    def test_python_fs_triple_slash(self):
        """Python Path.as_uri() produces 'file:///path'."""
        self.assertEqual(
            _normalize_file_url("file:///data/library-sample/sample-series"),
            "file:///data/library-sample/sample-series",
        )

    def test_python_fs_trailing_slash(self):
        self.assertEqual(
            _normalize_file_url("file:///data/library-sample/sample-series/"),
            "file:///data/library-sample/sample-series",
        )

    def test_db_and_fs_match_after_normalization(self):
        """After normalization, DB and FS URLs for the same path are equal."""
        db_url = _normalize_file_url("file:/data/library-sample/sample-series/")
        fs_url = _normalize_file_url("file:///data/library-sample/sample-series")
        self.assertEqual(db_url, fs_url)
        self.assertEqual(db_url, "file:///data/library-sample/sample-series")

    def test_book_urls_match(self):
        """Book URLs should also normalize consistently."""
        db_book = _normalize_file_url(
            "file:/data/library-sample/sample-series/sample_random_cool.pdf"
        )
        fs_book = _normalize_file_url(
            "file:///data/library-sample/sample-series/sample_random_cool.pdf"
        )
        self.assertEqual(db_book, fs_book)

    def test_non_file_url_passthrough(self):
        self.assertEqual(_normalize_file_url("http://example.com"), "http://example.com")

    def test_empty_host_file_url(self):
        """file://host/path preserves host in path."""
        self.assertEqual(
            _normalize_file_url("file://localhost/data/foo"),
            "file:///localhost/data/foo",
        )


class TestDiffWithRealData(unittest.TestCase):
    """Integration test using the exact URLs from the debug output."""

    def test_normalized_diff_matches_correctly(self):
        # DB data (Java-style single-slash URLs with trailing slashes on dirs)
        db_series = [
            {"id": "s1", "name": "sample-series",
             "url": "file:/data/library-sample/sample-series/",
             "file_last_modified": "2026-01-01T00:00:00+00:00"},
        ]
        db_books = [
            {"id": "b1", "name": "sample_random_cool",
             "url": "file:/data/library-sample/sample-series/sample_random_cool.pdf",
             "file_last_modified": "2026-01-01T00:00:00+00:00",
             "file_size": 1000, "file_hash": "", "series_id": "s1"},
            {"id": "b2", "name": "sample_random_sage",
             "url": "file:/data/library-sample/sample-series/sample_random_sage.pdf",
             "file_last_modified": "2026-01-01T00:00:00+00:00",
             "file_size": 2000, "file_hash": "", "series_id": "s1"},
            {"id": "b3", "name": "sample_random_warm",
             "url": "file:/data/library-sample/sample-series/sample_random_warm.pdf",
             "file_last_modified": "2026-01-01T00:00:00+00:00",
             "file_size": 3000, "file_hash": "", "series_id": "s1"},
        ]

        # FS data (Python-style triple-slash URLs, no trailing slash on dirs)
        fs = {
            "series": {
                "file:///data/library-sample/sample-series": {
                    "url": "file:///data/library-sample/sample-series",
                    "name": "sample-series",
                    "file_last_modified": "2026-01-01T00:00:00+00:00",
                    "books": [
                        {"url": "file:///data/library-sample/sample-series/sample_random_cool.pdf",
                         "name": "sample_random_cool", "file_size": 1000,
                         "file_last_modified": "2026-01-01T00:00:00+00:00"},
                        {"url": "file:///data/library-sample/sample-series/sample_random_sage.pdf",
                         "name": "sample_random_sage", "file_size": 2000,
                         "file_last_modified": "2026-01-01T00:00:00+00:00"},
                        {"url": "file:///data/library-sample/sample-series/sample_random_warm.pdf",
                         "name": "sample_random_warm", "file_size": 3000,
                         "file_last_modified": "2026-01-01T00:00:00+00:00"},
                    ],
                },
                "file:///data/library-sample/sample-series-2": {
                    "url": "file:///data/library-sample/sample-series-2",
                    "name": "sample-series-2",
                    "file_last_modified": "2026-01-02T00:00:00+00:00",
                    "books": [
                        {"url": "file:///data/library-sample/sample-series-2/sample_random_cool.pdf",
                         "name": "sample_random_cool", "file_size": 1000,
                         "file_last_modified": "2026-01-02T00:00:00+00:00"},
                        {"url": "file:///data/library-sample/sample-series-2/sample_random_sage.pdf",
                         "name": "sample_random_sage", "file_size": 2000,
                         "file_last_modified": "2026-01-02T00:00:00+00:00"},
                    ],
                },
            },
            "oneshots": [],
        }

        d = compute(db_series, db_books, fs)

        # After normalization, the existing series should MATCH
        # sample-series exists in both DB and FS → 0 new, 0 deleted
        # sample-series-2 is new (FS only) → 1 new
        # All 3 DB books exist in FS with same mtime/size → 0 changed, 0 deleted
        # 2 new books in sample-series-2 → 2 new (but in new series, excluded)
        self.assertEqual(len(d.new_series), 1)
        self.assertEqual(d.new_series[0]["name"], "sample-series-2")
        self.assertEqual(len(d.deleted_series), 0)
        self.assertEqual(len(d.deleted_books), 0)
        self.assertEqual(len(d.changed_books), 0)


class TestDiff(unittest.TestCase):
    def test_no_changes(self):
        series = [{"id": "s1", "name": "Series A", "url": "file:///root/SeriesA",
                   "file_last_modified": "2026-01-01T00:00:00+00:00"}]
        books = [{"id": "b1", "name": "Ch01", "url": "file:///root/SeriesA/Ch01.pdf",
                  "file_last_modified": "2026-01-01T00:00:00+00:00",
                  "file_size": 1000, "file_hash": "", "series_id": "s1"}]
        fs = {
            "series": {
                "file:///root/SeriesA": {
                    "url": "file:///root/SeriesA",
                    "name": "Series A",
                    "file_last_modified": "2026-01-01T00:00:00+00:00",
                    "books": [
                        {"url": "file:///root/SeriesA/Ch01.pdf", "name": "Ch01",
                         "file_size": 1000,
                         "file_last_modified": "2026-01-01T00:00:00+00:00"},
                    ],
                }
            },
            "oneshots": [],
        }

        d = compute(series, books, fs)
        self.assertEqual(len(d.new_series), 0)
        self.assertEqual(len(d.deleted_series), 0)
        self.assertEqual(len(d.new_books), 0)
        self.assertEqual(len(d.deleted_books), 0)
        self.assertEqual(len(d.changed_books), 0)

    def test_new_series(self):
        books = []
        fs = {
            "series": {
                "file:///root/SeriesA": {
                    "url": "file:///root/SeriesA",
                    "name": "Series A",
                    "file_last_modified": "2026-01-01T00:00:00+00:00",
                    "books": [
                        {"url": "file:///root/SeriesA/Ch01.pdf", "name": "Ch01",
                         "file_size": 1000,
                         "file_last_modified": "2026-01-01T00:00:00+00:00"},
                    ],
                }
            },
            "oneshots": [],
        }

        d = compute([], books, fs)
        self.assertEqual(len(d.new_series), 1)
        self.assertEqual(d.new_series[0]["name"], "Series A")
        self.assertEqual(len(d.new_books), 0)  # books inside new series excluded

    def test_deleted_series(self):
        series = [{"id": "s1", "name": "Series A", "url": "file:///root/SeriesA",
                   "file_last_modified": "2026-01-01T00:00:00+00:00"}]
        books = [{"id": "b1", "name": "Ch01", "url": "file:///root/SeriesA/Ch01.pdf",
                  "file_last_modified": "2026-01-01T00:00:00+00:00",
                  "file_size": 1000, "file_hash": "", "series_id": "s1"}]
        fs = {"series": {}, "oneshots": []}

        d = compute(series, books, fs)
        self.assertEqual(len(d.deleted_series), 1)
        self.assertEqual(d.deleted_series[0]["id"], "s1")
        self.assertEqual(len(d.deleted_books), 1)
        self.assertEqual(d.deleted_books[0]["id"], "b1")

    def test_new_book_in_existing_series(self):
        series = [{"id": "s1", "name": "Series A", "url": "file:///root/SeriesA",
                   "file_last_modified": "2026-01-01T00:00:00+00:00"}]
        books = [{"id": "b1", "name": "Ch01", "url": "file:///root/SeriesA/Ch01.pdf",
                  "file_last_modified": "2026-01-01T00:00:00+00:00",
                  "file_size": 1000, "file_hash": "", "series_id": "s1"}]
        fs = {
            "series": {
                "file:///root/SeriesA": {
                    "url": "file:///root/SeriesA",
                    "name": "Series A",
                    "file_last_modified": "2026-01-01T00:00:00+00:00",
                    "books": [
                        {"url": "file:///root/SeriesA/Ch01.pdf", "name": "Ch01",
                         "file_size": 1000,
                         "file_last_modified": "2026-01-01T00:00:00+00:00"},
                        {"url": "file:///root/SeriesA/Ch02.pdf", "name": "Ch02",
                         "file_size": 2000,
                         "file_last_modified": "2026-01-02T00:00:00+00:00"},
                    ],
                }
            },
            "oneshots": [],
        }

        d = compute(series, books, fs)
        self.assertEqual(len(d.new_series), 0)
        self.assertEqual(len(d.new_books), 1)
        self.assertEqual(d.new_books[0]["name"], "Ch02")

    def test_changed_book_different_mtime(self):
        series = [{"id": "s1", "name": "Series A", "url": "file:///root/SeriesA",
                   "file_last_modified": "2026-01-01T00:00:00+00:00"}]
        books = [{"id": "b1", "name": "Ch01", "url": "file:///root/SeriesA/Ch01.pdf",
                  "file_last_modified": "2026-01-01T00:00:00+00:00",
                  "file_size": 1000, "file_hash": "", "series_id": "s1"}]
        fs = {
            "series": {
                "file:///root/SeriesA": {
                    "url": "file:///root/SeriesA",
                    "name": "Series A",
                    "file_last_modified": "2026-01-01T00:00:00+00:00",
                    "books": [
                        {"url": "file:///root/SeriesA/Ch01.pdf", "name": "Ch01",
                         "file_size": 1000,
                         "file_last_modified": "2026-02-01T00:00:00+00:00"},
                    ],
                }
            },
            "oneshots": [],
        }

        d = compute(series, books, fs)
        self.assertEqual(len(d.changed_books), 1)
        self.assertEqual(d.changed_books[0]["id"], "b1")

    def test_changed_book_different_size(self):
        series = [{"id": "s1", "name": "Series A", "url": "file:///root/SeriesA",
                   "file_last_modified": "2026-01-01T00:00:00+00:00"}]
        books = [{"id": "b1", "name": "Ch01", "url": "file:///root/SeriesA/Ch01.pdf",
                  "file_last_modified": "2026-01-01T00:00:00+00:00",
                  "file_size": 1000, "file_hash": "", "series_id": "s1"}]
        fs = {
            "series": {
                "file:///root/SeriesA": {
                    "url": "file:///root/SeriesA",
                    "name": "Series A",
                    "file_last_modified": "2026-01-01T00:00:00+00:00",
                    "books": [
                        {"url": "file:///root/SeriesA/Ch01.pdf", "name": "Ch01",
                         "file_size": 2000,
                         "file_last_modified": "2026-01-01T00:00:00+00:00"},
                    ],
                }
            },
            "oneshots": [],
        }

        d = compute(series, books, fs)
        self.assertEqual(len(d.changed_books), 1)
        self.assertEqual(d.changed_books[0]["id"], "b1")

    def test_group_new_books_by_series(self):
        new_books = [
            {"url": "file:///root/SeriesA/Ch03.pdf", "name": "Ch03",
             "file_size": 3000, "file_last_modified": "..."},
            {"url": "file:///root/SeriesB/New.pdf", "name": "New",
             "file_size": 4000, "file_last_modified": "..."},
        ]
        fs_series = {
            "file:///root/SeriesA": {"url": "file:///root/SeriesA"},
            "file:///root/SeriesB": {"url": "file:///root/SeriesB"},
        }
        db_series = {
            "file:///root/SeriesA": {"id": "id-A"},
            "file:///root/SeriesB": {"id": "id-B"},
        }

        result = group_new_books_by_series(new_books, fs_series, db_series)
        self.assertEqual(len(result), 2)
        self.assertIn("id-A", result)
        self.assertEqual(len(result["id-A"]), 1)
        self.assertEqual(result["id-A"][0]["name"], "Ch03")
        self.assertIn("id-B", result)
        self.assertEqual(len(result["id-B"]), 1)
        self.assertEqual(result["id-B"][0]["name"], "New")

