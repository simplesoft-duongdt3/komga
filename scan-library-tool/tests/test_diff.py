"""Tests for the diff engine."""

import unittest
from diff import Diff, compute, group_new_books_by_series, _normalize_file_url, _mtime_equals


class TestMtimeEquals(unittest.TestCase):
    """Test mtime comparison with sub-second/timezone tolerance."""

    def test_fs_with_subseconds_tz_matches_db_rounded_up(self):
        """FS: 10:25:23.774 vs DB: 10:25:24 (different second → not equal)."""
        self.assertFalse(_mtime_equals(
            "2026-05-26T10:25:23.774395+00:00",
            "2026-05-26T10:25:24",
        ))

    def test_same_second_different_subseconds(self):
        """Same second, different sub-second precision."""
        self.assertTrue(_mtime_equals(
            "2026-01-01T12:00:00.123456+00:00",
            "2026-01-01T12:00:00",
        ))

    def test_exact_match(self):
        self.assertTrue(_mtime_equals(
            "2026-01-01T12:00:00",
            "2026-01-01T12:00:00",
        ))

    def test_different_seconds(self):
        self.assertFalse(_mtime_equals(
            "2026-01-01T12:00:00",
            "2026-01-01T12:00:01",
        ))

    def test_missing_values(self):
        self.assertIsNone(_mtime_equals(None, "2026-01-01T12:00:00"))
        self.assertIsNone(_mtime_equals("2026-01-01T12:00:00", None))
        self.assertIsNone(_mtime_equals(None, None))

    def test_fs_with_utc_z(self):
        """FS format with Z suffix instead of +00:00."""
        self.assertTrue(_mtime_equals(
            "2026-01-01T12:00:00Z",
            "2026-01-01T12:00:00",
        ))


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


