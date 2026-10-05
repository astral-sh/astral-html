#!/usr/bin/env python3
"""Render per-session Criterion estimates without pooling samples or ranking parsers."""

import argparse
import html
import json
import math
import pathlib


def read_json(path):
    return json.loads(path.read_text())


def positive(value):
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ValueError("expected a number")
    if not math.isfinite(value) or value <= 0:
        raise ValueError("expected a finite positive measurement")
    return value


def statistics(directory):
    estimates = read_json(directory / "estimates.json")
    mean = positive(estimates["mean"]["point_estimate"])
    interval = estimates["mean"]["confidence_interval"]
    if not math.isclose(interval["confidence_level"], 0.95):
        raise ValueError("expected a 95% confidence interval")
    lower = positive(interval["lower_bound"])
    upper = positive(interval["upper_bound"])
    if lower > upper:
        raise ValueError("confidence interval bounds are reversed")
    median = positive(estimates["median"]["point_estimate"])
    samples = read_json(directory / "sample.json")
    if len(samples["iters"]) < 2 or len(samples["iters"]) != len(samples["times"]):
        raise ValueError("sample iteration and time arrays must have matching lengths >= 2")
    for value in samples["iters"] + samples["times"]:
        positive(value)
    return mean, lower, upper, median, len(samples["iters"])


def session_results(directory):
    results = {}
    for path in sorted(directory.glob("**/new/benchmark.json")):
        benchmark = read_json(path)
        name = benchmark["full_id"]
        if name in results:
            raise ValueError(f"duplicate Criterion benchmark ID: {name}")
        try:
            mean, lower, upper, median, count = statistics(path.parent)
            throughput = benchmark.get("throughput") or {}
            size = throughput.get("Bytes")
            rate = "—" if size is None else f"{positive(size) / mean * 1e9 / 2**20:.4g}"
            results[name] = [
                "measured",
                f"{mean / 1000:.6g}",
                f"[{lower / 1000:.6g}, {upper / 1000:.6g}]",
                f"{median / 1000:.6g}",
                rate,
                str(count),
            ]
        except (OSError, ValueError, KeyError, TypeError) as error:
            results[name] = [f"invalid statistics: {error}", *["—"] * 5]
    return results


def rows_for_run(run, metadata):
    eligibility = read_json(run / "eligibility.json")
    expected = {}
    for entry in eligibility:
        key = "/".join(entry[field] for field in ("workload", "fixture", "parser"))
        if key in expected:
            raise ValueError(f"duplicate eligibility ID: {key}")
        expected[key] = entry
    rows = []
    for session in metadata["sessions"]:
        measured = session_results(run / "criterion" / session["id"])
        for key, entry in expected.items():
            identity = [entry[field] for field in ("workload", "fixture", "parser")]
            if not entry["eligible"]:
                status = f"ineligible: {entry.get('error', 'output mismatch')}"
                values = [status, *["—"] * 5]
            elif metadata.get("filter") and metadata["filter"] not in key:
                values = ["not selected by filter", *["—"] * 5]
            else:
                values = measured.get(key, ["missing measurement", *["—"] * 5])
            rows.append(identity + [session["id"]] + values)
        unknown = measured.keys() - expected.keys()
        if unknown:
            raise ValueError(f"measurements absent from eligibility: {sorted(unknown)}")
    return rows


HEADERS = [
    "Workload", "Fixture", "Parser", "Session", "Status", "Mean (µs)",
    "Mean 95% CI (µs)", "Median (µs)", "MiB/s", "Samples",
]


def markdown_cell(value):
    return html.escape(str(value)).replace("|", "&#124;").replace("\n", "<br>")


def write_report(run):
    metadata = read_json(run / "metadata.json")
    rows = rows_for_run(run, metadata)
    title = "HTML benchmarks — SMOKE ONLY" if metadata["smoke"] else "HTML benchmarks"
    description = (
        f"Run status: {metadata['status']}. "
        f"Git revision: {metadata['git_before']['revision']}. "
        f"Source initially dirty: {bool(metadata['git_before']['status'])}. "
        f"Source changed during run: {metadata.get('source_changed', 'unknown')}. "
        "Confidence intervals describe the mean within each Criterion session; "
        "sessions are not pooled. Results compare each fixture separately. "
        "No overall ranking is computed. extract-links requires matching decoded link records; "
        "parse-document measures native representations with different tree semantics."
    )
    if metadata["smoke"]:
        description += " These short smoke measurements are not publishable performance results."
    if metadata["git_before"]["status"] or metadata.get("source_changed"):
        description += " This run is unsuitable for publication: the source tree was dirty or changed."
    machine = metadata.get("measurement_machine") or metadata.get("machine", {})
    compiler = metadata.get("rustc", "not recorded").splitlines()[0]
    cpu = machine.get("model name") or machine.get("processor") or machine.get("machine", "not recorded")
    affinity = (metadata["measurement_machine"].get("affinity")
                if "measurement_machine" in metadata else "not recorded (measurement did not start)")
    environment = (
        f"Compiler: {compiler}. CPU: {cpu}. OS: {machine.get('platform', 'not recorded')}. "
        f"Measurement affinity: {affinity}."
    )
    notes = metadata.get("notes") or "No environment notes were supplied."
    links = [
        (session["id"], f"criterion/{session['id']}/report/index.html")
        for session in metadata["sessions"]
        if (run / "criterion" / session["id"] / "report/index.html").exists()
    ]
    markdown = [f"# {title}", "", description, "", f"**Environment:** {markdown_cell(environment)}",
                "", f"**Notes:** {markdown_cell(notes)}", "", " | ".join(HEADERS),
                " | ".join(["---"] * len(HEADERS))]
    markdown.extend(" | ".join(map(markdown_cell, row)) for row in rows)
    markdown += ["", "[Run metadata](metadata.json) · [Eligibility](eligibility.json)"]
    markdown.extend(f"[{label} Criterion reports]({url})" for label, url in links)
    table = "<tr>" + "".join(f"<th>{html.escape(value)}</th>" for value in HEADERS) + "</tr>"
    for row in rows:
        table += "<tr>" + "".join(f"<td>{html.escape(value)}</td>" for value in row) + "</tr>"
    page = (
        '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width">'
        f"<title>{html.escape(title)}</title>"
        "<style>body{font:16px system-ui;margin:2rem}table{border-collapse:collapse}"
        "td,th{padding:.5rem;border:1px solid #aaa;text-align:left}"
        "th{background:#eee}td{vertical-align:top}</style>"
        f"<h1>{html.escape(title)}</h1><p>{html.escape(description)}</p>"
        f"<p><strong>Environment:</strong> {html.escape(environment)}</p>"
        f"<p><strong>Notes:</strong> {html.escape(notes)}</p>"
        '<p><a href="metadata.json">Run metadata</a> · <a href="eligibility.json">Eligibility</a></p>'
        + "".join(f'<p><a href="{url}">{label} Criterion reports</a></p>' for label, url in links)
        + f"<table>{table}</table>"
    )
    paths = [run / "report.md", run / "index.html"]
    if any(path.exists() for path in paths):
        raise FileExistsError("report.md or index.html already exists; refusing to overwrite")
    for path, content in zip(paths, ["\n".join(markdown) + "\n", page]):
        with path.open("x") as output:
            output.write(content)
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run", type=pathlib.Path)
    args = parser.parse_args()
    write_report(args.run)


if __name__ == "__main__":
    main()
