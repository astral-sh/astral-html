#!/usr/bin/env python3
"""Fill Ruff's original README SVG with our preserved extraction measurements."""

import argparse
from copy import deepcopy
import json
import math
from pathlib import Path
from statistics import median
import xml.etree.ElementTree as ET

WORKSPACE = Path(__file__).resolve().parents[1]
SVG = "{http://www.w3.org/2000/svg}"
ET.register_namespace("", SVG[1:-1])
LABELS = {
    "astral-reader": "astral-html",
    "tl": "tl",
    "html5gum": "html5gum",
    "lol-html": "lol_html",
    "scraper": "scraper",
}


def chart_data(data, fixture):
    metadata = data["metadata"]
    if (metadata["status"] != "complete" or metadata["smoke"]
            or metadata["git_before"]["status"] or metadata["source_changed"]):
        raise ValueError("expected a complete, clean, non-smoke run")
    sessions = {session["id"] for session in metadata["sessions"]}
    if len(sessions) < 3 or any(session["status"] != "complete" for session in metadata["sessions"]):
        raise ValueError("expected at least three completed sessions")
    eligibility = {
        row["parser"]: row for row in data["eligibility"]
        if row["workload"] == "extract-links" and row["fixture"] == fixture
        and row["parser"] in LABELS
    }
    if eligibility.keys() != LABELS.keys() or not all(row["eligible"] for row in eligibility.values()):
        raise ValueError("all five chart implementations must be eligible")
    values = {parser: {} for parser in LABELS}
    for row in data["measurements"]:
        workload, case, parser = row["benchmark"]["full_id"].split("/")
        if workload != "extract-links" or case != fixture or parser not in LABELS:
            continue
        mean = row["estimates"]["mean"]["point_estimate"] / 1e6
        if not math.isfinite(mean) or mean <= 0 or row["session"] in values[parser]:
            raise ValueError("expected one positive mean per implementation and session")
        values[parser][row["session"]] = mean
    if any(means.keys() != sessions for means in values.values()):
        raise ValueError("missing extraction measurements")
    return sorted(
        [(parser, median(means.values())) for parser, means in values.items()],
        key=lambda row: row[1],
    ), len(sessions), eligibility["astral-reader"]["links"]


def group(root, class_name):
    return next(node for node in root.iter(f"{SVG}g") if node.get("class") == class_name)


def render(rows, sessions, links, fixture, metadata, output):
    root = ET.parse(WORKSPACE / "scripts/templates/ruff.svg").getroot()
    title = ET.Element(f"{SVG}title")
    title.text = "Link extraction from the PEP index"
    root.insert(0, title)
    description = ET.Element(f"{SVG}desc")
    description.text = (
        f"{links:,} links from {fixture['bytes']:,} bytes of warm HTML input. "
        f"Median of {sessions} session means; lower is better. astral-html uses Reader. "
        f"Source {metadata['git_before']['revision']}. {metadata['notes']}"
    )
    root.insert(1, description)
    license_text = (WORKSPACE / "scripts/templates/LICENSE-RUFF").read_text().strip()
    root.insert(2, ET.Comment(
        "\nAdapted from Ruff's README benchmark SVG:\n"
        "https://user-images.githubusercontent.com/1309177/232603516-4fb4892d-585c-4b20-b810-3db9161831e4.svg\n\n"
        + license_text + "\n"
    ))

    # Retain Ruff's 585×167 viewBox, 483×140 plot, 13px bars, font, and colors.
    width, height = 483, 140
    magnitude = 10 ** math.floor(math.log10(max(value for _, value in rows) / 3))
    step = math.ceil(max(value for _, value in rows) / 3 / magnitude) * magnitude
    maximum = step * 3
    for class_name in ["mark-rule role-axis-grid", "mark-rule role-axis-tick"]:
        for index, line in enumerate(group(root, class_name)):
            line.set("transform", f"translate({index * width / 3:g},0)")
    axes = [node for node in root.iter(f"{SVG}g") if node.get("aria-roledescription") == "axis"]
    axes[0].set("aria-label", f"X-axis: time in milliseconds from 0 to {maximum:g}")
    for index, label in enumerate(group(axes[0], "mark-text role-axis-label")):
        label.set("transform", f"translate({index * width / 3:g},15)")
        label.text = f"{index * step:g}ms"
    axes[1].set("aria-label", "Y-axis: " + ", ".join(LABELS[parser] for parser, _ in rows))

    labels = group(axes[1], "mark-text role-axis-label")
    bars = group(root, "mark-rect role-mark layer_0_marks")
    times = group(root, "mark-text role-mark layer_1_marks")
    prototypes = [deepcopy(container[0]) for container in [labels, bars, times]]
    for container in [labels, bars, times]:
        container[:] = []
    for index, (parser, value) in enumerate(rows):
        label, bar, time = map(deepcopy, prototypes)
        center = (index + 0.5) * height / len(rows)
        length = value / maximum * width
        label.text = LABELS[parser]
        label.set("transform", f"translate(-10,{center + 3.5:g})")
        bar.set("d", f"M0,{center - 6.5:g}h{length:.9f}v13h-{length:.9f}Z")
        bar.set("aria-label", f"time: {value:.9f}ms; tool: {LABELS[parser]}")
        time.text = f"{value:.2f}ms"
        time.set("transform", f"translate({length + 6:.9f},{center + 4:g})")
        time.set("aria-label", bar.get("aria-label"))
        if parser != "astral-reader":
            label.attrib.pop("font-weight", None)
            time.attrib.pop("font-weight", None)
        for container, element in zip([labels, bars, times], [label, bar, time]):
            container.append(element)

    ET.indent(root)
    light = ET.tostring(root, encoding="unicode") + "\n"
    # This is the same sole color substitution used by Ruff's original dark SVG.
    for theme, svg in [("light", light), ("dark", light.replace("#333333", "#C9D1D9"))]:
        (output / f"extraction-{theme}.svg").write_text(svg)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("data", type=Path, help="preserved benchmark result JSON")
    parser.add_argument("--output", type=Path, default=WORKSPACE.parent / "docs/assets")
    args = parser.parse_args()
    data = json.loads(args.data.read_text())
    fixture = next(f for f in json.loads((WORKSPACE / "fixtures/manifest.json").read_text())["fixtures"]
                   if f["id"] == "pep-index")
    recorded = data["metadata"]["fixtures"][f"fixtures/{fixture['file']}"]
    if recorded["sha256"] != fixture["sha256"] or recorded["bytes"] != fixture["bytes"]:
        raise ValueError("the chart fixture differs from the measured input")
    rows, sessions, links = chart_data(data, fixture["id"])
    args.output.mkdir(parents=True, exist_ok=True)
    render(rows, sessions, links, fixture, data["metadata"], args.output)


if __name__ == "__main__":
    main()
