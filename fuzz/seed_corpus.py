#!/usr/bin/env python3
"""Seed local fuzz corpora from the pinned conformance and uv fixtures."""

import argparse
import hashlib
import json
from pathlib import Path
import re


ROOT = Path(__file__).resolve().parents[1]
TARGETS = ("tokenize", "entities", "reader", "document", "differential")
MAX_BYTES = 16_384


def seed_inputs():
    """Return distinct UTF-8 inputs and an audit of all source fixture files."""
    inputs = {}
    report = {
        "fixture_sha256": {},
        "html5lib_inputs": 0,
        "uv_inputs": 0,
        "handwritten_inputs": 0,
        "excluded_non_scalar": 0,
        "excluded_over_size": 0,
    }

    def add(source):
        try:
            data = source.encode("utf-8")
        except UnicodeEncodeError:
            report["excluded_non_scalar"] += 1
            return
        if len(data) > MAX_BYTES:
            report["excluded_over_size"] += 1
            return
        inputs[hashlib.sha256(data).hexdigest()] = data

    def read(path):
        data = path.read_bytes()
        report["fixture_sha256"][str(path.relative_to(ROOT))] = hashlib.sha256(data).hexdigest()
        return data

    fixtures = ROOT / "crates/astral-html/tests/fixtures"
    paths = sorted((fixtures / "html5lib/tokenizer").glob("*.test"))
    if len(paths) != 14:
        raise ValueError("expected all 14 pinned html5lib tokenizer fixture files")
    for path in paths:
        fixture = json.loads(read(path))
        # XML-coercion expectations are out of scope, but their inputs still
        # exercise ordinary tokenizer recovery and can seed fuzzing.
        for test in fixture.get("tests", fixture.get("xmlViolationTests", [])):
            source = test["input"]
            if test.get("doubleEscaped", False):
                source = re.sub(r"\\u([0-9a-fA-F]{4})", lambda match: chr(int(match[1], 16)), source)
            report["html5lib_inputs"] += 1
            add(source)
    if report["excluded_non_scalar"] != 4:
        raise ValueError("the pinned html5lib suite must exclude exactly four non-scalar inputs")

    for path in sorted((fixtures / "uv").glob("*.html")):
        report["uv_inputs"] += 1
        add(read(path).decode("utf-8"))
    if not report["uv_inputs"]:
        raise ValueError("the pinned uv fixture inputs are missing")

    for path in sorted((ROOT / "fuzz/corpus").glob("*/*")):
        if path.is_file():
            report["handwritten_inputs"] += 1
            add(read(path).decode("utf-8"))

    report["unique_inputs"] = len(inputs)
    report["total_bytes"] = sum(map(len, inputs.values()))
    report["corpus_sha256"] = hashlib.sha256("\n".join(sorted(inputs)).encode()).hexdigest()
    return inputs, report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("targets", nargs="*", choices=TARGETS)
    parser.add_argument("--output", type=Path, default=ROOT / "fuzz/generated")
    args = parser.parse_args()
    inputs, report = seed_inputs()
    targets = args.targets or TARGETS
    for target in targets:
        directory = args.output / target
        directory.mkdir(parents=True, exist_ok=True)
        for digest, data in sorted(inputs.items()):
            (directory / digest).write_bytes(data)
    report["targets"] = list(targets)
    print(json.dumps(report, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
