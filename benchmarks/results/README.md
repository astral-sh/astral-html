# Benchmark results

[epyc-vm-952e116.json](epyc-vm-952e116.json) preserves the completed run at
[revision 952e116](https://github.com/viarius-experiments/astral-html/tree/952e116f351d183560c8d758c28b8bafdf6549ce/benchmarks).
It contains the original run metadata and eligibility matrix, plus all 135
Criterion measurements: benchmark identifiers, statistical estimates, and raw
iteration counts and times. Times in the source data are nanoseconds. The run
started from a clean checkout and recorded no source changes.

The README chart selects the PEP index because it is the largest of the five
checked-in fixtures: 418,744 bytes and 2,099 extracted links. All six adapters
produced the same link records. The chart shows Reader for astral-html alongside
the four other crates; Document remains in the detailed results. Each bar is the
median of three independent session means. Individual session estimates and
confidence intervals are preserved in the source data. The complete corpus is
summarized below to make the selection explicit.

## Link extraction

Median of three session means, in **microseconds**; lower is better. Each session
collected 100 samples per measurement, with a one-second warmup and a three-second
measurement target. Sessions used different recorded order seeds. These values
summarize sessions without pooling their samples.

| Fixture | astral-html Reader | astral-html Document | astral-tl | html5gum | lol_html | scraper |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| iniconfig | 6.43 | 8.40 | 12.13 | 14.70 | 16.42 | 68.36 |
| Bootstrap dashboard | 38.30 | 69.28 | 44.53 | 92.93 | 65.10 | 402.29 |
| Rust Book | 180.37 | 346.68 | 200.31 | 389.45 | 283.44 | 1,646.63 |
| PEP 8 | 355.74 | 719.53 | 398.59 | 856.21 | 522.42 | 3,282.66 |
| PEP index | 1,278.47 | 2,493.36 | 1,545.82 | 2,834.91 | 2,764.06 | 10,489.41 |

The workload parses resident UTF-8 input, returns owned `href`, `title`, and `rel`
records, and drops parser state and output. Input loading and correctness checks
are outside timing. See the [methodology](../README.md) for adapter semantics and
the separate native-document workload, whose representations differ.

## Environment and provenance

These measurements came from a shared KVM VM on an AMD EPYC-Milan processor,
running Linux 6.8.0 with glibc 2.39. Measurement affinity was pinned to guest
vCPU 0. Physical-core exclusivity, host scheduling, CPU frequency/power policy,
and neighboring workloads were not controlled. Inputs were repeatedly measured
from memory. Five fixtures on this machine do not establish performance across
all websites or hardware; compare the independent sessions and their raw samples
when assessing variability.

The compiler was `rustc 1.98.1-dev (f62703110 2026-09-08)` from
`ohm-1.98.1-1`, with Ohm's experimental Cargo defaults disabled. The release
profile used thin LTO and one codegen unit, with the system allocator. Exact
compiler information, commands, settings, environment flags, hashes, and shared-VM
disclosures are retained in the JSON metadata.

The pinned [fixture manifest](https://github.com/viarius-experiments/astral-html/blob/952e116f351d183560c8d758c28b8bafdf6549ce/benchmarks/fixtures/manifest.json)
records source URLs and revisions, byte counts, SHA-256 hashes, and licenses.
The pinned [Cargo.lock](https://github.com/viarius-experiments/astral-html/blob/952e116f351d183560c8d758c28b8bafdf6549ce/benchmarks/Cargo.lock)
records the dependency graph. The PEP index is from
[python/peps revision 73849c8](https://github.com/python/peps/blob/73849c82abc9b9061664b2e2a064019f80c496db/pep-0000/index.html).
