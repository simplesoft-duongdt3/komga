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


SECTIONS = ["normalize_url", "mtime_equals", "diff", "xxh3_128", "hash_cache"]


def compare_section(py_data, rust_data, section):
    py_results = py_data.get(section, [])
    rust_results = rust_data.get(section, [])

    diffs = []
    max_len = max(len(py_results), len(rust_results))

    for i in range(max_len):
        py_r = py_results[i] if i < len(py_results) else None
        rust_r = rust_results[i] if i < len(rust_results) else None

        if py_r is None:
            diffs.append({"index": i, "section": section, "issue": "missing in Python"})
            continue
        if rust_r is None:
            diffs.append({"index": i, "section": section, "issue": "missing in Rust"})
            continue

        # Compare pass/fail status
        if py_r.get("pass") != rust_r.get("pass"):
            diffs.append({
                "index": i, "section": section,
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
                "index": i, "section": section,
                "name": py_r.get("name", rust_r.get("name", "")),
                "issue": "actual value mismatch",
                "python_actual": py_actual,
                "rust_actual": rust_actual,
            })

    return diffs


def main():
    py_data = run_python()
    rust_data = run_rust()

    all_diffs = {}
    for section in SECTIONS:
        diffs = compare_section(py_data, rust_data, section)
        if diffs:
            all_diffs[section] = diffs

    if not all_diffs:
        # Recompute totals including all sections for accurate reporting
        py_total = sum(len(py_data.get(s, [])) for s in SECTIONS)
        py_pass = sum(1 for s in SECTIONS for r in py_data.get(s, []) if r.get("pass"))
        rust_total = sum(len(rust_data.get(s, [])) for s in SECTIONS)
        rust_pass = sum(1 for s in SECTIONS for r in rust_data.get(s, []) if r.get("pass"))
        print(f"Python: {py_pass}/{py_total} passed")
        print(f"Rust:   {rust_pass}/{rust_total} passed")
        print("\nALL TESTS MATCH — Python and Rust produce identical results.")
        return 0
    else:
        print("DIFFERENCES FOUND:\n")
        for section, diffs in all_diffs.items():
            print(f"  [{section}]")
            for d in diffs:
                name = d.get("name", "")
                idx = d["index"]
                sec = d.get("section", section)
                label = f"  [{sec} #{idx}" + (f" ({name})" if name else "") + "]"
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
