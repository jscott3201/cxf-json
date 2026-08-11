import argparse
import json
import re
import statistics
import sys
from pathlib import Path
from typing import NoReturn


rss_pattern = re.compile(
    r"^[ \t]*(\d+)[ \t]+maximum resident set size$", re.MULTILINE
)
semantic_run_id_pattern = re.compile(r"[0-9a-f]+-[0-9a-f]+")
semantic_time_marker_pattern = re.compile(
    r"CXF_JSON_TIME_V1 "
    r"run_id=([0-9a-f]+-[0-9a-f]+) "
    r"instrumentation_revision=([0-9a-f]{40}) "
    r"workload_version=([0-9]+) "
    r"input_sha256=([0-9a-f]{64})"
)


def fail(message: str) -> NoReturn:
    print(f"benchmark summary error: {message}", file=sys.stderr)
    sys.exit(1)


def distribution(values):
    return {
        "median": statistics.median(values),
        "minimum": min(values),
        "maximum": max(values),
    }


def load_json(paths):
    return [json.loads(Path(path).read_text()) for path in paths]


def stable_value(reports, field):
    values = {json.dumps(report[field], sort_keys=True) for report in reports}
    if len(values) != 1:
        fail(f"{field} differs across runs")
    return reports[0][field]


def validate_reports(reports):
    if len(reports) != 5:
        fail("exactly five reports are required")
    run_ids = [report["run_id"] for report in reports]
    if len(set(run_ids)) != len(run_ids):
        fail("run IDs must be unique")
    revision = stable_value(reports, "instrumentation_revision")
    if not isinstance(revision, str) or re.fullmatch(r"[0-9a-f]{40}", revision) is None:
        fail("instrumentation_revision is not a 40-digit commit ID")
    return revision


def validate_unique_paths(paths, kind):
    resolved = [Path(path).resolve() for path in paths]
    if len(set(resolved)) != len(resolved):
        fail(f"{kind} paths must be unique")


def validate_time_pairs(report_paths, time_paths):
    for report, time_report in zip(report_paths, time_paths):
        if Path(report).stem != Path(time_report).stem:
            fail(f"report and time-report names do not match: {report}, {time_report}")


def positive_integer(report, field):
    if field not in report or type(report[field]) is not int or report[field] <= 0:
        fail(f"{field} must be a positive integer")
    return report[field]


def semantic_time_rss(report, path):
    text = Path(path).read_text()
    marker_lines = [line for line in text.splitlines() if "CXF_JSON_TIME_" in line]
    if len(marker_lines) != 1:
        fail(f"semantic time report must contain exactly one V1 identity marker: {path}")
    marker = semantic_time_marker_pattern.fullmatch(marker_lines[0])
    if marker is None:
        fail(f"semantic time report contains a malformed identity marker: {path}")
    expected = (
        report["run_id"],
        report["instrumentation_revision"],
        str(report["workload_version"]),
        report["input_sha256"],
    )
    if marker.groups() != expected:
        fail(f"semantic time-report identity does not match its JSON report: {path}")

    rss_matches = list(rss_pattern.finditer(text))
    if len(rss_matches) != 1:
        fail(f"semantic time report must contain exactly one maximum RSS value: {path}")
    return int(rss_matches[0].group(1))


def file_identity(report):
    return [
        {
            "path": file["path"],
            "expected_failure": file["expected_failure"],
            "input_bytes": file["input_bytes"],
            "failure_kind": (
                None if file["failure"] is None else file["failure"]["kind"]
            ),
        }
        for file in report["files"]
    ]


