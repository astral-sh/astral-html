#!/usr/bin/env python3
"""Run independent Criterion sessions and preserve their data (Python 3.11+)."""

import argparse
import datetime
import hashlib
import importlib.util
import json
import os
import pathlib
import platform
import re
import shlex
import shutil
import subprocess
import tomllib

WORKSPACE = pathlib.Path(__file__).resolve().parents[1]
REPOSITORY = WORKSPACE.parent
# Load the sibling explicitly, including when Python's safe-path mode is enabled.
REPORT_SPEC = importlib.util.spec_from_file_location("benchmark_report", WORKSPACE / "scripts/report.py")
REPORT = importlib.util.module_from_spec(REPORT_SPEC)
REPORT_SPEC.loader.exec_module(REPORT)
ENVIRONMENT_KEYS = [
    "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "CARGO_BUILD_TARGET", "CARGO_TARGET_DIR",
    "CARGO_BUILD_BUILD_DIR", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER",
    "CARGO_PROFILE_RELEASE_LTO", "CARGO_PROFILE_RELEASE_CODEGEN_UNITS",
    "CARGO_PROFILE_RELEASE_OPT_LEVEL", "CARGO_PROFILE_RELEASE_DEBUG",
    "CARGO_PROFILE_BENCH_LTO", "CARGO_PROFILE_BENCH_CODEGEN_UNITS",
    "CARGO_PROFILE_BENCH_OPT_LEVEL", "CARGO_PROFILE_BENCH_DEBUG",
    "CARGO_UNSTABLE_OHM_PROC_MACRO_TRUST", "CARGO_UNSTABLE_OHM_NATIVE_TOOL_TRUST",
]


def timestamp():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def capture(command):
    return subprocess.check_output(command, cwd=WORKSPACE, text=True).strip()


def git_state():
    return {
        "revision": capture(["git", "rev-parse", "HEAD"]),
        "status": capture(["git", "status", "--porcelain=v1", "--untracked-files=all"]),
        "diff_sha256": hashlib.sha256(
            subprocess.check_output(["git", "diff", "--binary", "HEAD"], cwd=WORKSPACE)
        ).hexdigest(),
    }


def fixture_hashes():
    return {
        str(path.relative_to(WORKSPACE)): {"bytes": path.stat().st_size, "sha256": digest(path)}
        for path in sorted((WORKSPACE / "fixtures").rglob("*")) if path.is_file()
    }


def source_hashes():
    paths = [
        REPOSITORY / "Cargo.toml", REPOSITORY / "rust-toolchain.toml",
        WORKSPACE / "Cargo.toml", REPOSITORY / "crates/astral-html/Cargo.toml",
    ]
    paths += list((REPOSITORY / "crates/astral-html/src").rglob("*.rs"))
    paths += list((WORKSPACE / "src").rglob("*.rs"))
    paths += list((WORKSPACE / "benches").rglob("*.rs"))
    paths += list((WORKSPACE / "scripts").glob("*.py"))
    return {str(path.relative_to(REPOSITORY)): digest(path) for path in sorted(paths)}


def machine_info():
    info = {
        "platform": platform.platform(), "machine": platform.machine(),
        "processor": platform.processor(), "logical_cpus": os.cpu_count(),
        "affinity": sorted(os.sched_getaffinity(0)) if hasattr(os, "sched_getaffinity") else None,
    }
    cpuinfo = pathlib.Path("/proc/cpuinfo")
    if cpuinfo.exists():
        fields = {"model name", "vendor_id", "cpu family", "model", "stepping", "microcode"}
        for line in cpuinfo.read_text().splitlines():
            key, separator, value = line.partition(":")
            if separator and key.strip() in fields:
                info.setdefault(key.strip(), value.strip())
    elif platform.system() == "Darwin":
        info["model name"] = capture(["sysctl", "-n", "machdep.cpu.brand_string"])
    return info


def create_run_directory(path):
    # Existing outputs may be evidence from a completed or interrupted run.
    path.mkdir(parents=True, exist_ok=False)


def select_benchmarks(eligibility, substring):
    selected = [
        "/".join(row[field] for field in ("workload", "fixture", "parser"))
        for row in eligibility if row["eligible"]
    ]
    selected = [name for name in selected if not substring or substring in name]
    if not selected:
        raise ValueError(f"no eligible benchmark IDs match the filter {substring!r}")
    return selected


def save_metadata(path, metadata):
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(metadata, indent=2, allow_nan=False) + "\n")
    temporary.replace(path)


