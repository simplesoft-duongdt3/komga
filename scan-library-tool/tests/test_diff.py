"""Tests for the diff engine."""

import unittest
from diff import Diff, compute, group_new_books_by_series


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