def summarize_corpus(reports, time_reports):
    instrumentation_revision = validate_reports(reports)
    if len(reports) != len(time_reports):
        fail("corpus report and time-report counts differ")
    commit = stable_value(reports, "git_commit")
    if not isinstance(commit, str) or re.fullmatch(r"[0-9a-f]{40}", commit) is None:
        fail("git_commit is not a 40-digit commit ID")
    if not all(report["git_origin_matches_expected"] for report in reports):
        fail("corpus reports must come from one verified Git commit")
    identities = {json.dumps(file_identity(report), sort_keys=True) for report in reports}
    if len(identities) != 1:
        fail("file identity differs across runs")
    for field in ["unexpected_failures", "unexpected_passes", "read_failures"]:
        if any(report[field] != 0 for report in reports):
            fail(f"one or more runs report {field}")
    if not all(report["structural_metrics_complete"] for report in reports):
        fail("structural metrics are incomplete")
    if any(
        report["preflight_micros"] is None or report["json_ld_micros"] is None
        for report in reports
    ):
        fail("one or more runs have incomplete stage timing")
    stable_value(reports, "passed")
    stable_value(reports, "expected_failures")

    combined_stage = [
        report["preflight_micros"] + report["json_ld_micros"] for report in reports
    ]
    if any(elapsed == 0 for elapsed in combined_stage):
        fail("combined stage timing is zero")
    rss = []
    for path in time_reports:
        match = rss_pattern.search(Path(path).read_text())
        if match is None:
            fail(f"maximum RSS is missing from {path}")
        rss.append(int(match.group(1)))

    throughput = [
        report["input_bytes"] / elapsed for report, elapsed in zip(reports, combined_stage)
    ]
    stable_fields = [
        "file_count",
        "input_bytes",
        "max_input_bytes",
        "quad_count",
        "max_nesting_depth",
        "max_object_members",
        "total_json_values",
        "decoded_member_name_bytes",
        "measured_structure_files",
    ]

    return {
        "runs": len(reports),
        "instrumentation_revision": instrumentation_revision,
        "git_commit": commit,
        "structure": {field: stable_value(reports, field) for field in stable_fields},
        "rdf_term_bytes": distribution([report["rdf_term_bytes"] for report in reports]),
        "preflight_micros": distribution(
            [report["preflight_micros"] for report in reports]
        ),
        "json_ld_micros": distribution(
            [report["json_ld_micros"] for report in reports]
        ),
        "combined_stage_micros": distribution(combined_stage),
        "stage_throughput_mb_s": distribution(throughput),
        "corpus_micros": distribution([report["elapsed_micros"] for report in reports]),
        "maximum_rss_bytes": distribution(rss),
    }


def summarize_wasm(reports):
    instrumentation_revision = validate_reports(reports)
    stable_fields = [
        "node",
        "platform",
        "architecture",
        "module_bytes",
        "module_sha256",
        "initial_memory_bytes",
        "final_memory_bytes",
    ]
    module_sha256 = stable_value(reports, "module_sha256")
    if not isinstance(module_sha256, str) or re.fullmatch(
        r"[0-9a-f]{64}", module_sha256
    ) is None:
        fail("module_sha256 is not a lowercase SHA-256 digest")
    return {
        "runs": len(reports),
        "instrumentation_revision": instrumentation_revision,
        "environment": {field: stable_value(reports, field) for field in stable_fields[:3]},
        "module_bytes": stable_value(reports, "module_bytes"),
        "module_sha256": module_sha256,
        "compile_micros": distribution([report["compile_micros"] for report in reports]),
        "instantiate_micros": distribution(
            [report["instantiate_micros"] for report in reports]
        ),
        "execute_micros": distribution([report["execute_micros"] for report in reports]),
        "initial_memory_bytes": stable_value(reports, "initial_memory_bytes"),
        "final_memory_bytes": stable_value(reports, "final_memory_bytes"),
    }


def stress_case_identity(case):
    identity = {
        field: case[field]
        for field in [
            "name",
            "family",
            "parameters",
            "input_bytes",
            "input_sha256",
            "expected",
            "actual",
        ]
    }
    identity["json_metrics"] = (
        None if case["metrics"] is None else case["metrics"]["json"]
    )
    return identity