class TestDiffWithHash(unittest.TestCase):
    """Tests for hash-based change detection."""

    def test_changed_book_different_hash(self):
        """Hash differs → changed, even if mtime/size match."""
        series = [{"id": "s1", "name": "S", "url": "file:///root/S",
                   "file_last_modified": "2026-01-01T00:00:00+00:00"}]
        books = [{"id": "b1", "name": "Ch01", "url": "file:///root/S/Ch01.pdf",
                  "file_last_modified": "2026-01-01T00:00:00+00:00",
                  "file_size": 1000, "file_hash": "aaaa", "series_id": "s1"}]
        fs = {
            "series": {
                "file:///root/S": {
                    "url": "file:///root/S", "name": "S",
                    "file_last_modified": "2026-01-01T00:00:00+00:00",
                    "books": [
                        {"url": "file:///root/S/Ch01.pdf", "name": "Ch01",
                         "file_size": 1000,
                         "file_last_modified": "2026-01-01T00:00:00+00:00",
                         "file_hash": "bbbb"},  # different hash
                    ],
                }
            },
            "oneshots": [],
        }

        d = compute(series, books, fs)
        self.assertEqual(len(d.changed_books), 1)

    def test_no_change_when_hash_matches(self):
        """Hash matches → not changed, even if mtime differs."""
        series = [{"id": "s1", "name": "S", "url": "file:///root/S",
                   "file_last_modified": "2026-01-01T00:00:00+00:00"}]
        books = [{"id": "b1", "name": "Ch01", "url": "file:///root/S/Ch01.pdf",
                  "file_last_modified": "2026-01-01T00:00:00+00:00",
                  "file_size": 1000, "file_hash": "xxhash123", "series_id": "s1"}]
        fs = {
            "series": {
                "file:///root/S": {
                    "url": "file:///root/S", "name": "S",
                    "file_last_modified": "2026-01-01T00:00:00+00:00",
                    "books": [
                        {"url": "file:///root/S/Ch01.pdf", "name": "Ch01",
                         "file_size": 1000,
                         "file_last_modified": "2026-02-02T00:00:00+00:00",  # different mtime
                         "file_hash": "xxhash123"},  # same hash
                    ],
                }
            },
            "oneshots": [],
        }

        d = compute(series, books, fs)
        self.assertEqual(len(d.changed_books), 0)

    def test_no_change_when_hash_matches_and_size_differs(self):
        """Hash matches → not changed, even if size differs (hash is truth)."""
        series = [{"id": "s1", "name": "S", "url": "file:///root/S",
                   "file_last_modified": "2026-01-01T00:00:00+00:00"}]
        books = [{"id": "b1", "name": "Ch01", "url": "file:///root/S/Ch01.pdf",
                  "file_last_modified": "2026-01-01T00:00:00+00:00",
                  "file_size": 1000, "file_hash": "xxhash456", "series_id": "s1"}]
        fs = {
            "series": {
                "file:///root/S": {
                    "url": "file:///root/S", "name": "S",
                    "file_last_modified": "2026-01-01T00:00:00+00:00",
                    "books": [
                        {"url": "file:///root/S/Ch01.pdf", "name": "Ch01",
                         "file_size": 9999,  # different size
                         "file_last_modified": "2026-01-01T00:00:00+00:00",
                         "file_hash": "xxhash456"},  # same hash
                    ],
                }
            },
            "oneshots": [],
        }

        d = compute(series, books, fs)
        self.assertEqual(len(d.changed_books), 0)

    def test_hash_used_only_when_available_on_both_sides(self):
        """If FS hash is available but DB hash is empty → not changed (pending analysis)."""
        series = [{"id": "s1", "name": "S", "url": "file:///root/S",
                   "file_last_modified": "2026-01-01T00:00:00+00:00"}]
        books = [{"id": "b1", "name": "Ch01", "url": "file:///root/S/Ch01.pdf",
                  "file_last_modified": "2026-01-01T00:00:00+00:00",
                  "file_size": 1000, "file_hash": "", "series_id": "s1"}]  # empty hash in DB
        fs = {
            "series": {
                "file:///root/S": {
                    "url": "file:///root/S", "name": "S",
                    "file_last_modified": "2026-01-01T00:00:00+00:00",
                    "books": [
                        {"url": "file:///root/S/Ch01.pdf", "name": "Ch01",
                         "file_size": 9999,  # different size
                         "file_last_modified": "2026-01-01T00:00:00+00:00",
                         "file_hash": "somehash"},  # FS has hash
                    ],
                }
            },
            "oneshots": [],
        }

        d = compute(series, books, fs)
        # DB hash is empty, FS hash is set → not changed
        # The file simply hasn't been analyzed by Komga yet
        self.assertEqual(len(d.changed_books), 0)

    def test_fallback_to_mtime_when_both_hashes_empty(self):
        """If both hashes are empty → fall back to mtime/size."""
        series = [{"id": "s1", "name": "S", "url": "file:///root/S",
                   "file_last_modified": "2026-01-01T00:00:00+00:00"}]
        books = [{"id": "b1", "name": "Ch01", "url": "file:///root/S/Ch01.pdf",
                  "file_last_modified": "2026-01-01T00:00:00+00:00",
                  "file_size": 1000, "file_hash": "", "series_id": "s1"}]
        fs = {
            "series": {
                "file:///root/S": {
                    "url": "file:///root/S", "name": "S",
                    "file_last_modified": "2026-01-01T00:00:00+00:00",
                    "books": [
                        {"url": "file:///root/S/Ch01.pdf", "name": "Ch01",
                         "file_size": 9999,  # different size
                         "file_last_modified": "2026-01-01T00:00:00+00:00",
                         "file_hash": ""},  # no hash on either side
                    ],
                }
            },
            "oneshots": [],
        }

        d = compute(series, books, fs)
        self.assertEqual(len(d.changed_books), 1)