def run_command(command, log, environment, metadata):
    metadata["commands"].append({"argv": command, "log": log.name})
    save_metadata(log.parent / "metadata.json", metadata)
    print(shlex.join(command), flush=True)
    with log.open("x") as output:
        subprocess.run(
            command, cwd=WORKSPACE, env=environment, stdout=output,
            stderr=subprocess.STDOUT, check=True,
        )


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=pathlib.Path, required=True, help="new artifact directory")
    parser.add_argument("--toolchain", help="Rust toolchain (default: repository rust-toolchain.toml)")
    parser.add_argument("--no-ohm-defaults", action="store_true")
    parser.add_argument("--runs", type=int, default=3, help="independent sessions (default: 3)")
    parser.add_argument("--seed", type=int, default=1729, help="first BENCH_ORDER_SEED")
    parser.add_argument("--cpu", type=int, help="pin this process and its children to one CPU")
    parser.add_argument("--filter", help="literal substring of benchmark IDs to measure")
    parser.add_argument("--smoke", action="store_true", help="short, non-publishable measurements")
    parser.add_argument("--notes", help="environmental limitations or other disclosures shown in reports")
    args = parser.parse_args()
    if args.runs < 1 or not 0 <= args.seed < 2**64:
        parser.error("--runs must be positive and --seed must fit in an unsigned 64-bit integer")
    if args.toolchain is None:
        args.toolchain = tomllib.loads((REPOSITORY / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
    if args.no_ohm_defaults and args.toolchain != "ohm":
        parser.error("--no-ohm-defaults requires --toolchain ohm")
    if args.cpu is not None:
        if not hasattr(os, "sched_setaffinity"):
            parser.error("--cpu requires an operating system with sched_setaffinity")
        if args.cpu not in os.sched_getaffinity(0):
            parser.error("--cpu must be in this process's allowed affinity set")
    return args


def main():
    args = arguments()
    output = args.output.resolve()
    create_run_directory(output)
    environment = os.environ.copy()
    environment.setdefault("CARGO_TARGET_DIR", str(WORKSPACE / "target"))
    environment["CARGO_TERM_COLOR"] = "never"
    cargo = ["cargo", f"+{args.toolchain}"]
    if args.no_ohm_defaults:
        cargo.append("-Zohm-defaults=no")
    settings = {
        "sample-size": 10 if args.smoke else 100,
        "warm-up-time": 0.05 if args.smoke else 1,
        "measurement-time": 0.1 if args.smoke else 3,
        "nresamples": 1000 if args.smoke else 10000,
        "confidence-level": 0.95,
    }
    sessions = [
        {"id": f"session-{index + 1:03d}", "seed": (args.seed + index) % 2**64, "status": "pending"}
        for index in range(args.runs)
    ]
    metadata = {
        "schema_version": 1, "started_at": timestamp(), "status": "running",
        "smoke": args.smoke, "filter": args.filter, "settings": settings,
        "notes": args.notes,
        "requested_cpu": args.cpu, "python": platform.python_version(),
        "toolchain": args.toolchain, "ohm_defaults_disabled": args.no_ohm_defaults,
        "rustc": capture(["rustc", f"+{args.toolchain}", "-Vv"]),
        "cargo": capture(["cargo", f"+{args.toolchain}", "-V"]),
        "machine": machine_info(), "git_before": git_state(),
        "lockfile_sha256": digest(WORKSPACE / "Cargo.lock"),
        "fixtures": fixture_hashes(), "source_files": source_hashes(),
        "sessions": sessions, "commands": [],
        "environment": {key: environment[key] for key in ENVIRONMENT_KEYS if key in environment},
    }
    metadata_path = output / "metadata.json"
    save_metadata(metadata_path, metadata)
    shutil.copy2(WORKSPACE / "Cargo.lock", output / "Cargo.lock")
    try:
        run_command(
            cargo + ["run", "--release", "--locked", "--bin", "validate", "--", "--output", str(output / "eligibility.json")],
            output / "validate.log", environment, metadata,
        )
        metadata["selected_benchmarks"] = select_benchmarks(
            json.loads((output / "eligibility.json").read_text()), args.filter,
        )
        run_command(
            cargo + ["bench", "--locked", "--bench", "workloads", "--no-run"],
            output / "build.log", environment, metadata,
        )
        if args.cpu is not None:
            os.sched_setaffinity(0, {args.cpu})
        metadata["measurement_machine"] = machine_info()
        for session in sessions:
            session["status"] = "running"
            session["started_at"] = timestamp()
            criterion_home = output / "criterion" / session["id"]
            criterion_home.mkdir(parents=True)
            session_environment = environment | {
                "CRITERION_HOME": str(criterion_home), "BENCH_ORDER_SEED": str(session["seed"]),
            }
            command = cargo + ["bench", "--locked", "--bench", "workloads", "--"]
            if args.filter:
                command.append(re.escape(args.filter))
            for key, value in settings.items():
                command += [f"--{key}", str(value)]
            save_metadata(metadata_path, metadata)
            run_command(command, output / f"{session['id']}.log", session_environment, metadata)
            session["status"] = "complete"
            session["finished_at"] = timestamp()
        problems = [
            row for row in REPORT.rows_for_run(output, metadata)
            if row[4] == "missing measurement" or row[4].startswith("invalid statistics:")
        ]
        if problems:
            raise ValueError(f"{len(problems)} selected eligible measurements are missing or invalid")
        metadata["status"] = "complete"
    except BaseException as error:
        metadata["status"] = "failed"
        metadata["error"] = str(error)
        for session in sessions:
            if session["status"] == "running":
                session["status"] = "failed"
        raise
    finally:
        metadata["finished_at"] = timestamp()
        metadata["git_after"] = git_state()
        metadata["source_changed"] = (
            metadata["git_before"] != metadata["git_after"]
            or metadata["fixtures"] != fixture_hashes()
            or metadata["source_files"] != source_hashes()
            or metadata["lockfile_sha256"] != digest(WORKSPACE / "Cargo.lock")
        )
        save_metadata(metadata_path, metadata)
        if (output / "eligibility.json").exists():
            REPORT.write_report(output)
    print(f"Saved complete reports and samples to {output}", flush=True)


if __name__ == "__main__":
    main()
