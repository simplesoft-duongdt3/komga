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

try:
    import xxhash
    HAS_XXHASH = True
except ImportError:
    HAS_XXHASH = False


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


def run_xxh3_128_tests(cases):
    results = []
    if not HAS_XXHASH:
        for i, case in enumerate(cases):
            results.append({
                "index": i,
                "input": case.get("input", case.get("desc", "")),
                "expected": case["expected"],
                "actual": "SKIPPED (xxhash not installed)",
                "pass": False,
                "skip": True,
            })
        return results

    for i, case in enumerate(cases):
        if "input" in case:
            data = case["input"].encode("utf-8") if case["input"] else b""
        elif "input-hex" in case:
            data = bytes.fromhex(case["input-hex"])
        else:
            data = b""

        actual = xxhash.xxh3_128(data, seed=0).hexdigest()
        ok = actual == case["expected"]
        results.append({
            "index": i,
            "input": case.get("desc", case.get("input", case.get("input-hex", ""))),
            "expected": case["expected"],
            "actual": actual,
            "pass": ok,
        })
    return results


def run_hash_cache_tests(cases):
    results = []
    for i, case in enumerate(cases):
        cache = case["cache"]
        checks = case["checks"]

        actual = {
            "total_files": cache["total_files"],
            "total_bytes": cache["total_bytes"],
            "root": cache["root"],
            "entry_count": len(cache["entries"]),
        }

        # Check first entry (sorted by URI for deterministic order)
        entries = sorted(cache["entries"].items(), key=lambda x: x[0])
        if entries:
            first_uri, first_entry = entries[0]
            actual["first_entry_hash"] = first_entry["hash"]
            actual["first_entry_size"] = first_entry["size"]
            last_uri, last_entry = entries[-1]
            actual["last_entry_hash"] = last_entry["hash"]

        ok = all(actual.get(k) == v for k, v in checks.items())
        results.append({
            "index": i,
            "name": case["name"],
            "checks": checks,
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
        "xxh3_128": run_xxh3_128_tests(fixtures.get("xxh3_128", [])),
        "hash_cache": run_hash_cache_tests(fixtures.get("hash_cache", [])),
    }

    # Summary
    all_results = []
    for section in ["normalize_url", "mtime_equals", "diff", "xxh3_128", "hash_cache"]:
        all_results.extend(output.get(section, []))

    total = len(all_results)
    passed = sum(1 for r in all_results if r["pass"])
    output["summary"] = {"total": total, "passed": passed, "failed": total - passed}

    print(json.dumps(output, indent=2))


if __name__ == "__main__":
    main()
