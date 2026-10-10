"""Tests of coverage_gate.py, run with `python -m unittest scripts/test_coverage_gate.py`."""
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("coverage_gate.py")

SOURCE = "\n".join(
    [
        "fn covered() {}",  # 1
        "fn missed() {}",  # 2
        "",  # 3
        "#[cfg(test)]",  # 4
        "mod tests {",  # 5
        "    fn helper() {}",  # 6
        "}",  # 7
    ]
)


def run_gate(rust_hits: dict[int, int], frontend: tuple[int, int], floors: dict[str, float]) -> tuple[int, str]:
    with tempfile.TemporaryDirectory() as folder:
        root = Path(folder)
        (root / "src").mkdir()
        (root / "src" / "lib.rs").write_text(SOURCE, encoding="utf-8")
        records = "".join(f"DA:{line},{hits}\n" for line, hits in rust_hits.items())
        (root / "lcov.info").write_text(f"SF:{root / 'src' / 'lib.rs'}\n{records}end_of_record\n", encoding="utf-8")
        # A file outside src/ must be ignored.
        with (root / "lcov.info").open("a", encoding="utf-8") as lcov:
            lcov.write(f"SF:{root / 'qa' / 'tests' / 'x.rs'}\nDA:1,0\nend_of_record\n")
        covered, total = frontend
        (root / "summary.json").write_text(json.dumps({"total": {"lines": {"covered": covered, "total": total}}}), encoding="utf-8")
        (root / "floors.json").write_text(json.dumps(floors), encoding="utf-8")
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "--lcov", str(root / "lcov.info"), "--frontend", str(root / "summary.json"),
             "--thresholds", str(root / "floors.json"), "--root", str(root)],
            capture_output=True, text=True, check=False,
        )
        return result.returncode, result.stdout + result.stderr


class CoverageGate(unittest.TestCase):
    def test_passes_when_every_floor_is_met(self) -> None:
        code, output = run_gate({1: 1, 2: 0}, (50, 100), {"rust": 50, "frontend": 50, "total": 50})
        self.assertEqual(code, 0, output)
        self.assertIn("50.0 %", output)

    def test_fails_when_rust_is_below_its_floor(self) -> None:
        code, output = run_gate({1: 1, 2: 0}, (100, 100), {"rust": 60, "frontend": 10, "total": 10})
        self.assertEqual(code, 1, output)
        self.assertIn("below the floor", output)

    def test_fails_when_the_frontend_is_below_its_floor(self) -> None:
        code, _ = run_gate({1: 1, 2: 1}, (10, 100), {"rust": 10, "frontend": 50, "total": 10})
        self.assertEqual(code, 1)

    def test_the_inline_tests_do_not_count(self) -> None:
        # Line 6 is test code: covered or not, it must not change the production figure (1/2).
        code, output = run_gate({1: 1, 2: 0, 6: 1}, (0, 10), {"rust": 50, "frontend": 0, "total": 0})
        self.assertEqual(code, 0, output)
        self.assertIn("(1/2 lines)", output)

    def test_suggests_raising_a_floor_that_is_far_below(self) -> None:
        code, output = run_gate({1: 1, 2: 1}, (90, 100), {"rust": 50, "frontend": 50, "total": 50})
        self.assertEqual(code, 0, output)
        self.assertIn("raise", output)

    def test_an_empty_report_is_an_error_not_a_pass(self) -> None:
        code, _ = run_gate({}, (0, 0), {"rust": 0, "frontend": 0, "total": 0})
        self.assertEqual(code, 2)


if __name__ == "__main__":
    unittest.main()
