#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["matplotlib==3.10.8"]
# ///
"""Render the README extraction chart from preserved Criterion measurements."""

import argparse
import json
import math
from pathlib import Path
from statistics import median

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt

WORKSPACE = Path(__file__).resolve().parents[1]
LABELS = {
    "astral-reader": "astral-html Reader",
    "astral-document": "astral-html Document",
    "astral-tl": "astral-tl",
    "html5gum": "html5gum",
    "lol-html": "lol_html",
    "scraper": "scraper",
}
THEMES = {
    "light": {
        "background": "#ffffff", "text": "#24292f", "muted": "#57606a",
        "grid": "#eaeef2", "reader": "#7545d0", "document": "#aa8ddc", "other": "#929aa5",
    },
    "dark": {
        "background": "#0d1117", "text": "#e6edf3", "muted": "#a5aeb9",
        "grid": "#252c35", "reader": "#b29aff", "document": "#8065b5", "other": "#798594",
    },
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
    }
    if eligibility.keys() != LABELS.keys() or not all(row["eligible"] for row in eligibility.values()):
        raise ValueError("all six extraction implementations must be eligible")
    values = {parser: {} for parser in LABELS}
    for row in data["measurements"]:
        workload, case, parser = row["benchmark"]["full_id"].split("/")
        if workload != "extract-links" or case != fixture:
            continue
        mean = row["estimates"]["mean"]["point_estimate"] / 1e6
        if not math.isfinite(mean) or mean <= 0 or row["session"] in values[parser]:
            raise ValueError("expected one positive mean per implementation and session")
        values[parser][row["session"]] = mean
    if any(means.keys() != sessions for means in values.values()):
        raise ValueError("missing extraction measurements")
    return sorted(
        [(parser, median(means.values()), min(means.values()), max(means.values()))
         for parser, means in values.items()],
        key=lambda row: row[1],
    ), len(sessions), eligibility["astral-reader"]["links"]


def render(rows, sessions, links, fixture, metadata, theme, output, preview):
    colors = THEMES[theme]
    with plt.rc_context({
        "font.family": "DejaVu Sans", "font.size": 11,
        "svg.hashsalt": "astral-html-extraction", "svg.fonttype": "path",
        "text.color": colors["text"], "axes.labelcolor": colors["muted"],
        "xtick.color": colors["muted"], "ytick.color": colors["text"],
    }):
        figure, axis = plt.subplots(figsize=(9.2, 4.8), facecolor=colors["background"])
        figure.subplots_adjust(left=0.28, right=0.96, top=0.78, bottom=0.23)
        axis.set_facecolor(colors["background"])
        upper = max(row[3] for row in rows)
        for position, (parser, center, low, high) in enumerate(rows):
            color = colors["reader" if parser == "astral-reader" else
                           "document" if parser == "astral-document" else "other"]
            axis.barh(position, center, height=0.6, color=color, zorder=2)
            axis.errorbar(center, position, xerr=[[center - low], [high - center]],
                          fmt="none", ecolor=colors["text"], capsize=3, linewidth=1, zorder=3)
            axis.text(high + upper * 0.025, position, f"{center:.2f} ms", va="center",
                      fontsize=11, weight="bold" if parser == "astral-reader" else "normal")
        axis.set_yticks(range(len(rows)), [LABELS[row[0]] for row in rows])
        for label, row in zip(axis.get_yticklabels(), rows):
            if row[0] == "astral-reader":
                label.set_weight("bold")
                label.set_color(colors["reader"])
        axis.set_ylim(len(rows) - 0.45, -0.65)
        axis.set_xlim(0, upper * 1.17)
        axis.set_xlabel("Time (ms) · lower is better", labelpad=9)
        axis.tick_params(axis="both", length=0, pad=10)
        axis.xaxis.grid(True, color=colors["grid"], linewidth=0.8, zorder=0)
        for spine in axis.spines.values():
            spine.set_visible(False)
        figure.text(0.03, 0.925, "Link extraction", fontsize=20, weight="bold")
        figure.text(0.03, 0.86, f"PEP index · {links:,} links · {fixture['bytes'] / 1024:.0f} KiB HTML",
                    fontsize=12, color=colors["muted"])
        figure.text(0.03, 0.075,
                    f"Median of {sessions} session means · whiskers show the min–max session range",
                    fontsize=9, color=colors["muted"])
        environment = metadata.get("notes") or metadata["measurement_machine"]["model name"]
        environment = environment.split(". ", 1)[0].rstrip(".")
        figure.text(0.03, 0.025,
                    f"Warm input · {environment} · source {metadata['git_before']['revision'][:7]}",
                    fontsize=9, color=colors["muted"])
        description = (
            "Parse HTML, extract owned decoded href/title/rel records, and drop all state. "
            f"{fixture['bytes']} input bytes. {metadata['notes']}"
        )
        svg = output / f"extraction-{theme}.svg"
        figure.savefig(svg, metadata={
            "Date": None, "Title": "Link extraction from the PEP index", "Description": description,
        })
        svg.write_text("\n".join(line.rstrip() for line in svg.read_text().splitlines()) + "\n")
        if preview:
            figure.savefig(output / f"extraction-{theme}.png", dpi=160)
        plt.close(figure)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("data", type=Path, help="preserved benchmark result JSON")
    parser.add_argument("--output", type=Path, default=WORKSPACE.parent / "docs/assets")
    parser.add_argument("--preview", action="store_true", help="also render PNG previews")
    args = parser.parse_args()
    data = json.loads(args.data.read_text())
    fixture = next(f for f in json.loads((WORKSPACE / "fixtures/manifest.json").read_text())["fixtures"]
                   if f["id"] == "pep-index")
    recorded = data["metadata"]["fixtures"][f"fixtures/{fixture['file']}"]
    if recorded["sha256"] != fixture["sha256"] or recorded["bytes"] != fixture["bytes"]:
        raise ValueError("the chart fixture differs from the measured input")
    rows, sessions, links = chart_data(data, fixture["id"])
    args.output.mkdir(parents=True, exist_ok=True)
    for theme in THEMES:
        render(rows, sessions, links, fixture, data["metadata"], theme, args.output, args.preview)


if __name__ == "__main__":
    main()
