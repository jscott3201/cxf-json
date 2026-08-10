import contextlib
import importlib.util
import io
import tempfile
import unittest
from pathlib import Path


module_path = Path(__file__).with_name("summarize-benchmarks.py")
spec = importlib.util.spec_from_file_location("summarize_benchmarks", module_path)
benchmarks = importlib.util.module_from_spec(spec)
spec.loader.exec_module(benchmarks)


def corpus_report(commit="abc123"):
    return {
        "git_commit": commit,
        "git_origin_matches_expected": True,
        "files": [
            {
                "path": "/corpus/fixture.jsonld",
                "expected_failure": False,
                "input_bytes": 100,
                "failure": None,
            }
        ],
        "file_count": 1,
        "passed": 1,
        "expected_failures": 0,
        "unexpected_failures": 0,
        "unexpected_passes": 0,
        "read_failures": 0,
        "input_bytes": 100,
        "max_input_bytes": 100,
        "quad_count": 1,
        "max_nesting_depth": 1,
        "max_object_members": 1,
        "total_json_values": 2,
        "decoded_member_name_bytes": 1,
        "rdf_term_bytes": 10,
        "measured_structure_files": 1,
        "structural_metrics_complete": True,
        "preflight_micros": 2,
        "json_ld_micros": 3,
        "elapsed_micros": 8,
    }


def wasm_report(digest="0" * 64):
    return {
        "node": "v26.7.0",
        "platform": "darwin",
        "architecture": "arm64",
        "module_bytes": 100,
        "module_sha256": digest,
        "compile_micros": 2,
        "instantiate_micros": 1,
        "execute_micros": 3,
        "initial_memory_bytes": 65_536,
        "final_memory_bytes": 131_072,
    }


class SummaryTests(unittest.TestCase):
    def test_summarizes_five_pinned_corpus_runs(self):
        reports = [corpus_report() for _ in range(5)]
        with tempfile.TemporaryDirectory() as directory:
            times = []
            for run in range(5):
                path = Path(directory) / f"owned-{run}.time"
                path.write_text(" 4096  maximum resident set size\n")
                times.append(path)
            summary = benchmarks.summarize_corpus(reports, times)

        self.assertEqual(summary["git_commit"], "abc123")
        self.assertEqual(summary["combined_stage_micros"]["median"], 5)
        self.assertEqual(summary["maximum_rss_bytes"]["median"], 4096)

    def test_rejects_mixed_corpus_commits(self):
        reports = [corpus_report() for _ in range(4)] + [corpus_report("def456")]
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            benchmarks.summarize_corpus(reports, ["unused"] * 5)

    def test_rejects_mixed_wasm_modules(self):
        reports = [wasm_report() for _ in range(4)] + [wasm_report("1" * 64)]
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            benchmarks.summarize_wasm(reports)

    def test_rejects_mismatched_time_report_names(self):
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            benchmarks.validate_time_pairs(
                ["oce-1.json", "oce-2.json"], ["buildings-1.time", "oce-2.time"]
            )


if __name__ == "__main__":
    unittest.main()
