"""Report per-case and corpus timings from repeated parser comparisons."""

import collections
import csv
import pathlib
import statistics
import sys


def main(directory: pathlib.Path) -> None:
    values = collections.defaultdict(list)
    bytes_by_case = {}
    failures = collections.defaultdict(set)
    for path in sorted(directory.glob("*.tsv")):
        with path.open(newline="") as file:
            for row in csv.DictReader(file, delimiter="\t"):
                key = (row["case"], row["parser"])
                bytes_by_case[row["case"]] = int(row["bytes"])
                if row["status"] == "ok":
                    values[key].append(int(row["median_ns"]))
                else:
                    failures[key].add(row["status"])

    names = ("astral-baseline", "astral-recovering", "servo-html5ever-rcdom")
    total_ns = collections.Counter()
    total_bytes = 0
    shared_cases = 0
    shared_names = []
    ratios = []
    print("case\tbytes\tbaseline_ns\trecovering_ns\tservo_ns\trecovering/base\tservo/base")
    for case in sorted(bytes_by_case):
        measurements = [values.get((case, name), []) for name in names]
        medians = [round(statistics.median(samples)) if samples else None for samples in measurements]
        ratio = lambda index: (f"{medians[index] / medians[0]:.3f}" if medians[0] and medians[index] else "-")
        print(f"{case}\t{bytes_by_case[case]}\t" + "\t".join(str(median) if median is not None else "-" for median in medians) + f"\t{ratio(1)}\t{ratio(2)}")
        if all(median is not None for median in medians):
            shared_cases += 1
            shared_names.append(case)
            total_bytes += bytes_by_case[case]
            for name, median in zip(names, medians):
                total_ns[name] += median
            ratios.append((medians[1] / medians[0], case))

    print(f"\nShared cases: {shared_cases}/{len(bytes_by_case)}; total decoded UTF-8 bytes: {total_bytes}")
    for name in names:
        count = sum(bool(values.get((case, name))) for case in bytes_by_case)
        failures_by_name = [(case, failures[case, name]) for case in bytes_by_case if failures[case, name]]
        throughput = total_bytes / total_ns[name] * 1e9 / 1024 / 1024 if total_ns[name] else 0
        print(f"{name}: successes={count}, failures={len(failures_by_name)}, shared-corpus MiB/s={throughput:.2f}, shared-corpus ns={total_ns[name]}")
        repetitions = len(values[shared_names[0], name]) if shared_names else 0
        if repetitions > 0 and all(len(values[case, name]) == repetitions for case in shared_names):
            repetition_ns = [sum(values[case, name][index] for case in shared_names) for index in range(repetitions)]
            repetition_speed = [total_bytes / nanoseconds * 1e9 / 1024 / 1024 for nanoseconds in repetition_ns]
            print(f"  {repetitions} process repetitions: {', '.join(f'{speed:.2f}' for speed in repetition_speed)} MiB/s; range {min(repetition_speed):.2f}–{max(repetition_speed):.2f}")
        for case, errors in failures_by_name:
            print(f"  FAILED {case}: {', '.join(sorted(errors))}")
    if ratios:
        print("Largest recovery slowdowns by per-page median: " + ", ".join(f"{case} {ratio:.2f}x" for ratio, case in sorted(ratios, reverse=True)[:6]))
        print("Largest recovery speedups by per-page median: " + ", ".join(f"{case} {ratio:.2f}x" for ratio, case in sorted(ratios)[:6]))

    expanded_total_ns = collections.Counter()
    expanded_total_bytes = 0
    expanded_cases = 0
    for case in bytes_by_case:
        old = values.get((case, names[0])) or values.get((case, "astral-baseline-expanded-depth"))
        recovered = values.get((case, names[1]))
        servo = values.get((case, names[2]))
        if old and recovered and servo:
            expanded_cases += 1
            expanded_total_bytes += bytes_by_case[case]
            for name, samples in zip(names, (old, recovered, servo)):
                expanded_total_ns[name] += statistics.median(samples)
    print(f"\nCommon successes after explicitly raising only the old parser's depth on default-limit failures: {expanded_cases}/{len(bytes_by_case)} cases, {expanded_total_bytes} bytes")
    for name in names:
        nanoseconds = expanded_total_ns[name]
        speed = expanded_total_bytes / nanoseconds * 1e9 / 1024 / 1024 if nanoseconds else 0
        print(f"  {name}: {speed:.2f} MiB/s, {nanoseconds:.0f} ns")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit("usage: summarize.py RESULTS_DIRECTORY")
    main(pathlib.Path(sys.argv[1]))
