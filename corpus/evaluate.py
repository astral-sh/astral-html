#!/usr/bin/env python3
"""Compile the evaluator against a specified astral-html checkout.

Usage: python3 corpus/evaluate.py /path/to/checkout baseline|recovering
The corpus must have been captured/downloaded and verified first.
"""

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
WORK = ROOT / ".corpus" / "evaluation"


def main():
    if len(sys.argv) != 3 or sys.argv[2] not in ("baseline", "recovering"):
        raise SystemExit(__doc__)
    checkout = Path(sys.argv[1]).resolve()
    mode = sys.argv[2]
    if not (checkout / "crates" / "astral-html" / "Cargo.toml").exists():
        raise SystemExit(f"not an astral-html checkout: {checkout}")
    subprocess.run([sys.executable, str(ROOT / "corpus" / "fetch.py"), "verify"], check=True)
    (WORK / "src").mkdir(parents=True, exist_ok=True)
    shutil.copyfile(ROOT / "corpus" / "evaluator.rs", WORK / "src" / "main.rs")
    (WORK / "Cargo.toml").write_text(
        "[package]\nname = \"astral-html-corpus-evaluator\"\nversion = \"0.0.0\"\nedition = \"2024\"\n"
        + "[workspace]\n"
        + f"[dependencies]\nastral-html = {{ path = {json.dumps(str(checkout / 'crates' / 'astral-html'))} }}\n"
        + 'serde_json = "1"\n[features]\nrecovering = []\n'
    )
    command = ["cargo", "run", "--quiet", "--manifest-path", str(WORK / "Cargo.toml")]
    if mode == "recovering":
        command.extend(["--features", "recovering"])
    entries = json.loads((ROOT / "corpus" / "manifest.json").read_text())["entries"]
    command.extend(["--", mode, str(ROOT / ".corpus" / "pages")])
    command.extend(entry["file"] for entry in entries)
    environment = os.environ.copy()
    # Some development hosts mount the default Cargo cache read-only.
    environment.setdefault("CARGO_HOME", str(ROOT / ".corpus" / "cargo-home"))
    result = subprocess.run(command, stdout=subprocess.PIPE, check=True, text=True, env=environment)
    rows = [json.loads(line) for line in result.stdout.splitlines()]
    rows.sort(key=lambda row: row["file"])
    if len(rows) != len(entries):
        raise SystemExit(f"evaluator emitted only {len(rows)} pages")
    result_file = ROOT / ".corpus" / f"{mode}-results.json"
    result_file.write_text(json.dumps(rows, indent=2) + "\n")
    print(f"Saved {len(rows)} results to {result_file}")
    for row in rows:
        baseline = row["baseline"]
        recovering = row.get("recovering")
        difference = row.get("difference")
        base_count = baseline.get("summary", {}).get("elements", "error")
        recovered_count = recovering.get("summary", {}).get("elements", "error") if recovering else "-"
        changed = difference.get("changed_parents", "-") if difference else "-"
        print(f"{row['file']:<45} {base_count!s:>8} {recovered_count!s:>8} {changed!s:>8}")


if __name__ == "__main__":
    main()
