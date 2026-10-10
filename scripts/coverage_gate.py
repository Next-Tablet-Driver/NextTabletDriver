#!/usr/bin/env python3
"""Merges the Rust and frontend coverage, writes a report and enforces the minimum thresholds.

Inputs
  --lcov        lcov.info from `cargo llvm-cov nextest --lcov`
  --frontend    frontend/coverage/coverage-summary.json from `vitest --coverage` (json-summary)
  --thresholds  coverage-thresholds.json (floors, in percent of lines)

Outputs
  --markdown    the report, meant for the job summary and the pull request comment
  exit status   1 when a floor is not met

Rust figures count production code only. The inline `#[cfg(test)] mod tests` of a file is
dropped (every line from the first `#[cfg(test)]` on), because covering the tests themselves
would flatter the number. Files outside the root crate's `src/` (the QA crate, the SDK, the
plugins) are not measured here.

The floors are a ratchet: when the measured value is at least RATCHET_MARGIN points above its
floor, the report says so, and the floor is raised in coverage-thresholds.json in a normal pull
request. Lowering a floor is a reviewed change to that file, never a side effect.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

RATCHET_MARGIN = 3.0
MARKER = "<!-- coverage-report -->"
LOWEST_FILES = 10


def percent(covered: int, total: int) -> float:
    return 100.0 * covered / total if total else 100.0


def rust_production_coverage(lcov_path: Path, root: Path) -> tuple[int, int, list[tuple[float, int, str]]]:
    """Returns (covered, total, files) for the production lines of `<root>/src`."""
    src_root = (root / "src").resolve()
    covered = total = 0
    files: list[tuple[float, int, str]] = []
    for record in lcov_path.read_text(encoding="utf-8").split("end_of_record"):
        match = re.search(r"^SF:(.*)$", record, re.M)
        if match is None:
            continue
        source = Path(match.group(1).strip())
        if not source.is_absolute():
            source = root / source
        try:
            relative = source.resolve().relative_to(src_root)
        except ValueError:
            continue
        try:
            lines = source.read_text(encoding="utf-8", errors="replace").split("\n")
        except OSError:
            continue
        test_start = next((number for number, line in enumerate(lines, 1) if line.strip() == "#[cfg(test)]"), None)
        file_covered = file_total = 0
        for number, hits in re.findall(r"^DA:(\d+),(\d+)", record, re.M):
            if test_start is not None and int(number) >= test_start:
                continue
            file_total += 1
            file_covered += int(hits) > 0
        covered += file_covered
        total += file_total
        if file_total:
            files.append((percent(file_covered, file_total), file_total - file_covered, relative.as_posix()))
    return covered, total, files


def frontend_coverage(summary_path: Path) -> tuple[int, int]:
    lines = json.loads(summary_path.read_text(encoding="utf-8"))["total"]["lines"]
    return int(lines["covered"]), int(lines["total"])


def status(value: float, floor: float) -> str:
    return "pass" if value >= floor else "**below the floor**"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--lcov", type=Path, required=True)
    parser.add_argument("--frontend", type=Path, required=True)
    parser.add_argument("--thresholds", type=Path, default=Path("coverage-thresholds.json"))
    parser.add_argument("--markdown", type=Path)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    args = parser.parse_args()

    floors = json.loads(args.thresholds.read_text(encoding="utf-8"))
    rust_cov, rust_total, rust_files = rust_production_coverage(args.lcov, args.root)
    front_cov, front_total = frontend_coverage(args.frontend)
    if rust_total == 0 or front_total == 0:
        print("error: no coverage data found (is the lcov report empty, or the root wrong?)", file=sys.stderr)
        return 2

    measured = {
        "rust": percent(rust_cov, rust_total),
        "frontend": percent(front_cov, front_total),
        "total": percent(rust_cov + front_cov, rust_total + front_total),
    }
    labels = {
        "rust": f"Rust, production code ({rust_cov}/{rust_total} lines)",
        "frontend": f"Frontend ({front_cov}/{front_total} lines)",
        "total": f"All code ({rust_cov + front_cov}/{rust_total + front_total} lines)",
    }

    failed = [name for name in ("rust", "frontend", "total") if measured[name] < floors[name]]
    raisable = [name for name in ("rust", "frontend", "total") if measured[name] >= floors[name] + RATCHET_MARGIN]

    report = [MARKER, "## Coverage", "", "| | Lines | Floor | |", "| --- | ---: | ---: | --- |"]
    for name in ("rust", "frontend", "total"):
        report.append(f"| {labels[name]} | {measured[name]:.1f} % | {floors[name]:g} % | {status(measured[name], floors[name])} |")
    report.append("")
    if failed:
        report.append("A coverage floor is not met: add tests for the new code. Lowering a floor in `coverage-thresholds.json` needs a reviewed justification.")
        report.append("")
    if raisable and not failed:
        suggestion = ", ".join(f"`{name}` to {int(measured[name])}" for name in raisable)
        report.append(f"Coverage is well above its floor: raise {suggestion} in `coverage-thresholds.json`.")
        report.append("")
    report += [
        f"<details><summary>The {LOWEST_FILES} least covered Rust files</summary>",
        "",
        "| File | Lines | Missed |",
        "| --- | ---: | ---: |",
    ]
    for pct, missed, name in sorted(rust_files)[:LOWEST_FILES]:
        report.append(f"| `{name}` | {pct:.1f} % | {missed} |")
    report += ["", "</details>", ""]

    text = "\n".join(report)
    print(text)
    if args.markdown is not None:
        args.markdown.write_text(text, encoding="utf-8")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