def summarize_resource_stress(reports, time_reports):
    instrumentation_revision = validate_reports(reports)
    if len(reports) != len(time_reports):
        fail("resource-stress report and time-report counts differ")
    if any(report["unexpected_outcomes"] != 0 for report in reports):
        fail("one or more resource-stress runs have unexpected outcomes")

    generator_version = stable_value(reports, "generator_version")
    case_count = stable_value(reports, "case_count")
    input_bytes = stable_value(reports, "input_bytes")
    identities = [
        [stress_case_identity(case) for case in report["cases"]] for report in reports
    ]
    if len({json.dumps(identity, sort_keys=True) for identity in identities}) != 1:
        fail("resource-stress case identity differs across runs")
    if len(identities[0]) != case_count:
        fail("resource-stress case count does not match the report")
    for case in identities[0]:
        if re.fullmatch(r"[0-9a-f]{64}", case["input_sha256"]) is None:
            fail(f"{case['name']} input_sha256 is not a lowercase SHA-256 digest")

    rss = []
    for path in time_reports:
        match = rss_pattern.search(Path(path).read_text())
        if match is None:
            fail(f"maximum RSS is missing from {path}")
        rss.append(int(match.group(1)))

    cases = []
    for index, identity in enumerate(identities[0]):
        preflight = [report["cases"][index]["preflight_micros"] for report in reports]
        json_ld = [report["cases"][index]["json_ld_micros"] for report in reports]
        if any(value is None for value in json_ld):
            if not all(value is None for value in json_ld):
                fail(f"{identity['name']} has inconsistent JSON-LD timing")
            json_ld_distribution = None
        else:
            json_ld_distribution = distribution(json_ld)
        rdf_term_bytes = [
            None if report["cases"][index]["metrics"] is None
            else report["cases"][index]["metrics"]["rdf_term_bytes"]
            for report in reports
        ]
        if any(value is None for value in rdf_term_bytes):
            if not all(value is None for value in rdf_term_bytes):
                fail(f"{identity['name']} has inconsistent RDF term metrics")
            rdf_term_distribution = None
        else:
            rdf_term_distribution = distribution(rdf_term_bytes)
        cases.append(
            {
                **identity,
                "rdf_term_bytes": rdf_term_distribution,
                "preflight_micros": distribution(preflight),
                "json_ld_micros": json_ld_distribution,
            }
        )

    return {
        "runs": len(reports),
        "instrumentation_revision": instrumentation_revision,
        "generator_version": generator_version,
        "case_count": case_count,
        "input_bytes": input_bytes,
        "cases": cases,
        "report_micros": distribution(
            [report["elapsed_micros"] for report in reports]
        ),
        "maximum_rss_bytes": distribution(rss),
    }


