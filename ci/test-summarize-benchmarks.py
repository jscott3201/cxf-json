import contextlib
import importlib.util
import io
import tempfile
import unittest
from pathlib import Path


module_path = Path(__file__).with_name("summarize-benchmarks.py")
spec = importlib.util.spec_from_file_location("summarize_benchmarks", module_path)
if spec is None or spec.loader is None:
    raise RuntimeError("failed to load benchmark summarizer")
benchmarks = importlib.util.module_from_spec(spec)
spec.loader.exec_module(benchmarks)


def corpus_report(run_id, commit="b" * 40):
    return {
        "run_id": run_id,
        "instrumentation_revision": "a" * 40,
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


def wasm_report(run_id, digest="0" * 64):
    return {
        "run_id": run_id,
        "instrumentation_revision": "a" * 40,
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


def stress_report(run_id, digest="0" * 64):
    return {
        "run_id": run_id,
        "instrumentation_revision": "a" * 40,
        "generator_version": 1,
        "cases": [
            {
                "name": "width-4",
                "family": "width",
                "parameters": {"members": 4},
                "input_bytes": 20,
                "input_sha256": digest,
                "expected": {"kind": "success", "quad_count": 0},
                "actual": {"kind": "success", "quad_count": 0},
                "metrics": {
                    "json": {
                        "max_nesting_depth": 1,
                        "max_object_members": 4,
                        "total_values": 5,
                        "decoded_member_name_bytes": 4,
                    },
                    "rdf_term_bytes": 0,
                },
                "preflight_micros": 2,
                "json_ld_micros": 3,
            }
        ],
        "case_count": 1,
        "input_bytes": 20,
        "unexpected_outcomes": 0,
        "elapsed_micros": 8,
    }


def semantic_report(run_id, digest="0" * 64):
    return {
        "run_id": f"{run_id}-1",
        "instrumentation_revision": "a" * 40,
        "workload_version": 1,
        "retained_values": 32_768,
        "input_bytes": 131_119,
        "input_sha256": digest,
        "outcome": "success",
        "source_matches_input": True,
        "max_nesting_depth": 2,
        "max_object_members": 2,
        "total_values": 32_771,
        "decoded_member_name_bytes": 34,
        "emitted_rdf_quads": 32_768,
        "retained_rdf_term_bytes": 2_392_064,
        "returned_rdf_quads": 32_768,
        "preflight_ordered_micros": 2,
        "jsonld_quad_retention_micros": 5,
        "elapsed_micros": 10,
    }


def semantic_time_report(report, rss=16_384):
    return (
        f"CXF_JSON_TIME_V1 run_id={report['run_id']} "
        f"instrumentation_revision={report['instrumentation_revision']} "
        f"workload_version={report['workload_version']} "
        f"input_sha256={report['input_sha256']}\n"
        f" {rss}  maximum resident set size\n"
    )


class SummaryTests(unittest.TestCase):
    def test_summarizes_five_pinned_corpus_runs(self):
        reports = [corpus_report(str(run)) for run in range(5)]
        with tempfile.TemporaryDirectory() as directory:
            times = []
            for run in range(5):
                path = Path(directory) / f"owned-{run}.time"
                path.write_text(" 4096  maximum resident set size\n")
                times.append(path)
            summary = benchmarks.summarize_corpus(reports, times)

        self.assertEqual(summary["git_commit"], "b" * 40)
        self.assertEqual(summary["combined_stage_micros"]["median"], 5)
        self.assertEqual(summary["maximum_rss_bytes"]["median"], 4096)

    def test_rejects_mixed_corpus_commits(self):
        reports = [corpus_report(str(run)) for run in range(4)]
        reports.append(corpus_report("4", "c" * 40))
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            benchmarks.summarize_corpus(reports, ["unused"] * 5)

    def test_rejects_mixed_wasm_modules(self):
        reports = [wasm_report(str(run)) for run in range(4)]
        reports.append(wasm_report("4", "1" * 64))
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            benchmarks.summarize_wasm(reports)

    def test_summarizes_five_resource_stress_runs(self):
        reports = [stress_report(str(run)) for run in range(5)]
        reports[4]["cases"][0]["metrics"]["rdf_term_bytes"] = 4
        with tempfile.TemporaryDirectory() as directory:
            times = []
            for run in range(5):
                path = Path(directory) / f"stress-{run}.time"
                path.write_text(" 8192  maximum resident set size\n")
                times.append(path)
            summary = benchmarks.summarize_resource_stress(reports, times)

        self.assertEqual(summary["generator_version"], 1)
        self.assertEqual(summary["cases"][0]["preflight_micros"]["median"], 2)
        self.assertEqual(summary["cases"][0]["rdf_term_bytes"]["maximum"], 4)
        self.assertEqual(summary["maximum_rss_bytes"]["median"], 8192)

    def test_rejects_changed_resource_stress_case(self):
        reports = [stress_report(str(run)) for run in range(5)]
        reports[4]["cases"][0]["input_sha256"] = "1" * 64
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            benchmarks.summarize_resource_stress(reports, ["unused"] * 5)

    def test_summarizes_five_semantic_ingestion_runs(self):
        reports = [semantic_report(str(run)) for run in range(5)]
        with tempfile.TemporaryDirectory() as directory:
            times = []
            for run in range(5):
                path = Path(directory) / f"semantic-{run}.time"
                path.write_text(semantic_time_report(reports[run]))
                times.append(path)
            summary = benchmarks.summarize_semantic_ingestion(reports, times)

        self.assertEqual(summary["workload"]["retained_values"], 32_768)
        self.assertEqual(summary["rdf"]["returned_rdf_quads"], 32_768)
        self.assertEqual(summary["preflight_ordered_micros"]["median"], 2)
        self.assertEqual(summary["jsonld_quad_retention_micros"]["median"], 5)
        self.assertEqual(summary["combined_stage_micros"]["median"], 7)
        self.assertEqual(summary["elapsed_micros"]["median"], 10)
        self.assertEqual(summary["maximum_rss_bytes"]["median"], 16_384)

    def test_rejects_mismatched_semantic_time_identity(self):
        reports = [semantic_report(str(run)) for run in range(5)]
        with tempfile.TemporaryDirectory() as directory:
            times = []
            for run in range(5):
                path = Path(directory) / f"semantic-{run}.time"
                marker_report = reports[1] if run == 0 else reports[run]
                path.write_text(semantic_time_report(marker_report))
                times.append(path)
            with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                benchmarks.summarize_semantic_ingestion(reports, times)

    def test_rejects_missing_or_duplicate_semantic_time_fields(self):
        reports = [semantic_report(str(run)) for run in range(5)]
        invalid = {
            "missing marker": " 16384  maximum resident set size\n",
            "wrong marker version": semantic_time_report(reports[0]).replace(
                "CXF_JSON_TIME_V1", "CXF_JSON_TIME_V2"
            ),
            "prefixed marker": " " + semantic_time_report(reports[0]),
            "valid plus malformed marker": semantic_time_report(reports[0])
            + "CXF_JSON_TIME_V1 run_id=bad\n",
            "valid plus prefixed malformed marker": semantic_time_report(reports[0])
            + " CXF_JSON_TIME_V1 run_id=bad\n",
            "duplicate marker": semantic_time_report(reports[0]).splitlines()[0]
            + "\n"
            + semantic_time_report(reports[0]),
            "duplicate RSS": semantic_time_report(reports[0])
            + " 16384  maximum resident set size\n",
            "split RSS": semantic_time_report(reports[0]).replace(
                " 16384  maximum resident set size", " 16384\n maximum resident set size"
            ),
        }
        for name, contents in invalid.items():
            with self.subTest(name=name), tempfile.TemporaryDirectory() as directory:
                times = []
                for run in range(5):
                    path = Path(directory) / f"semantic-{run}.time"
                    path.write_text(
                        contents if run == 0 else semantic_time_report(reports[run])
                    )
                    times.append(path)
                with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                    benchmarks.summarize_semantic_ingestion(reports, times)

    def test_rejects_invalid_semantic_identity_types(self):
        invalid = {
            "workload_version": "1",
            "retained_values": 1.5,
            "input_bytes": True,
        }
        for field, value in invalid.items():
            with self.subTest(field=field), tempfile.TemporaryDirectory() as directory:
                reports = [semantic_report(str(run)) for run in range(5)]
                for report in reports:
                    report[field] = value
                times = []
                for run in range(5):
                    path = Path(directory) / f"semantic-{run}.time"
                    path.write_text(semantic_time_report(reports[run]))
                    times.append(path)
                with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                    benchmarks.summarize_semantic_ingestion(reports, times)

    def test_rejects_missing_semantic_stage_timing(self):
        reports = [semantic_report(str(run)) for run in range(5)]
        del reports[0]["preflight_ordered_micros"]
        with tempfile.TemporaryDirectory() as directory:
            times = []
            for run in range(5):
                path = Path(directory) / f"semantic-{run}.time"
                path.write_text(semantic_time_report(reports[run]))
                times.append(path)
            with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                benchmarks.summarize_semantic_ingestion(reports, times)

    def test_rejects_invalid_semantic_stage_timing(self):
        invalid = [None, 0, -1, 1.5, "1", True]
        for value in invalid:
            with self.subTest(value=value), tempfile.TemporaryDirectory() as directory:
                reports = [semantic_report(str(run)) for run in range(5)]
                reports[0]["preflight_ordered_micros"] = value
                times = []
                for run in range(5):
                    path = Path(directory) / f"semantic-{run}.time"
                    path.write_text(semantic_time_report(reports[run]))
                    times.append(path)
                with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                    benchmarks.summarize_semantic_ingestion(reports, times)

    def test_rejects_semantic_stage_time_above_elapsed(self):
        reports = [semantic_report(str(run)) for run in range(5)]
        reports[0]["preflight_ordered_micros"] = 6
        with tempfile.TemporaryDirectory() as directory:
            times = []
            for run in range(5):
                path = Path(directory) / f"semantic-{run}.time"
                path.write_text(semantic_time_report(reports[run]))
                times.append(path)
            with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                benchmarks.summarize_semantic_ingestion(reports, times)

    def test_rejects_changed_semantic_workload(self):
        reports = [semantic_report(str(run)) for run in range(5)]
        reports[4]["input_sha256"] = "1" * 64
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            benchmarks.summarize_semantic_ingestion(reports, ["unused"] * 5)

    def test_rejects_semantic_graph_count_mismatch(self):
        reports = [semantic_report(str(run)) for run in range(5)]
        for report in reports:
            report["returned_rdf_quads"] = 1
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            benchmarks.summarize_semantic_ingestion(reports, ["unused"] * 5)

    def test_requires_five_reports(self):
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            benchmarks.summarize_wasm([wasm_report(str(run)) for run in range(4)])

    def test_rejects_reused_run_ids(self):
        reports = [wasm_report("same") for _ in range(5)]
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            benchmarks.summarize_wasm(reports)

    def test_rejects_corpus_regressions(self):
        reports = [corpus_report(str(run)) for run in range(5)]
        reports[0]["unexpected_failures"] = 1
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            benchmarks.summarize_corpus(reports, ["unused"] * 5)

    def test_rejects_zero_stage_time(self):
        reports = [corpus_report(str(run)) for run in range(5)]
        reports[0]["preflight_micros"] = 0
        reports[0]["json_ld_micros"] = 0
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            benchmarks.summarize_corpus(reports, ["unused"] * 5)

    def test_rejects_mismatched_time_report_names(self):
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            benchmarks.validate_time_pairs(
                ["oce-1.json", "oce-2.json"], ["buildings-1.time", "oce-2.time"]
            )

    def test_rejects_reused_report_paths(self):
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            benchmarks.validate_unique_paths(["wasm-1.json"] * 5, "report")


if __name__ == "__main__":
    unittest.main()