class TestPendingHash(unittest.TestCase):
    """Tests for the pending_hash feature (FS hash exists, DB hash empty)."""

    def test_fs_has_hash_db_empty(self):
        series = [{"id": "s1", "name": "S", "url": "file:///root/S",
                   "file_last_modified": "2026-01-01T00:00:00+00:00"}]
        books = [{"id": "b1", "name": "Ch01", "url": "file:///root/S/Ch01.pdf",
                  "file_last_modified": "2026-01-01T00:00:00+00:00",
                  "file_size": 1000, "file_hash": "", "series_id": "s1"}]
        fs = {
            "series": {"file:///root/S": {
                "url": "file:///root/S", "name": "S",
                "file_last_modified": "2026-01-01T00:00:00+00:00",
                "books": [{"url": "file:///root/S/Ch01.pdf", "name": "Ch01",
                           "file_size": 1000,
                           "file_last_modified": "2026-01-01T00:00:00+00:00",
                           "file_hash": "xxhash"}],
            }},
            "oneshots": [],
        }
        d = compute(series, books, fs)
        self.assertEqual(len(d.pending_hash), 1)
        self.assertEqual(d.pending_hash[0]["id"], "b1")
        self.assertEqual(len(d.changed_books), 0)

    def test_fs_and_db_both_empty_hash_no_change(self):
        series = [{"id": "s1", "name": "S", "url": "file:///root/S",
                   "file_last_modified": "2026-01-01T00:00:00+00:00"}]
        books = [{"id": "b1", "name": "Ch01", "url": "file:///root/S/Ch01.pdf",
                  "file_last_modified": "2026-01-01T00:00:00+00:00",
                  "file_size": 1000, "file_hash": "", "series_id": "s1"}]
        fs = {
            "series": {"file:///root/S": {
                "url": "file:///root/S", "name": "S",
                "file_last_modified": "2026-01-01T00:00:00+00:00",
                "books": [{"url": "file:///root/S/Ch01.pdf", "name": "Ch01",
                           "file_size": 1000,
                           "file_last_modified": "2026-01-01T00:00:00+00:00",
                           "file_hash": ""}],
            }},
            "oneshots": [],
        }
        d = compute(series, books, fs)
        self.assertEqual(len(d.pending_hash), 0)
        self.assertEqual(len(d.changed_books), 0)

    def test_different_hash_is_changed_not_pending(self):
        series = [{"id": "s1", "name": "S", "url": "file:///root/S",
                   "file_last_modified": "2026-01-01T00:00:00+00:00"}]
        books = [{"id": "b1", "name": "Ch01", "url": "file:///root/S/Ch01.pdf",
                  "file_last_modified": "2026-01-01T00:00:00+00:00",
                  "file_size": 1000, "file_hash": "old", "series_id": "s1"}]
        fs = {
            "series": {"file:///root/S": {
                "url": "file:///root/S", "name": "S",
                "file_last_modified": "2026-01-01T00:00:00+00:00",
                "books": [{"url": "file:///root/S/Ch01.pdf", "name": "Ch01",
                           "file_size": 1000,
                           "file_last_modified": "2026-01-01T00:00:00+00:00",
                           "file_hash": "new"}],
            }},
            "oneshots": [],
        }
        d = compute(series, books, fs)
        self.assertEqual(len(d.pending_hash), 0)
        self.assertEqual(len(d.changed_books), 1)

    def test_same_hash_no_change(self):
        series = [{"id": "s1", "name": "S", "url": "file:///root/S",
                   "file_last_modified": "2026-01-01T00:00:00+00:00"}]
        books = [{"id": "b1", "name": "Ch01", "url": "file:///root/S/Ch01.pdf",
                  "file_last_modified": "2026-01-01T00:00:00+00:00",
                  "file_size": 1000, "file_hash": "same", "series_id": "s1"}]
        fs = {
            "series": {"file:///root/S": {
                "url": "file:///root/S", "name": "S",
                "file_last_modified": "2026-01-01T00:00:00+00:00",
                "books": [{"url": "file:///root/S/Ch01.pdf", "name": "Ch01",
                           "file_size": 1000,
                           "file_last_modified": "2026-01-01T00:00:00+00:00",
                           "file_hash": "same"}],
            }},
            "oneshots": [],
        }
        d = compute(series, books, fs)
        self.assertEqual(len(d.pending_hash), 0)
        self.assertEqual(len(d.changed_books), 0)

    def test_mixed_pending_and_changed(self):
        series = [{"id": "s1", "name": "S", "url": "file:///root/S",
                   "file_last_modified": "2026-01-01T00:00:00+00:00"}]
        books = [
            {"id": "b1", "name": "Ch01", "url": "file:///root/S/Ch01.pdf",
             "file_last_modified": "2026-01-01T00:00:00+00:00",
             "file_size": 1000, "file_hash": "", "series_id": "s1"},
            {"id": "b2", "name": "Ch02", "url": "file:///root/S/Ch02.pdf",
             "file_last_modified": "2026-01-01T00:00:00+00:00",
             "file_size": 2000, "file_hash": "old", "series_id": "s1"},
        ]
        fs = {
            "series": {"file:///root/S": {
                "url": "file:///root/S", "name": "S",
                "file_last_modified": "2026-01-01T00:00:00+00:00",
                "books": [
                    {"url": "file:///root/S/Ch01.pdf", "name": "Ch01",
                     "file_size": 1000,
                     "file_last_modified": "2026-01-01T00:00:00+00:00",
                     "file_hash": "new"},
                    {"url": "file:///root/S/Ch02.pdf", "name": "Ch02",
                     "file_size": 2000,
                     "file_last_modified": "2026-01-01T00:00:00+00:00",
                     "file_hash": "new"},
                ],
            }},
            "oneshots": [],
        }
        d = compute(series, books, fs)
        self.assertEqual(len(d.pending_hash), 1)
        self.assertEqual(d.pending_hash[0]["id"], "b1")
        self.assertEqual(len(d.changed_books), 1)
        self.assertEqual(d.changed_books[0]["id"], "b2")