def summarize_semantic_ingestion(reports, time_reports):
    instrumentation_revision = validate_reports(reports)
    if len(reports) != len(time_reports):
        fail("semantic-ingestion report and time-report counts differ")
    stable_fields = [
        "workload_version",
        "retained_values",
        "input_bytes",
        "input_sha256",
        "outcome",
        "source_matches_input",
        "max_nesting_depth",
        "max_object_members",
        "total_values",
        "decoded_member_name_bytes",
        "emitted_rdf_quads",
        "retained_rdf_term_bytes",
        "returned_rdf_quads",
    ]
    stable = {field: stable_value(reports, field) for field in stable_fields}
    for field in ["workload_version", "retained_values", "input_bytes"]:
        if type(stable[field]) is not int or stable[field] <= 0:
            fail(f"{field} must be a positive integer")
    for field in [
        "max_nesting_depth",
        "max_object_members",
        "total_values",
        "decoded_member_name_bytes",
        "emitted_rdf_quads",
        "retained_rdf_term_bytes",
        "returned_rdf_quads",
    ]:
        if type(stable[field]) is not int or stable[field] < 0:
            fail(f"{field} must be a non-negative integer")
    if not all(
        isinstance(report["run_id"], str)
        and semantic_run_id_pattern.fullmatch(report["run_id"]) is not None
        for report in reports
    ):
        fail("semantic-ingestion run_id has an invalid format")
    if not isinstance(stable["input_sha256"], str) or re.fullmatch(
        r"[0-9a-f]{64}", stable["input_sha256"]
    ) is None:
        fail("input_sha256 is not a lowercase SHA-256 digest")
    if stable["outcome"] != "success" or stable["source_matches_input"] is not True:
        fail("semantic-ingestion reports must describe source-preserving success")
    if stable["returned_rdf_quads"] != stable["emitted_rdf_quads"]:
        fail("returned and emitted RDF quad counts differ")
    elapsed = [positive_integer(report, "elapsed_micros") for report in reports]
    preflight_ordered = [
        positive_integer(report, "preflight_ordered_micros") for report in reports
    ]
    jsonld_quad_retention = [
        positive_integer(report, "jsonld_quad_retention_micros")
        for report in reports
    ]
    combined_stage = [
        preflight + jsonld
        for preflight, jsonld in zip(preflight_ordered, jsonld_quad_retention)
    ]
    if any(combined > total for combined, total in zip(combined_stage, elapsed)):
        fail("combined semantic stage time exceeds elapsed time")

    rss = [
        semantic_time_rss(report, path)
        for report, path in zip(reports, time_reports)
    ]

    throughput = [stable["input_bytes"] / value for value in elapsed]
    stage_throughput = [stable["input_bytes"] / value for value in combined_stage]
    return {
        "runs": len(reports),
        "instrumentation_revision": instrumentation_revision,
        "workload": {
            field: stable[field]
            for field in [
                "workload_version",
                "retained_values",
                "input_bytes",
                "input_sha256",
            ]
        },
        "structure": {
            field: stable[field]
            for field in [
                "max_nesting_depth",
                "max_object_members",
                "total_values",
                "decoded_member_name_bytes",
            ]
        },
        "rdf": {
            field: stable[field]
            for field in [
                "emitted_rdf_quads",
                "retained_rdf_term_bytes",
                "returned_rdf_quads",
            ]
        },
        "preflight_ordered_micros": distribution(preflight_ordered),
        "jsonld_quad_retention_micros": distribution(jsonld_quad_retention),
        "combined_stage_micros": distribution(combined_stage),
        "stage_throughput_mb_s": distribution(stage_throughput),
        "elapsed_micros": distribution(elapsed),
        "throughput_mb_s": distribution(throughput),
        "maximum_rss_bytes": distribution(rss),
    }


def main():
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="mode", required=True)
    corpus = subparsers.add_parser("corpus")
    corpus.add_argument("reports", nargs="+")
    corpus.add_argument("--times", nargs="+", required=True)
    wasm = subparsers.add_parser("wasm")
    wasm.add_argument("reports", nargs="+")
    stress = subparsers.add_parser("resource-stress")
    stress.add_argument("reports", nargs="+")
    stress.add_argument("--times", nargs="+", required=True)
    semantic = subparsers.add_parser("semantic-ingestion")
    semantic.add_argument("reports", nargs="+")
    semantic.add_argument("--times", nargs="+", required=True)
    arguments = parser.parse_args()

    reports = load_json(arguments.reports)
    validate_unique_paths(arguments.reports, "report")
    if arguments.mode in ["corpus", "resource-stress", "semantic-ingestion"]:
        validate_unique_paths(arguments.times, "time-report")
        validate_time_pairs(arguments.reports, arguments.times)
        if arguments.mode == "corpus":
            summary = summarize_corpus(reports, arguments.times)
        elif arguments.mode == "semantic-ingestion":
            summary = summarize_semantic_ingestion(reports, arguments.times)
        else:
            summary = summarize_resource_stress(reports, arguments.times)
    else:
        summary = summarize_wasm(reports)
    json.dump(summary, sys.stdout, indent=2)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
