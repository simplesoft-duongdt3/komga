#!/usr/bin/env python3
"""Compatibility test runner — runs test cases against the Python implementation
   and outputs JSON results for comparison with the Rust implementation."""

import json
import sys
import os
from pathlib import Path

# Add scan-library-tool to path
sys.path.insert(0, str(Path(__file__).parent.parent / "python"))

from diff import compute as diff_compute, _normalize_file_url, _mtime_equals


def run_normalize_url_tests(cases):
    results = []
    for i, case in enumerate(cases):
        actual = _normalize_file_url(case["input"])
        ok = actual == case["expected"]
        results.append({
            "index": i,
            "input": case["input"],
            "expected": case["expected"],
            "actual": actual,
            "pass": ok,
        })
    return results


def run_mtime_equals_tests(cases):
    results = []
    for i, case in enumerate(cases):
        actual = _mtime_equals(case["fs"], case["db"])
        expected = case["expected"]
        # JSON null → Python None
        if expected is None:
            ok = actual is None
        else:
            ok = actual == expected
        results.append({
            "index": i,
            "fs": case["fs"],
            "db": case["db"],
            "expected": expected,
            "actual": actual,
            "pass": ok,
        })
    return results


def run_diff_tests(cases):
    results = []
    for i, case in enumerate(cases):
        db_series = case["db_series"]
        db_books = case["db_books"]
        fs_data = {
            "series": case["fs_series"],
            "oneshots": case.get("fs_oneshots", []),
        }

        d = diff_compute(db_series, db_books, fs_data)

        actual = {
            "new_series": len(d.new_series),
            "deleted_series": len(d.deleted_series),
            "new_books": len(d.new_books),
            "deleted_books": len(d.deleted_books),
            "changed_books": len(d.changed_books),
            "pending_hash": len(d.pending_hash),
        }

        expected = case["expected"]
        ok = actual == expected

        results.append({
            "index": i,
            "name": case["name"],
            "expected": expected,
            "actual": actual,
            "pass": ok,
        })
    return results


def main():
    fixture_path = Path(__file__).parent / "fixtures" / "test_cases.json"
    with open(fixture_path) as f:
        fixtures = json.load(f)

    output = {
        "implementation": "python",
        "normalize_url": run_normalize_url_tests(fixtures["normalize_url"]),
        "mtime_equals": run_mtime_equals_tests(fixtures["mtime_equals"]),
        "diff": run_diff_tests(fixtures["diff"]),
    }

    # Summary
    all_results = output["normalize_url"] + output["mtime_equals"] + output["diff"]
    total = len(all_results)
    passed = sum(1 for r in all_results if r["pass"])
    output["summary"] = {"total": total, "passed": passed, "failed": total - passed}

    print(json.dumps(output, indent=2))


if __name__ == "__main__":
    main()
