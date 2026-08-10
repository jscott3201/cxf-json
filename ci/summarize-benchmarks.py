import argparse
import json
import re
import statistics
import sys
from pathlib import Path


rss_pattern = re.compile(r"^\s*(\d+)\s+maximum resident set size$", re.MULTILINE)


def fail(message):
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


def validate_report_count(reports):
    if len(reports) != 5:
        fail("exactly five reports are required")


def validate_time_pairs(report_paths, time_paths):
    for report, time_report in zip(report_paths, time_paths):
        if Path(report).stem != Path(time_report).stem:
            fail(f"report and time-report names do not match: {report}, {time_report}")


def file_identity(report):
    return [
        {
            "path": file["path"],
            "expected_failure": file["expected_failure"],
            "input_bytes": file["input_bytes"],
            "failure_kind": None if file["failure"] is None else file["failure"]["kind"],
        }
        for file in report["files"]
    ]


def summarize_corpus(reports, time_reports):
    validate_report_count(reports)
    if len(reports) != len(time_reports):
        fail("corpus report and time-report counts differ")
    commit = stable_value(reports, "git_commit")
    if not commit or not all(report["git_origin_matches_expected"] for report in reports):
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

    rss = []
    for path in time_reports:
        match = rss_pattern.search(Path(path).read_text())
        if match is None:
            fail(f"maximum RSS is missing from {path}")
        rss.append(int(match.group(1)))

    combined_stage = [
        report["preflight_micros"] + report["json_ld_micros"] for report in reports
    ]
    if any(elapsed == 0 for elapsed in combined_stage):
        fail("combined stage timing is zero")
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
    validate_report_count(reports)
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
    if re.fullmatch(r"[0-9a-f]{64}", module_sha256) is None:
        fail("module_sha256 is not a lowercase SHA-256 digest")
    return {
        "runs": len(reports),
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


def main():
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="mode", required=True)
    corpus = subparsers.add_parser("corpus")
    corpus.add_argument("reports", nargs="+")
    corpus.add_argument("--times", nargs="+", required=True)
    wasm = subparsers.add_parser("wasm")
    wasm.add_argument("reports", nargs="+")
    arguments = parser.parse_args()

    reports = load_json(arguments.reports)
    if arguments.mode == "corpus":
        validate_time_pairs(arguments.reports, arguments.times)
        summary = summarize_corpus(reports, arguments.times)
    else:
        summary = summarize_wasm(reports)
    json.dump(summary, sys.stdout, indent=2)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
