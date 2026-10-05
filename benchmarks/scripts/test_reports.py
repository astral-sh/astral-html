import json
import pathlib
import subprocess
import sys
import tempfile
import unittest

from report import rows_for_run, statistics, write_report
from run import create_run_directory, select_benchmarks


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value))


def estimates(mean=1000.0, lower=900.0, upper=1100.0, confidence=0.95):
    return {
        "mean": {"point_estimate": mean, "confidence_interval": {
            "confidence_level": confidence, "lower_bound": lower, "upper_bound": upper,
        }},
        "median": {"point_estimate": 950.0},
    }


class Reports(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.run = pathlib.Path(self.temporary.name)
        # Criterion sanitizes directory names; identify rows by full_id, not paths.
        self.data = self.run / "criterion/session-001/extract-links_page/parser/new"
        write_json(self.data / "benchmark.json", {
            "full_id": "extract-links/page/parser", "throughput": {"Bytes": 1024},
        })
        write_json(self.data / "estimates.json", estimates())
        write_json(self.data / "sample.json", {"iters": [10, 20], "times": [10000, 20000]})
        self.metadata = {
            "smoke": True, "status": "complete", "filter": None,
            "git_before": {"revision": "abc123", "status": ""}, "source_changed": False,
            "sessions": [{"id": "session-001"}, {"id": "session-002"}],
        }
        write_json(self.run / "metadata.json", self.metadata)
        write_json(self.run / "eligibility.json", [
            {"workload": "extract-links", "fixture": "page", "parser": "parser", "eligible": True},
            {"workload": "extract-links", "fixture": "page", "parser": "other", "eligible": False,
             "error": "<different>|field\nreason"},
        ])

    def test_sessions_keep_estimates_and_missing_or_ineligible_rows(self):
        rows = rows_for_run(self.run, self.metadata)
        self.assertEqual(len(rows), 4)
        self.assertEqual(rows[0][3:], ["session-001", "measured", "1", "[0.9, 1.1]", "0.95", "976.6", "2"])
        self.assertTrue(rows[1][4].startswith("ineligible:"))
        self.assertEqual(rows[2][3:5], ["session-002", "missing measurement"])

    def test_missing_and_invalid_statistics_are_visible(self):
        (self.data / "sample.json").unlink()
        self.assertTrue(rows_for_run(self.run, self.metadata)[0][4].startswith("invalid statistics:"))
        write_json(self.data / "sample.json", {"iters": [1], "times": [1, 2]})
        with self.assertRaisesRegex(ValueError, "matching lengths"):
            statistics(self.data)
        write_json(self.data / "sample.json", {"iters": [1, 2], "times": [1, 2]})
        for invalid in [
            estimates(mean=float("nan")), estimates(mean=-1), estimates(mean=True),
            estimates(lower=1200, upper=1000), estimates(confidence=0.99),
        ]:
            with self.subTest(invalid=invalid):
                write_json(self.data / "estimates.json", invalid)
                with self.assertRaises(ValueError):
                    statistics(self.data)

    def test_filter_is_literal_and_does_not_hide_mismatches(self):
        self.metadata["filter"] = "not-present"
        rows = rows_for_run(self.run, self.metadata)
        self.assertEqual(rows[0][4], "not selected by filter")
        self.assertTrue(rows[1][4].startswith("ineligible:"))

    def test_empty_or_only_ineligible_selection_is_rejected(self):
        eligibility = json.loads((self.run / "eligibility.json").read_text())
        self.assertEqual(select_benchmarks(eligibility, "page/parser"), ["extract-links/page/parser"])
        self.assertEqual(select_benchmarks(eligibility, None), ["extract-links/page/parser"])
        for substring in ["typo", "page/other"]:
            with self.subTest(substring=substring):
                with self.assertRaisesRegex(ValueError, "no eligible benchmark IDs"):
                    select_benchmarks(eligibility, substring)

    def test_report_escapes_errors_and_refuses_to_overwrite(self):
        write_report(self.run)
        markdown = (self.run / "report.md").read_text()
        self.assertIn("SMOKE ONLY", markdown)
        self.assertIn("&lt;different&gt;&#124;field<br>reason", markdown)
        self.assertIn("&lt;different&gt;", (self.run / "index.html").read_text())
        with self.assertRaises(FileExistsError):
            write_report(self.run)
        self.assertEqual((self.run / "report.md").read_text(), markdown)

    def test_environment_disclosure_is_visible_and_escaped(self):
        self.metadata.update({
            "rustc": "rustc 1.97.1 (ohm)\nbinary: rustc",
            "measurement_machine": {
                "model name": "AMD EPYC", "platform": "Linux KVM", "affinity": [4],
            },
            "notes": "Shared <KVM> | host\nPower policy is not controlled.",
        })
        write_json(self.run / "metadata.json", self.metadata)
        write_report(self.run)
        for filename in ["report.md", "index.html"]:
            content = (self.run / filename).read_text()
            self.assertIn("Compiler: rustc 1.97.1 (ohm)", content)
            self.assertIn("CPU: AMD EPYC", content)
            self.assertIn("OS: Linux KVM", content)
            self.assertIn("Measurement affinity: [4]", content)
            self.assertIn("Shared &lt;KVM&gt;", content)
            self.assertNotIn("Shared <KVM>", content)
            self.assertNotIn("binary: rustc", content)
        self.assertIn("&#124; host<br>Power policy", (self.run / "report.md").read_text())

    def test_existing_run_directory_is_preserved(self):
        with self.assertRaises(FileExistsError):
            create_run_directory(self.run)
        self.assertTrue((self.data / "sample.json").exists())

    def test_runner_help_works_with_python_safe_path(self):
        command = [sys.executable, "-P", str(pathlib.Path(__file__).with_name("run.py")), "--help"]
        result = subprocess.run(command, check=True, text=True, capture_output=True)
        self.assertIn("--no-ohm-defaults", result.stdout)
        self.assertIn("--notes", result.stdout)


if __name__ == "__main__":
    unittest.main()
