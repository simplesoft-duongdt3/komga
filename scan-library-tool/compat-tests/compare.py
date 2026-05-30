#!/usr/bin/env python3
"""Compare Python and Rust compat test outputs and report differences."""

import json
import subprocess
import sys
import os
from pathlib import Path

ROOT = Path(__file__).parent.parent
FIXTURES = ROOT / "compat-tests" / "fixtures" / "test_cases.json"
PYTHON_RUNNER = ROOT / "compat-tests" / "run_python.py"
RUST_BINARY = ROOT / "rust" / "target" / "release" / "komga-smart-scanner"

def run_python():
    print("Running Python tests...")
    result = subprocess.run(
        [sys.executable, str(PYTHON_RUNNER)],
        capture_output=True, text=True, cwd=str(ROOT),
    )
    if result.returncode != 0:
        print(f"Python runner failed:\n{result.stderr}")
        sys.exit(1)
    return json.loads(result.stdout)


def run_rust():
    print("Running Rust tests...")
    # Build release if needed
    if not RUST_BINARY.exists():
        print("Building Rust binary (release)...")
        subprocess.run(
            ["cargo", "build", "--release"],
            cwd=str(ROOT / "rust"),
            check=True,
        )
    result = subprocess.run(
        [str(RUST_BINARY), "compat-test", "--fixtures", str(FIXTURES)],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print(f"Rust runner failed:\n{result.stderr}")
        sys.exit(1)
    return json.loads(result.stdout)


def compare_section(py_data, rust_data, section):
    py_results = py_data.get(section, [])
    rust_results = rust_data.get(section, [])

    diffs = []
    max_len = max(len(py_results), len(rust_results))

    for i in range(max_len):
        py_r = py_results[i] if i < len(py_results) else None
        rust_r = rust_results[i] if i < len(rust_results) else None

        if py_r is None:
            diffs.append({"index": i, "issue": "missing in Python"})
            continue
        if rust_r is None:
            diffs.append({"index": i, "issue": "missing in Rust"})
            continue

        # Compare pass/fail status
        if py_r.get("pass") != rust_r.get("pass"):
            diffs.append({
                "index": i,
                "name": py_r.get("name", rust_r.get("name", "")),
                "issue": "pass/fail mismatch",
                "python_pass": py_r.get("pass"),
                "rust_pass": rust_r.get("pass"),
            })

        # Compare actual values
        py_actual = py_r.get("actual")
        rust_actual = rust_r.get("actual")
        if py_actual != rust_actual:
            diffs.append({
                "index": i,
                "name": py_r.get("name", rust_r.get("name", "")),
                "issue": "actual value mismatch",
                "python_actual": py_actual,
                "rust_actual": rust_actual,
            })

    return diffs


def main():
    py_data = run_python()
    rust_data = run_rust()

    py_summary = py_data.get("summary", {})
    rust_summary = rust_data.get("summary", {})

    print(f"\n{'='*60}")
    print(f"Python: {py_summary.get('passed', 0)}/{py_summary.get('total', 0)} passed")
    print(f"Rust:   {rust_summary.get('passed', 0)}/{rust_summary.get('total', 0)} passed")
    print(f"{'='*60}\n")

    all_diffs = {}
    for section in ["normalize_url", "mtime_equals", "diff"]:
        diffs = compare_section(py_data, rust_data, section)
        if diffs:
            all_diffs[section] = diffs

    if not all_diffs:
        print("ALL TESTS MATCH — Python and Rust produce identical results.")
        return 0
    else:
        print("DIFFERENCES FOUND:\n")
        for section, diffs in all_diffs.items():
            print(f"  [{section}]")
            for d in diffs:
                name = d.get("name", "")
                idx = d["index"]
                label = f"  #{idx}" + (f" ({name})" if name else "")
                print(f"{label}: {d['issue']}")
                if "python_actual" in d:
                    print(f"    Python: {d['python_actual']}")
                    print(f"    Rust:   {d['rust_actual']}")
                if "python_pass" in d:
                    print(f"    Python pass: {d['python_pass']}")
                    print(f"    Rust pass:   {d['rust_pass']}")
            print()

        return 1


if __name__ == "__main__":
    sys.exit(main())
